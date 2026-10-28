use crate::{mock, ApiError};
use axum::http::StatusCode;
use hre_core::*;
use hre_store::{EventKind as StoredKind, Store, StoredEvent};
use std::{collections::BTreeMap, time::Instant};
use tokio::sync::Mutex;

pub(crate) struct Engine {
    store: Option<Store>,
    ids: Pseudonymizer,
    // Serialize read/insert/assessment across requests, including reverse-direction replies.
    gate: Mutex<()>,
    behavior: hre_behavior::BehaviorConfig,
    fusion: hre_fusion::FusionConfig,
    registry: hre_ml::ModelRegistry,
}
impl Engine {
    pub async fn new(config: &EngineConfig, mock: bool) -> anyhow::Result<Self> {
        let models_dir = config.models_dir.clone();
        let (mut behavior, fusion, registry) =
            tokio::task::spawn_blocking(move || -> anyhow::Result<_> {
                let behavior = match std::fs::read_to_string("config/behavior_rules.toml") {
                    Ok(text) => hre_behavior::BehaviorConfig::parse(&text)?,
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                        hre_behavior::BehaviorConfig::default()
                    }
                    Err(e) => return Err(e.into()),
                };
                let fusion = match std::fs::read_to_string("config/fusion.toml") {
                    Ok(text) => hre_fusion::FusionConfig::parse(&text)?,
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                        hre_fusion::FusionConfig::default()
                    }
                    Err(e) => return Err(e.into()),
                };
                Ok((
                    behavior,
                    fusion,
                    hre_ml::ModelRegistry::load(std::path::Path::new(&models_dir)),
                ))
            })
            .await??;
        behavior.night_start_hour = u32::from(config.night_start_hour);
        behavior.night_end_hour = u32::from(config.night_end_hour);
        if config.night_timezone == "Asia/Kolkata" {
            behavior.recipient_utc_offset_minutes = 330;
        } else if config.night_timezone != "UTC" {
            anyhow::bail!("night_timezone currently supports Asia/Kolkata or UTC");
        }
        Ok(Self {
            store: if mock {
                None
            } else {
                Some(Store::connect(&config.database_url).await?)
            },
            ids: Pseudonymizer::new(config.salt.as_bytes())?,
            gate: Mutex::new(()),
            behavior,
            fusion,
            registry,
        })
    }
    pub async fn analyze(
        &self,
        request: &AnalyzeRequest,
    ) -> Result<(AnalyzeResponse, bool), ApiError> {
        let started = Instant::now();
        let _gate = self.gate.lock().await;
        let store = self.store.as_ref().ok_or_else(|| {
            ApiError::new(StatusCode::SERVICE_UNAVAILABLE, "real engine unavailable")
        })?;
        let request_hash = self.ids.opaque(
            "request",
            &serde_json::to_string(request).map_err(|_| {
                ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, "cannot encode request")
            })?,
        )?;
        if !request.draft {
            if let Some(cached) = store.assessment(&request.event_id.to_string()).await? {
                return cached_response(&cached, &request_hash).map(|response| (response, false));
            }
        }
        if matches!(request.kind, EventKind::Image | EventKind::Audio) {
            return Err(ApiError::new(
                StatusCode::NOT_IMPLEMENTED,
                "media analysis requires the modality pipeline; use --mock for client fixtures",
            ));
        }
        let sender = self.ids.subject(&request.platform, &request.sender_id)?;
        let recipient = self.ids.subject(&request.platform, &request.recipient_id)?;
        let now = request.occurred_at.timestamp_millis();
        let mut events = store
            .target_window(&recipient.0, now.saturating_sub(7 * 86400000), now)
            .await?;
        events.extend(
            store
                .pair_window(
                    &recipient.0,
                    &sender.0,
                    now.saturating_sub(7 * 86400000),
                    now,
                )
                .await?,
        );
        let markers = store
            .latest_refusal_markers(&sender.0, &recipient.0, now)
            .await?;
        let after_refusal = refusal_active(&markers)
            || request.context.recipient_blocked_sender
            || request.context.recipient_muted_sender
            || request.context.stop_requested;
        events.extend(markers);
        events.extend(
            store
                .latest_refusal_markers(&recipient.0, &sender.0, now)
                .await?,
        );
        let mut event = StoredEvent {
            event_id: if request.draft {
                uuid::Uuid::new_v4().to_string()
            } else {
                request.event_id.to_string()
            },
            sender: sender.0.clone(),
            recipient: recipient.0.clone(),
            timestamp_ms: now,
            kind: if request.kind == EventKind::CallEvent {
                StoredKind::Call
            } else {
                StoredKind::Message
            },
            channel: self
                .ids
                .opaque("channel", &request.channel.conversation_id)?,
            content_hash: request
                .message
                .as_ref()
                .map(|m| self.ids.opaque("content", &m.text))
                .transpose()?,
            sender_account_age_days: request.context.sender_account_age_days.map(f64::from),
            is_contact: request.context.is_contact,
            after_refusal,
            answered: request.call.as_ref().is_some_and(|c| c.answered),
            duration_s: request
                .call
                .as_ref()
                .map_or(0.0, |c| f64::from(c.duration_s)),
            ..StoredEvent::default()
        };
        // shortcut: only a small explicit-threat fallback exists until hre-text is integrated.
        let text = request
            .message
            .as_ref()
            .map_or(String::new(), |m| m.text.to_lowercase());
        if ["i will kill you", "i will hurt you"]
            .iter()
            .any(|cue| text.contains(cue))
        {
            event.threat = 0.95;
            event.toxicity = 0.95;
        }
        events.retain(|e| e.event_id != event.event_id);
        events.push(event.clone());
        let behavior_config = self.behavior.clone();
        let fusion_config = self.fusion.clone();
        let registry = self.registry.clone();
        let pending = event.clone();
        let event_id = request.event_id;
        let mut response = tokio::task::spawn_blocking(move || {
            Self::assess(
                event_id,
                &events,
                &pending,
                &behavior_config,
                &fusion_config,
                &registry,
            )
        })
        .await
        .map_err(|_| {
            ApiError::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "analysis worker unavailable",
            )
        })?;
        response.latency_ms = started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
        response.nudge = mock::nudge(request.draft, response.severity);
        if !request.draft {
            if response.severity >= 31 {
                response.alert_id = Some(uuid::Uuid::new_v4());
            }
            let assessment = serde_json::json!({"request_hash":request_hash,"response":response});
            let alert = response
                .alert_id
                .map(|_| serde_json::to_value(&response))
                .transpose()
                .map_err(|_| {
                    ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "cannot encode alert")
                })?;
            let (fresh, saved) = store
                .persist_assessment(
                    &event,
                    &assessment,
                    alert.as_ref(),
                    response.signals.pile_on.score >= 0.6,
                )
                .await?;
            return cached_response(&saved, &request_hash).map(|response| (response, fresh));
        }
        Ok((response, false))
    }
    fn assess(
        event_id: uuid::Uuid,
        events: &[StoredEvent],
        event: &StoredEvent,
        behavior_config: &hre_behavior::BehaviorConfig,
        fusion_config: &hre_fusion::FusionConfig,
        registry: &hre_ml::ModelRegistry,
    ) -> AnalyzeResponse {
        let behavior = hre_behavior::analyze(
            events,
            &event.sender,
            &event.recipient,
            event.timestamp_ms,
            behavior_config,
        );
        let mut signals = Signals {
            behavior: behavior.behavior,
            pile_on: behavior.pile_on,
            ..Signals::default()
        };
        signals.text = Signal {
            score: event.toxicity,
            label: "minimal_explicit_threat_fallback".into(),
            status: Status::Built,
            backend: Backend::Heuristic,
            confidence: 0.3,
            details: BTreeMap::new(),
        };
        let mut degraded = behavior.degraded;
        degraded.extend(registry.degradation());
        degraded.extend([
            "text.onnx_missing".into(),
            "text.minimal_threat_fallback".into(),
            "call.behavior_model_unavailable".into(),
        ]);
        let input = hre_fusion::FusionInput {
            signals: signals.clone(),
            reasons: behavior.reasons,
            degraded,
            threat: event.threat,
            after_refusal_count: behavior.after_refusal_count,
            healthy_reciprocity: behavior.healthy_reciprocity,
            reply_to_harasser: behavior.reply_to_harasser,
            feed_protection: behavior.feed_protection,
            simulated: behavior.simulated,
            ..hre_fusion::FusionInput::default()
        };
        let fused = hre_fusion::fuse(&input, fusion_config);
        AnalyzeResponse {
            event_id,
            severity: fused.severity,
            tier: fused.tier,
            reasons: fused.reasons,
            signals,
            recommended_action: fused.recommended_action,
            secondary_actions: fused.secondary_actions,
            alert_id: None,
            latency_ms: 0,
            model_versions: BTreeMap::from([
                ("behavior_rules".into(), "0.1.0".into()),
                ("fusion_rules".into(), "0.1.0".into()),
            ]),
            simulated: fused.simulated,
            degraded: fused.degraded,
            nudge: None,
        }
    }
    pub fn model_health(&self) -> Vec<ModelHealth> {
        [
            "text",
            "behavior_msg",
            "behavior_call",
            "image",
            "ocr",
            "audio",
        ]
        .iter()
        .map(|name| {
            let report = self.registry.report(name);
            ModelHealth {
                name: report.name,
                version: report.version,
                hash_verified: report.hash_verified,
                backend: report.backend,
                degraded: report.degraded,
            }
        })
        .collect()
    }
    pub async fn refusal(&self, request: &RefusalRequest) -> Result<(), ApiError> {
        for id in [&request.sender_id, &request.recipient_id, &request.platform] {
            if id.trim().is_empty() || id.len() > 256 {
                return Err(ApiError::new(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "invalid refusal identity",
                ));
            }
        }
        let Some(store) = &self.store else {
            return Ok(());
        };
        let _gate = self.gate.lock().await;
        let event = StoredEvent {
            event_id: request.event_id.to_string(),
            sender: self.ids.subject(&request.platform, &request.sender_id)?.0,
            recipient: self
                .ids
                .subject(&request.platform, &request.recipient_id)?
                .0,
            timestamp_ms: request.occurred_at.timestamp_millis(),
            kind: StoredKind::Refusal,
            channel: serde_json::to_value(request.kind)
                .ok()
                .and_then(|v| v.as_str().map(str::to_owned))
                .unwrap_or_default(),
            after_refusal: request.kind != RefusalKind::Unblock,
            ..StoredEvent::default()
        };
        store.insert_event(&event).await?;
        Ok(())
    }
    pub async fn alerts(&self) -> Result<Vec<AnalyzeResponse>, ApiError> {
        let Some(store) = &self.store else {
            return Ok(Vec::new());
        };
        store
            .alerts()
            .await?
            .into_iter()
            .map(|value| {
                serde_json::from_value(value).map_err(|_| {
                    ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "invalid stored alert")
                })
            })
            .collect()
    }
    pub async fn feed_protection(&self, id: &str) -> Result<bool, ApiError> {
        if !hre_store::is_pseudonym(id) {
            return Err(ApiError::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "invalid subject pseudonym",
            ));
        }
        match &self.store {
            Some(store) => Ok(store.feed_protected(id).await?),
            None => Ok(false),
        }
    }
}
fn cached_response(
    cached: &serde_json::Value,
    request_hash: &str,
) -> Result<AnalyzeResponse, ApiError> {
    if cached
        .get("request_hash")
        .and_then(serde_json::Value::as_str)
        != Some(request_hash)
    {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "event_id already exists with different content",
        ));
    }
    serde_json::from_value(cached["response"].clone()).map_err(|_| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "invalid cached assessment",
        )
    })
}
fn refusal_active(markers: &[StoredEvent]) -> bool {
    markers
        .iter()
        .any(|e| e.channel == "mute" || e.channel == "stop_request")
        || markers
            .iter()
            .filter(|e| e.channel == "block" || e.channel == "unblock")
            .max_by_key(|e| (e.timestamp_ms, &e.event_id))
            .is_some_and(|e| e.channel == "block")
}
