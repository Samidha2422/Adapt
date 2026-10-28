use hre_connectors::{
    discord::DiscordMessage, map_engine_response, Action, AnalyzeResponse, DiscordConnector,
    HreClient, PlatformModerationAction, SecondaryAction, Tier,
};
use std::collections::BTreeMap;
use uuid::Uuid;

fn sample_response(tier: Tier, action: Action, secondary: Vec<SecondaryAction>) -> AnalyzeResponse {
    AnalyzeResponse {
        event_id: Uuid::new_v4(),
        severity: match tier {
            Tier::Low => 10,
            Tier::Review => 45,
            Tier::High => 75,
            Tier::Urgent => 95,
        },
        tier,
        reasons: vec![hre_core::Reason {
            code: "TEST_REASON".into(),
            text: "Detected abusive behavior".into(),
            weight: 0.8,
            evidence: BTreeMap::new(),
        }],
        signals: Default::default(),
        recommended_action: action,
        secondary_actions: secondary,
        alert_id: None,
        latency_ms: 12,
        model_versions: BTreeMap::new(),
        simulated: false,
        degraded: Vec::new(),
        nudge: None,
    }
}

#[test]
fn test_action_mapping_low_tier_allows() {
    let resp = sample_response(Tier::Low, Action::Log, vec![]);
    let actions = map_engine_response(&resp);
    assert_eq!(actions, vec![PlatformModerationAction::Allow]);
}

#[test]
fn test_action_mapping_warn_sender() {
    let resp = sample_response(Tier::Review, Action::WarnSender, vec![]);
    let actions = map_engine_response(&resp);
    assert_eq!(actions.len(), 1);
    assert!(matches!(
        actions[0],
        PlatformModerationAction::WarnAuthor { .. }
    ));
}

#[test]
fn test_action_mapping_throttle_deletes_and_timeouts() {
    let resp = sample_response(Tier::High, Action::Throttle, vec![]);
    let actions = map_engine_response(&resp);
    assert_eq!(actions.len(), 2);
    assert!(matches!(
        actions[0],
        PlatformModerationAction::DeleteMessage { .. }
    ));
    assert!(matches!(
        actions[1],
        PlatformModerationAction::TimeoutUser {
            duration_seconds: 3600,
            ..
        }
    ));
}

#[test]
fn test_action_mapping_blur_media() {
    let resp = sample_response(Tier::High, Action::BlurMedia, vec![]);
    let actions = map_engine_response(&resp);
    assert_eq!(actions.len(), 2);
    assert!(matches!(
        actions[0],
        PlatformModerationAction::DeleteMessage { .. }
    ));
    assert!(matches!(
        actions[1],
        PlatformModerationAction::BlurMediaNotice { .. }
    ));
}

#[test]
fn test_action_mapping_escalate_alerts_staff() {
    let resp = sample_response(
        Tier::Urgent,
        Action::Escalate,
        vec![SecondaryAction::AlertModerator],
    );
    let actions = map_engine_response(&resp);
    assert!(actions
        .iter()
        .any(|a| matches!(a, PlatformModerationAction::DeleteMessage { .. })));
    assert!(actions.iter().any(|a| matches!(
        a,
        PlatformModerationAction::TimeoutUser {
            duration_seconds: 86400,
            ..
        }
    )));
    assert!(actions
        .iter()
        .any(|a| matches!(a, PlatformModerationAction::AlertStaffChannel { .. })));
}

#[tokio::test]
async fn test_discord_ignores_bot_messages() {
    let client = HreClient::new("http://127.0.0.1:9999", "key");
    let connector = DiscordConnector::new(client);

    let msg = DiscordMessage {
        id: "msg_1".into(),
        channel_id: "chan_1".into(),
        guild_id: Some("guild_1".into()),
        author_id: "bot_1".into(),
        author_username: "bot_user".into(),
        content: "Beep boop".into(),
        is_bot: true,
    };

    let actions = connector.process_message(&msg).await.unwrap();
    assert_eq!(actions, vec![PlatformModerationAction::Allow]);
}
