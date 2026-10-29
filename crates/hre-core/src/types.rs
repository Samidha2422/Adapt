use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use utoipa::ToSchema;
use uuid::Uuid;

macro_rules! wire_enum {
    ($name:ident { $($variant:ident),+ $(,)? }) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
        #[serde(rename_all = "snake_case")]
        pub enum $name { $($variant),+ }
    };
}
wire_enum!(Status {
    Built,
    Simulated,
    Roadmap
});
wire_enum!(EventKind {
    Message,
    Image,
    Audio,
    CallEvent
});
wire_enum!(Tier {
    Low,
    Review,
    High,
    Urgent
});
wire_enum!(Action {
    Log,
    WarnSender,
    BlurMedia,
    Throttle,
    Escalate
});
wire_enum!(SecondaryAction {
    AlertModerator,
    CollapseReplies,
    RateLimit,
    NeedsHumanReview,
    ShowTranscript
});
wire_enum!(Backend {
    Onnx,
    Heuristic,
    Disabled,
    Ggml
});
wire_enum!(ChannelType {
    Dm,
    Group,
    Comment,
    Livestream,
    Voice,
    Call
});
wire_enum!(CallDirection { Incoming, Outgoing });
wire_enum!(RefusalKind {
    Block,
    Unblock,
    Mute,
    StopRequest
});
wire_enum!(ReviewDecision {
    Confirm,
    Reject,
    Escalate
});

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Channel {
    #[serde(rename = "type")]
    pub kind: ChannelType,
    pub conversation_id: String,
    #[serde(default)]
    pub group_id: Option<String>,
    #[serde(default)]
    pub post_id: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
#[serde(default, deny_unknown_fields)]
pub struct EventContext {
    /// Marks a synthetic input; this never changes its risk score.
    pub simulated: bool,
    pub is_contact: bool,
    pub recipient_blocked_sender: bool,
    pub recipient_muted_sender: bool,
    pub stop_requested: bool,
    pub sender_account_age_days: Option<u32>,
    pub recipient_is_minor: Option<bool>,
    pub reply_to_event_id: Option<Uuid>,
    pub mentions: Vec<String>,
    pub language_hint: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Message {
    pub text: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Media {
    pub mime: String,
    #[serde(default)]
    pub caption: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Call {
    pub direction: CallDirection,
    pub duration_s: u32,
    pub answered: bool,
    #[serde(default)]
    pub voicemail: bool,
}

/// Client envelope. Raw identities must be pseudonymized before storage.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AnalyzeRequest {
    pub event_id: Uuid,
    pub kind: EventKind,
    pub occurred_at: DateTime<Utc>,
    #[serde(default)]
    pub draft: bool,
    pub sender_id: String,
    pub recipient_id: String,
    pub platform: String,
    pub channel: Channel,
    #[serde(default)]
    pub context: EventContext,
    #[serde(default)]
    pub message: Option<Message>,
    #[serde(default)]
    pub media: Option<Media>,
    #[serde(default)]
    pub call: Option<Call>,
}
impl AnalyzeRequest {
    /// Enforces bounded identities and exactly one payload of the declared kind.
    pub fn validate(&self) -> Result<(), CoreError> {
        for value in [
            &self.sender_id,
            &self.recipient_id,
            &self.platform,
            &self.channel.conversation_id,
        ] {
            if value.trim().is_empty() || value.len() > 256 {
                return Err(CoreError::Validation(
                    "identity/channel must contain 1..256 bytes".into(),
                ));
            }
        }
        let valid = match self.kind {
            EventKind::Message => {
                self.message
                    .as_ref()
                    .is_some_and(|m| !m.text.trim().is_empty() && m.text.len() <= 32768)
                    && self.media.is_none()
                    && self.call.is_none()
            }
            EventKind::CallEvent => {
                self.call.is_some() && self.message.is_none() && self.media.is_none()
            }
            EventKind::Image | EventKind::Audio => {
                self.media
                    .as_ref()
                    .is_some_and(|m| !m.mime.is_empty() && m.mime.len() <= 128)
                    && self.message.is_none()
                    && self.call.is_none()
            }
        };
        if !valid {
            return Err(CoreError::Validation(
                "payload does not match event kind or exceeds bounds".into(),
            ));
        }
        if self.context.mentions.len() > 100
            || self.context.mentions.iter().any(|id| id.len() > 256)
        {
            return Err(CoreError::Validation("mentions exceed bounds".into()));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Reason {
    pub code: String,
    pub text: String,
    pub weight: f64,
    pub evidence: BTreeMap<String, Value>,
}
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Signal {
    #[schema(minimum = 0, maximum = 1)]
    pub score: f64,
    pub label: String,
    pub status: Status,
    pub backend: Backend,
    #[schema(minimum = 0, maximum = 1)]
    pub confidence: f64,
    pub details: BTreeMap<String, Value>,
}
impl Default for Signal {
    fn default() -> Self {
        Self {
            score: 0.0,
            label: "unavailable".into(),
            status: Status::Roadmap,
            backend: Backend::Disabled,
            confidence: 0.0,
            details: BTreeMap::new(),
        }
    }
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct Signals {
    pub text: Signal,
    pub ocr: Signal,
    pub image: Signal,
    pub audio: Signal,
    pub call: Signal,
    pub behavior: Signal,
    pub pile_on: Signal,
    pub pii: Signal,
}
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Nudge {
    pub text: String,
    pub rewrite_hint: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AnalyzeResponse {
    pub event_id: Uuid,
    #[schema(minimum = 0, maximum = 100)]
    pub severity: u8,
    pub tier: Tier,
    pub reasons: Vec<Reason>,
    pub signals: Signals,
    pub recommended_action: Action,
    pub secondary_actions: Vec<SecondaryAction>,
    pub alert_id: Option<Uuid>,
    pub latency_ms: u64,
    pub model_versions: BTreeMap<String, String>,
    pub simulated: bool,
    pub degraded: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub nudge: Option<Nudge>,
}
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Problem {
    #[serde(rename = "type")]
    pub kind: String,
    pub title: String,
    pub status: u16,
    pub detail: String,
    pub instance: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub mock: bool,
    pub degraded: Vec<String>,
}
/// Model integrity is separate from runnable inference availability.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ModelHealth {
    pub name: String,
    pub version: Option<String>,
    pub hash_verified: bool,
    pub backend: Backend,
    pub degraded: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RefusalRequest {
    pub event_id: Uuid,
    pub sender_id: String,
    pub recipient_id: String,
    pub occurred_at: DateTime<Utc>,
    pub kind: RefusalKind,
    pub platform: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReportRequest {
    pub event_id: Uuid,
    pub reason: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReviewRequest {
    pub decision: ReviewDecision,
    pub note: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FeedProtection {
    pub subject_id: String,
    pub enabled: bool,
    pub actions: Vec<SecondaryAction>,
}
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("{0}")]
    Validation(String),
    #[error("configuration: {0}")]
    Config(String),
    #[error("invalid HMAC key")]
    InvalidKey,
}
