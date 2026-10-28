use crate::{map_engine_response, ConnectorError, HreClient, PlatformModerationAction};
use chrono::Utc;
use hre_core::{AnalyzeRequest, Channel, ChannelType, EventContext, EventKind, Media, Message};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramUpdate {
    pub update_id: u64,
    pub message_id: u64,
    pub from_id: u64,
    pub chat_id: i64,
    pub text: Option<String>,
    pub caption: Option<String>,
    pub photo_file_id: Option<String>,
    pub voice_file_id: Option<String>,
}

pub struct TelegramConnector {
    client: HreClient,
}

impl TelegramConnector {
    pub fn new(client: HreClient) -> Self {
        Self { client }
    }

    /// Evaluates an incoming Telegram text message against the Harassment Risk Engine.
    pub async fn process_text(
        &self,
        update: &TelegramUpdate,
    ) -> Result<Vec<PlatformModerationAction>, ConnectorError> {
        let text = update
            .text
            .as_deref()
            .or(update.caption.as_deref())
            .unwrap_or_default();

        let request = AnalyzeRequest {
            event_id: Uuid::new_v4(),
            kind: EventKind::Message,
            occurred_at: Utc::now(),
            draft: false,
            sender_id: format!("tg:user:{}", update.from_id),
            recipient_id: format!("tg:chat:{}", update.chat_id),
            platform: "telegram".into(),
            channel: Channel {
                kind: if update.chat_id > 0 {
                    ChannelType::Dm
                } else {
                    ChannelType::Group
                },
                conversation_id: update.chat_id.to_string(),
                group_id: None,
                post_id: None,
            },
            context: EventContext {
                is_contact: false,
                ..Default::default()
            },
            message: Some(Message { text: text.into() }),
            media: None,
            call: None,
        };

        let response = self.client.analyze_event(&request).await?;
        let actions = map_engine_response(&response);

        tracing::info!(
            from_id = %update.from_id,
            chat_id = %update.chat_id,
            severity = %response.severity,
            tier = ?response.tier,
            "Telegram message analyzed"
        );

        Ok(actions)
    }

    /// Evaluates incoming Telegram image or voice attachment against the Harassment Risk Engine.
    pub async fn process_media(
        &self,
        update: &TelegramUpdate,
        media_bytes: &[u8],
        is_voice: bool,
    ) -> Result<Vec<PlatformModerationAction>, ConnectorError> {
        let (kind, mime, field_name, filename) = if is_voice {
            (EventKind::Audio, "audio/ogg", "audio", "telegram_voice.ogg")
        } else {
            (
                EventKind::Image,
                "image/jpeg",
                "image",
                "telegram_photo.jpg",
            )
        };

        let request = AnalyzeRequest {
            event_id: Uuid::new_v4(),
            kind,
            occurred_at: Utc::now(),
            draft: false,
            sender_id: format!("tg:user:{}", update.from_id),
            recipient_id: format!("tg:chat:{}", update.chat_id),
            platform: "telegram".into(),
            channel: Channel {
                kind: if update.chat_id > 0 {
                    ChannelType::Dm
                } else {
                    ChannelType::Group
                },
                conversation_id: update.chat_id.to_string(),
                group_id: None,
                post_id: None,
            },
            context: EventContext {
                is_contact: false,
                ..Default::default()
            },
            message: None,
            media: Some(Media {
                mime: mime.into(),
                caption: update.caption.clone(),
            }),
            call: None,
        };

        let response = self
            .client
            .analyze_media(&request, media_bytes, field_name, filename, mime)
            .await?;
        let actions = map_engine_response(&response);

        tracing::info!(
            from_id = %update.from_id,
            chat_id = %update.chat_id,
            is_voice,
            severity = %response.severity,
            tier = ?response.tier,
            "Telegram media analyzed"
        );

        Ok(actions)
    }
}
