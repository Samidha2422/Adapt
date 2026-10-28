use chrono::{DateTime, Utc};
use hre_core::{
    AnalyzeRequest, Call, CallDirection, Channel, ChannelType, EventContext, EventKind, Media,
    Message,
};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct GeneratedCase {
    pub id: String,
    pub scenario_class: String,
    pub is_harassment: bool,
    pub events: Vec<AnalyzeRequest>,
    pub notes: String,
}

pub struct GeneratorB {
    base_time: DateTime<Utc>,
}

impl Default for GeneratorB {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(clippy::too_many_arguments)]
fn make_msg_req(
    occurred_at: DateTime<Utc>,
    sender_id: &str,
    recipient_id: &str,
    channel_kind: ChannelType,
    conv_id: &str,
    group_id: Option<String>,
    post_id: Option<String>,
    context: EventContext,
    text: String,
) -> AnalyzeRequest {
    AnalyzeRequest {
        event_id: Uuid::new_v4(),
        kind: EventKind::Message,
        occurred_at,
        draft: false,
        sender_id: sender_id.into(),
        recipient_id: recipient_id.into(),
        platform: "demo-chat".into(),
        channel: Channel {
            kind: channel_kind,
            conversation_id: conv_id.into(),
            group_id,
            post_id,
        },
        context,
        message: Some(Message { text }),
        media: None,
        call: None,
    }
}

#[allow(clippy::too_many_arguments)]
fn make_media_req(
    kind: EventKind,
    occurred_at: DateTime<Utc>,
    sender_id: &str,
    recipient_id: &str,
    channel_kind: ChannelType,
    conv_id: &str,
    context: EventContext,
    mime: &str,
    caption: Option<String>,
) -> AnalyzeRequest {
    AnalyzeRequest {
        event_id: Uuid::new_v4(),
        kind,
        occurred_at,
        draft: false,
        sender_id: sender_id.into(),
        recipient_id: recipient_id.into(),
        platform: "demo-chat".into(),
        channel: Channel {
            kind: channel_kind,
            conversation_id: conv_id.into(),
            group_id: None,
            post_id: None,
        },
        context,
        message: None,
        media: Some(Media {
            mime: mime.into(),
            caption,
        }),
        call: None,
    }
}

fn make_call_req(
    occurred_at: DateTime<Utc>,
    sender_id: &str,
    recipient_id: &str,
    conv_id: &str,
    context: EventContext,
    call: Call,
) -> AnalyzeRequest {
    AnalyzeRequest {
        event_id: Uuid::new_v4(),
        kind: EventKind::CallEvent,
        occurred_at,
        draft: false,
        sender_id: sender_id.into(),
        recipient_id: recipient_id.into(),
        platform: "demo-chat".into(),
        channel: Channel {
            kind: ChannelType::Call,
            conversation_id: conv_id.into(),
            group_id: None,
            post_id: None,
        },
        context,
        message: None,
        media: None,
        call: Some(call),
    }
}

impl GeneratorB {
    pub fn new() -> Self {
        Self {
            base_time: DateTime::from_timestamp(1_793_188_800, 0).unwrap_or_else(Utc::now),
        }
    }

    /// Generates a comprehensive evaluation suite with independent parameter distributions.
    pub fn generate_all_cases(&self) -> Vec<GeneratedCase> {
        vec![
            self.generate_heavy_benign_couple(),
            self.generate_heavy_benign_friends(),
            self.generate_normal_dialogue(),
            self.generate_polite_stalker(),
            self.generate_escalating_stalker(),
            self.generate_post_block_contacter(),
            self.generate_pile_on(),
            self.generate_bully_and_doxxing(),
            self.generate_cyberflashing(),
            self.generate_screenshot_meme(),
            self.generate_voice_threat(),
            self.generate_silent_caller(),
            self.generate_night_caller(),
            self.generate_post_block_caller(),
            self.generate_noisy_ambiguous(),
            self.generate_multilingual_indian(),
        ]
    }

