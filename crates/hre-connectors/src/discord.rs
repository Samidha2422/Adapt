use crate::{map_engine_response, ConnectorError, HreClient, PlatformModerationAction};
use chrono::Utc;
use hre_core::{AnalyzeRequest, Channel, ChannelType, EventContext, EventKind, Message};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordMessage {
    pub id: String,
    pub channel_id: String,
    pub guild_id: Option<String>,
    pub author_id: String,
    pub author_username: String,
    pub content: String,
    pub is_bot: bool,
}

pub struct DiscordConnector {
    client: HreClient,
}

impl DiscordConnector {
    pub fn new(client: HreClient) -> Self {
        Self { client }
    }

    /// Evaluates an incoming Discord message against the Harassment Risk Engine.
    pub async fn process_message(
        &self,
        msg: &DiscordMessage,
    ) -> Result<Vec<PlatformModerationAction>, ConnectorError> {
        if msg.is_bot {
            return Ok(vec![PlatformModerationAction::Allow]);
        }

        let channel_kind = if msg.guild_id.is_some() {
            ChannelType::Group
        } else {
            ChannelType::Dm
        };

        let request = AnalyzeRequest {
            event_id: Uuid::new_v4(),
            kind: EventKind::Message,
            occurred_at: Utc::now(),
            draft: false,
            sender_id: format!("discord:user:{}", msg.author_id),
            recipient_id: format!("discord:channel:{}", msg.channel_id),
            platform: "discord".into(),
            channel: Channel {
                kind: channel_kind,
                conversation_id: msg.channel_id.clone(),
                group_id: msg.guild_id.clone(),
                post_id: None,
            },
            context: EventContext {
                is_contact: true,
                ..Default::default()
            },
            message: Some(Message {
                text: msg.content.clone(),
            }),
            media: None,
            call: None,
        };

        let response = self.client.analyze_event(&request).await?;
        let actions = map_engine_response(&response);

        tracing::info!(
            author_id = %msg.author_id,
            channel_id = %msg.channel_id,
            severity = %response.severity,
            tier = ?response.tier,
            actions_count = actions.len(),
            "Discord message analyzed by HRE engine"
        );

        Ok(actions)
    }
}