    fn generate_heavy_benign_couple(&self) -> GeneratedCase {
        let mut events = Vec::new();
        let target = "target_couple_eval";
        let partner = "partner_couple_eval";

        for i in 0..50 {
            events.push(make_msg_req(
                DateTime::from_timestamp(1_793_188_800 + i * 20, 0).unwrap_or(self.base_time),
                partner,
                target,
                ChannelType::Dm,
                "eval-couple",
                None,
                None,
                EventContext {
                    is_contact: true,
                    ..Default::default()
                },
                format!("Hey sweetie! Dinner at 8 tonight? Case {}", i),
            ));

            events.push(make_msg_req(
                DateTime::from_timestamp(1_793_188_800 + i * 20 + 5, 0).unwrap_or(self.base_time),
                target,
                partner,
                ChannelType::Dm,
                "eval-couple",
                None,
                None,
                EventContext {
                    is_contact: true,
                    ..Default::default()
                },
                format!("Yes absolutely! See you soon love case {}", i),
            ));
        }

        for i in 0..5 {
            events.push(make_call_req(
                DateTime::from_timestamp(1_793_188_800 + 1500 + i * 300, 0)
                    .unwrap_or(self.base_time),
                partner,
                target,
                "eval-couple-call",
                EventContext {
                    is_contact: true,
                    ..Default::default()
                },
                Call {
                    direction: CallDirection::Incoming,
                    duration_s: 720,
                    answered: true,
                    voicemail: false,
                },
            ));
        }

        GeneratedCase {
            id: "case_heavy_benign_couple".into(),
            scenario_class: "heavy_benign".into(),
            is_harassment: false,
            events,
            notes: "Reciprocal messaging and answered calls; FPR test".into(),
        }
    }

    fn generate_heavy_benign_friends(&self) -> GeneratedCase {
        let mut events = Vec::new();
        let target = "target_friends";
        for i in 0..30 {
            events.push(make_msg_req(
                DateTime::from_timestamp(1_793_188_800 + i * 30, 0).unwrap_or(self.base_time),
                &format!("friend_{}", i % 3),
                target,
                ChannelType::Group,
                "eval-friends-group",
                Some("group-study".into()),
                None,
                EventContext {
                    is_contact: true,
                    ..Default::default()
                },
                format!(
                    "Check out this study link for tomorrow: https://example.org/notes/{}",
                    i
                ),
            ));
        }
        GeneratedCase {
            id: "case_heavy_benign_friends".into(),
            scenario_class: "heavy_benign".into(),
            is_harassment: false,
            events,
            notes: "Active group study conversation".into(),
        }
    }

    fn generate_normal_dialogue(&self) -> GeneratedCase {
        let events = vec![make_msg_req(
            self.base_time,
            "colleague_1",
            "target_normal",
            ChannelType::Dm,
            "eval-colleague",
            None,
            None,
            EventContext::default(),
            "Could you please send over the quarterly report?".into(),
        )];
        GeneratedCase {
            id: "case_normal_dialogue".into(),
            scenario_class: "normal".into(),
            is_harassment: false,
            events,
            notes: "Benign workplace question".into(),
        }
    }

    fn generate_polite_stalker(&self) -> GeneratedCase {
        let mut events = Vec::new();
        let stalker = "stalker_polite_eval";
        let target = "target_polite_eval";

        for i in 0..80 {
            events.push(make_msg_req(
                DateTime::from_timestamp(1_793_188_800 + i * 15, 0).unwrap_or(self.base_time),
                stalker,
                target,
                ChannelType::Dm,
                "eval-polite",
                None,
                None,
                EventContext {
                    is_contact: false,
                    recipient_blocked_sender: i >= 70,
                    ..Default::default()
                },
                format!("Hello, hope you had a lovely day today! Message {}", i),
            ));
        }

        GeneratedCase {
            id: "case_polite_stalker".into(),
            scenario_class: "polite_stalker".into(),
            is_harassment: true,
            events,
            notes: "80 polite messages, no replies, continued after block".into(),
        }
    }

    fn generate_escalating_stalker(&self) -> GeneratedCase {
        let mut events = Vec::new();
        let stalker = "stalker_escalating_eval";
        let target = "target_escalating_eval";

        let messages = [
            "Hi, nice to see you",
            "Why aren't you replying to my texts?",
            "I saw where you had coffee this afternoon",
            "You cannot ignore me forever",
            "I will wait outside your apartment until you talk to me",
        ];

        for (i, &msg) in messages.iter().enumerate() {
            events.push(make_msg_req(
                DateTime::from_timestamp(1_793_188_800 + (i as i64) * 3600, 0)
                    .unwrap_or(self.base_time),
                stalker,
                target,
                ChannelType::Dm,
                "eval-escalating",
                None,
                None,
                EventContext {
                    is_contact: false,
                    ..Default::default()
                },
                msg.into(),
            ));
        }

        GeneratedCase {
            id: "case_escalating_stalker".into(),
            scenario_class: "escalating_stalker".into(),
            is_harassment: true,
            events,
            notes: "Escalating stalker tracking physical movements".into(),
        }
    }

    fn generate_post_block_contacter(&self) -> GeneratedCase {
        let mut events = Vec::new();
        for i in 0..10 {
            events.push(make_msg_req(
                DateTime::from_timestamp(1_793_188_800 + i * 60, 0).unwrap_or(self.base_time),
                "blocked_contacter",
                "target_blocked",
                ChannelType::Dm,
                "eval-blocked-contacter",
                None,
                None,
                EventContext {
                    is_contact: false,
                    recipient_blocked_sender: true,
                    ..Default::default()
                },
                format!("Unblock me right now, we need to talk {}", i),
            ));
        }

        GeneratedCase {
            id: "case_post_block_contacter".into(),
            scenario_class: "post_block_contacter".into(),
            is_harassment: true,
            events,
            notes: "Persistent messages with recipient_blocked_sender=true".into(),
        }
    }

    fn generate_pile_on(&self) -> GeneratedCase {
        let mut events = Vec::new();
        let target = "target_pile_on";

        for i in 0..10 {
            events.push(make_msg_req(
                DateTime::from_timestamp(1_793_188_800 + i * 2, 0).unwrap_or(self.base_time),
                &format!("pile_account_{}", i),
                target,
                ChannelType::Comment,
                "eval-pile",
                None,
                Some("post-target-status".into()),
                EventContext {
                    is_contact: false,
                    ..Default::default()
                },
                format!("Ratio and delete your account {}", i),
            ));
        }

        GeneratedCase {
            id: "case_pile_on".into(),
            scenario_class: "pile_on".into(),
            is_harassment: true,
            events,
            notes: "Ten distinct accounts swarming within 20s".into(),
        }
    }

    fn generate_bully_and_doxxing(&self) -> GeneratedCase {
        let events = vec![make_msg_req(
            self.base_time,
            "bully_sender",
            "target_bully",
            ChannelType::Dm,
            "eval-bully",
            None,
            None,
            EventContext::default(),
            "I know your phone +91 9876543210 and Aadhaar 9876 5432 1098. I will ruin you!".into(),
        )];

        GeneratedCase {
            id: "case_bully_doxxing".into(),
            scenario_class: "bully".into(),
            is_harassment: true,
            events,
            notes: "Direct threat with Indian phone and Aadhaar doxxing".into(),
        }
    }

    fn generate_cyberflashing(&self) -> GeneratedCase {
        let events = vec![make_media_req(
            EventKind::Image,
            self.base_time,
            "stranger_flasher",
            "target_flasher",
            ChannelType::Dm,
            "eval-flasher",
            EventContext {
                is_contact: false,
                ..Default::default()
            },
            "image/png",
            Some("cyberflashing".into()),
        )];

        GeneratedCase {
            id: "case_cyberflashing".into(),
            scenario_class: "cyberflasher".into(),
            is_harassment: true,
            events,
            notes: "Unsolicited explicit image proxy".into(),
        }
    }

    fn generate_screenshot_meme(&self) -> GeneratedCase {
        let events = vec![make_media_req(
            EventKind::Image,
            self.base_time,
            "meme_harasser",
            "target_meme",
            ChannelType::Dm,
            "eval-meme",
            EventContext {
                is_contact: false,
                ..Default::default()
            },
            "image/png",
            Some("I will kill you and your family".into()),
        )];

        GeneratedCase {
            id: "case_screenshot_meme".into(),
            scenario_class: "screenshot_meme".into(),
            is_harassment: true,
            events,
            notes: "Image meme containing abusive text for OCR extraction".into(),
        }
    }

    fn generate_voice_threat(&self) -> GeneratedCase {
        let events = vec![make_media_req(
            EventKind::Audio,
            self.base_time,
            "voice_harasser",
            "target_voice",
            ChannelType::Voice,
            "eval-voice",
            EventContext {
                is_contact: false,
                ..Default::default()
            },
            "audio/mpeg",
            Some("threat:I will find you and kill you".into()),
        )];

        GeneratedCase {
            id: "case_voice_threat".into(),
            scenario_class: "voice_threat".into(),
            is_harassment: true,
            events,
            notes: "Synthetic voice note threat with transcript and acoustic cue".into(),
        }
    }

    fn generate_silent_caller(&self) -> GeneratedCase {
        let mut events = Vec::new();
        let caller = "caller_silent_eval";
        let target = "target_silent_eval";

        for i in 0..15 {
            events.push(make_call_req(
                DateTime::from_timestamp(1_793_188_800 + i * 40, 0).unwrap_or(self.base_time),
                caller,
                target,
                "eval-silent-calls",
                EventContext {
                    is_contact: false,
                    ..Default::default()
                },
                Call {
                    direction: CallDirection::Incoming,
                    duration_s: 0,
                    answered: false,
                    voicemail: false,
                },
            ));
        }

        GeneratedCase {
            id: "case_silent_caller".into(),
            scenario_class: "silent_caller".into(),
            is_harassment: true,
            events,
            notes: "Rapid sequence of unanswered 0-second calls".into(),
        }
    }

    fn generate_night_caller(&self) -> GeneratedCase {
        let mut events = Vec::new();
        let caller = "caller_night_eval";
        let target = "target_night_eval";
        let night_base = 1_793_214_000_i64; // 01:00 AM IST

        for i in 0..25 {
            events.push(make_call_req(
                DateTime::from_timestamp(night_base + i * 120, 0).unwrap_or(self.base_time),
                caller,
                target,
                "eval-night-calls",
                EventContext {
                    is_contact: false,
                    ..Default::default()
                },
                Call {
                    direction: CallDirection::Incoming,
                    duration_s: 0,
                    answered: false,
                    voicemail: false,
                },
            ));
        }

        GeneratedCase {
            id: "case_night_caller".into(),
            scenario_class: "night_caller".into(),
            is_harassment: true,
            events,
            notes: "Repeated calls overnight during rest hours (01:00 AM IST)".into(),
        }
    }

    fn generate_post_block_caller(&self) -> GeneratedCase {
        let mut events = Vec::new();
        let caller = "caller_post_block_eval";
        let target = "target_post_block_eval";

        for i in 0..10 {
            events.push(make_call_req(
                DateTime::from_timestamp(1_793_188_800 + i * 180, 0).unwrap_or(self.base_time),
                caller,
                target,
                "eval-post-block-calls",
                EventContext {
                    is_contact: false,
                    recipient_blocked_sender: true,
                    ..Default::default()
                },
                Call {
                    direction: CallDirection::Incoming,
                    duration_s: 0,
                    answered: false,
                    voicemail: false,
                },
            ));
        }

        GeneratedCase {
            id: "case_post_block_caller".into(),
            scenario_class: "post_block_caller".into(),
            is_harassment: true,
            events,
            notes: "Repeated calls after recipient blocked caller".into(),
        }
    }

    fn generate_noisy_ambiguous(&self) -> GeneratedCase {
        let events = vec![make_msg_req(
            self.base_time,
            "friend_banter",
            "target_banter",
            ChannelType::Dm,
            "eval-banter",
            None,
            None,
            EventContext {
                is_contact: true,
                ..Default::default()
            },
            "You are totally insane dude haha lol, that movie was epic".into(),
        )];

        GeneratedCase {
            id: "case_noisy_ambiguous".into(),
            scenario_class: "noisy_ambiguous".into(),
            is_harassment: false,
            events,
            notes: "Colloquial banter between friends with mild intensifiers".into(),
        }
    }

    fn generate_multilingual_indian(&self) -> GeneratedCase {
        let events = vec![
            make_msg_req(
                self.base_time,
                "indian_sender_hinglish",
                "target_indian",
                ChannelType::Dm,
                "eval-hinglish",
                None,
                None,
                EventContext::default(),
                "Teri location mil gayi hai, ghar se nikal ke dikha".into(),
            ),
            make_msg_req(
                DateTime::from_timestamp(1_793_188_800 + 30, 0).unwrap_or(self.base_time),
                "indian_sender_marathi",
                "target_indian",
                ChannelType::Dm,
                "eval-marathi",
                None,
                None,
                EventContext::default(),
                "तुला बघून घेईन, ami tuka sodchena [synthetic regional threat sample]".into(),
            ),
        ];

        GeneratedCase {
            id: "case_multilingual_indian".into(),
            scenario_class: "multilingual_indian".into(),
            is_harassment: true,
            events,
            notes: "Indian multilingual threat in Hinglish and Marathi/Konkani regional phrasing"
                .into(),
        }
    }
}
