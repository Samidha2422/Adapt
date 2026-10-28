use crate::{mock, ApiError};
use axum::http::StatusCode;
use hre_core::*;
use hre_store::{EventKind as StoredKind, Store, StoredEvent};
use hre_vision::ImageBackend;
use std::{collections::BTreeMap, time::Instant};
use tokio::sync::Mutex;

pub(crate) struct Engine {
    store: Option<Store>,
    pub(crate) ids: Pseudonymizer,
    // Serialize read/insert/assessment across requests, including reverse-direction replies.
    gate: Mutex<()>,
    behavior: hre_behavior::BehaviorConfig,
    fusion: hre_fusion::FusionConfig,
    registry: hre_ml::ModelRegistry,
    image: hre_vision::HeuristicImageBackend,
    ocr: hre_vision::HeuristicOcrBackend,
    pub(crate) audio: hre_audio::HeuristicAudioBackend,
    pub(crate) call_config: hre_calls::CallFeatureConfig,
    pub(crate) jobs: hre_audio::JobManager,
    pub(crate) slang_lexicon: hre_slang::SlangLexicon,
    pub(crate) slang_miner: hre_slang::CandidateMiner,
    pub(crate) slang_adapter: hre_slang::AdapterManager,
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
        let image = tokio::task::spawn_blocking(|| {
            hre_vision::HeuristicImageBackend::from_config_file(std::path::Path::new(
                "config/demo_overrides.toml",
            ))
        })
        .await?;
        let ocr = hre_vision::HeuristicOcrBackend::new();
        let audio = hre_audio::HeuristicAudioBackend::new();
        let call_config = hre_calls::CallFeatureConfig::default();
        let jobs = hre_audio::JobManager::new();
        let slang_lexicon = hre_slang::SlangLexicon::new();
        let slang_miner = hre_slang::CandidateMiner::new();
        let slang_adapter = hre_slang::AdapterManager::new();
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
            image,
            ocr,
            audio,
            call_config,
            jobs,
            slang_lexicon,
            slang_miner,
            slang_adapter,
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
            kind: match request.kind {
                EventKind::CallEvent => StoredKind::Call,
                EventKind::Image => StoredKind::Image,
                EventKind::Audio => StoredKind::Audio,
                EventKind::Message => StoredKind::Message,
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
        // Full text normalization, multi-label analysis, PII masking, cue detection, and slang matching
        let mut slang_matches: Vec<hre_slang::SlangMatch> = Vec::new();
        let text_report = if let Some(msg) = request.message.as_ref() {
            if let Ok(matches) = self.slang_lexicon.find_matches(&msg.text) {
                slang_matches = matches;
            }
            match hre_text::analyze(&msg.text) {
                Ok(mut rep) => {
                    if let Some(m) = slang_matches.first() {
                        rep.primary_signal.score = rep.primary_signal.score.max(m.severity);
                        rep.primary_signal.label = "slang_harassment".into();
                        rep.primary_signal.backend = Backend::Heuristic;
                        rep.primary_signal.status = Status::Built;
                        rep.reasons.push(Reason {
                            code: "SLANG_MATCH".into(),
                            text: format!(
                                "Confirmed slang term '{}' detected: {}",
                                m.term, m.meaning
                            ),
                            weight: m.severity,
                            evidence: BTreeMap::from([
                                ("term".into(), serde_json::json!(m.term)),
                                ("meaning".into(), serde_json::json!(m.meaning)),
                            ]),
                        });
                    }
                    event.toxicity = rep
                        .toxicity
                        .max(slang_matches.first().map_or(0.0, |m| m.severity));
                    event.threat = rep.threat;
                    event.sexual = rep.sexual_explicit;
                    event.pii_mentions = rep.pii_matches.len() as u32;
                    event.location_cues = rep.location_cues as u32;
                    Some(rep)
                }
                Err(hre_text::TextError::EmptyText) => None,
                Err(e) => {
                    return Err(ApiError::new(StatusCode::BAD_REQUEST, &e.to_string()));
                }
            }
        } else {
            None
        };

        // Vision screening, OCR extraction, and reported-hash matching
        let mut image_analysis: Option<hre_vision::ImageAnalysis> = None;
        let mut ocr_report: Option<hre_vision::OcrReport> = None;
        let mut reported_hash_match = false;
        let mut unsolicited_explicit = false;

        if request.kind == EventKind::Image {
            if let Some(media) = &request.media {
                let caption = media.caption.as_deref().unwrap_or("").trim();
                let is_proxy = caption.contains("cyberflasher")
                    || caption.contains("cyberflashing")
                    || caption.starts_with("proxy:")
                    || caption.starts_with("hash:");

                let img_bytes = if is_proxy {
                    let proxy_hash =
                        if caption.starts_with("hash:") || caption.starts_with("sha256:") {
                            caption.split(':').nth(1).unwrap_or("").trim()
                        } else {
                            "cf10000000000000000000000000000000000000000000000000000000000001"
                        };
                    hre_vision::png_with_text("ProxyHash", proxy_hash)
                } else if !caption.is_empty() {
                    hre_vision::png_with_text("Description", caption)
                } else {
                    hre_vision::minimal_png(10, 10)
                };

                // Image screening
                if let Ok(mut analysis) = self.image.analyze(&img_bytes, &media.mime) {
                    if is_proxy {
                        analysis.signal.status = Status::Simulated;
                        analysis.signal.score = 0.95;
                        analysis.simulated = true;
                        analysis.recommended_action = Some(Action::BlurMedia);
                    }
                    if let Ok(Some(_)) = store.find_reported_hash(analysis.p_hash, 5).await {
                        reported_hash_match = true;
                    }
                    if (analysis.signal.score >= 0.5 || is_proxy) && !request.context.is_contact {
                        unsolicited_explicit = true;
                    }
                    image_analysis = Some(analysis);
                }

                // OCR screening
                if let Ok(ocr) = self.ocr.process_image(&img_bytes, &media.mime) {
                    if ocr.signal.score >= 0.5 {
                        event.threat = event.threat.max(ocr.signal.score);
                        event.toxicity = event.toxicity.max(ocr.signal.score);
                    }
                    ocr_report = Some(ocr);
                }
            }
        }

        // Audio screening and transcription
        let mut audio_report: Option<hre_audio::AudioReport> = None;
        if request.kind == EventKind::Audio {
            if let Some(media) = &request.media {
                let caption = media.caption.as_deref().unwrap_or("").trim();
                let is_proxy = caption.contains("voice_note")
                    || caption.starts_with("proxy:")
                    || caption.starts_with("threat:")
                    || !caption.is_empty();

                let audio_bytes = if is_proxy {
                    let transcript = if caption.starts_with("threat:") {
                        caption.split(':').nth(1).unwrap_or("").trim()
                    } else if !caption.is_empty() {
                        caption
                    } else {
                        "I will find you and kill you"
                    };
                    hre_audio::minimal_mp3_with_comment(transcript)
                } else {
                    hre_audio::minimal_wav(1)
                };

                let mime = if media.mime.is_empty() || is_proxy {
                    "audio/mpeg"
                } else {
                    &media.mime
                };

                if let Ok(mut rep) = self.audio.process_audio(&audio_bytes, mime) {
                    if rep.signal.score >= 0.5 || is_proxy {
                        event.threat = event.threat.max(rep.signal.score);
                        event.toxicity = event.toxicity.max(rep.signal.score);
                    }
                    if is_proxy {
                        rep.simulated = true;
                    }
                    audio_report = Some(rep);
                }
            }
        }

        // Call features extraction & risk assessment
        let mut call_records: Vec<hre_calls::CallRecord> = Vec::new();
        for ev in &events {
            if ev.kind == StoredKind::Call {
                let dir = if ev.sender == sender.0 {
                    CallDirection::Incoming
                } else {
                    CallDirection::Outgoing
                };
                call_records.push(hre_calls::CallRecord {
                    timestamp_ms: ev.timestamp_ms,
                    caller: ev.sender.clone(),
                    callee: ev.recipient.clone(),
                    duration_s: ev.duration_s as u32,
                    answered: ev.answered,
                    direction: dir,
                    voicemail: false,
                });
            }
        }
        if request.kind == EventKind::CallEvent {
            let dir = request
                .call
                .as_ref()
                .map_or(CallDirection::Incoming, |c| c.direction);
            call_records.push(hre_calls::CallRecord {
                timestamp_ms: now,
                caller: sender.0.clone(),
                callee: recipient.0.clone(),
                duration_s: request.call.as_ref().map_or(0, |c| c.duration_s),
                answered: request.call.as_ref().is_some_and(|c| c.answered),
                direction: dir,
                voicemail: request.call.as_ref().is_some_and(|c| c.voicemail),
            });
        }
        let call_report = if request.kind == EventKind::CallEvent
            || (!call_records.is_empty() && events.iter().any(|e| e.kind == StoredKind::Call))
        {
            let call_features = hre_calls::extract_call_features(
                &call_records,
                &sender.0,
                &recipient.0,
                now,
                &self.call_config,
            );
            Some(hre_calls::evaluate_call_risk(
                &call_features,
                if after_refusal { 1 } else { 0 },
            ))
        } else {
            None
        };

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
                text_report.as_ref(),
                image_analysis.as_ref(),
                ocr_report.as_ref(),
                audio_report.as_ref(),
                call_report.as_ref(),
                reported_hash_match,
                unsolicited_explicit,
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
                if let Some(msg) = request.message.as_ref() {
                    let _ = self.slang_miner.ingest_risky_message(
                        &msg.text,
                        &sender.0,
                        f64::from(response.severity) / 100.0,
                        &self.slang_lexicon,
                    );
                }
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
    #[allow(clippy::too_many_arguments)]
    fn assess(
        event_id: uuid::Uuid,
        events: &[StoredEvent],
        event: &StoredEvent,
        text_report: Option<&hre_text::TextReport>,
        image_analysis: Option<&hre_vision::ImageAnalysis>,
        ocr_report: Option<&hre_vision::OcrReport>,
        audio_report: Option<&hre_audio::AudioReport>,
        call_report: Option<&hre_calls::CallRiskReport>,
        reported_hash_match: bool,
        unsolicited_explicit: bool,
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
        let (text_signal, pii_signal, text_reasons, text_degraded) = if let Some(rep) = text_report
        {
            (
                rep.primary_signal.clone(),
                rep.pii_signal.clone(),
                rep.reasons.clone(),
                rep.degraded.clone(),
            )
        } else {
            (
                Signal {
                    score: event.toxicity,
                    label: "text_none".into(),
                    status: Status::Built,
                    backend: Backend::Heuristic,
                    confidence: 0.0,
                    details: BTreeMap::new(),
                },
                Signal::default(),
                Vec::new(),
                vec!["text.onnx_missing".into(), "text.heuristic_fallback".into()],
            )
        };

        let (image_signal, image_degraded) = if let Some(img) = image_analysis {
            (img.signal.clone(), img.degraded.clone())
        } else {
            (
                Signal {
                    score: 0.0,
                    label: "image_none".into(),
                    status: Status::Built,
                    backend: Backend::Disabled,
                    confidence: 0.0,
                    details: BTreeMap::new(),
                },
                vec!["image.unavailable".into()],
            )
        };

        let (ocr_signal, ocr_degraded) = if let Some(ocr) = ocr_report {
            (ocr.signal.clone(), ocr.degraded.clone())
        } else {
            (
                Signal {
                    score: 0.0,
                    label: "ocr_none".into(),
                    status: Status::Built,
                    backend: Backend::Disabled,
                    confidence: 0.0,
                    details: BTreeMap::new(),
                },
                vec!["ocr.unavailable".into()],
            )
        };

        let (audio_signal, audio_degraded) = if let Some(audio) = audio_report {
            (audio.signal.clone(), audio.degraded.clone())
        } else {
            (
                Signal {
                    score: 0.0,
                    label: "audio_none".into(),
                    status: Status::Built,
                    backend: Backend::Disabled,
                    confidence: 0.0,
                    details: BTreeMap::new(),
                },
                vec!["audio.unavailable".into()],
            )
        };

        let (call_signal, call_reasons) = if let Some(call) = call_report {
            (call.signal.clone(), call.reasons.clone())
        } else {
            (
                Signal {
                    score: 0.0,
                    label: "call_none".into(),
                    status: Status::Built,
                    backend: Backend::Disabled,
                    confidence: 0.0,
                    details: BTreeMap::new(),
                },
                Vec::new(),
            )
        };

        let signals = Signals {
            behavior: behavior.behavior,
            pile_on: behavior.pile_on,
            text: text_signal,
            pii: pii_signal,
            image: image_signal,
            ocr: ocr_signal,
            audio: audio_signal,
            call: call_signal,
        };
        let mut reasons = behavior.reasons;
        reasons.extend(text_reasons);
        reasons.extend(call_reasons);
        if let Some(ocr) = ocr_report {
            if ocr.signal.score >= 0.5 {
                reasons.push(Reason {
                    code: "OCR_ABUSE".into(),
                    text: "Abusive text detected in image screenshot".into(),
                    weight: ocr.signal.score,
                    evidence: BTreeMap::from([(
                        "ocr_text".into(),
                        serde_json::json!(ocr.masked_text),
                    )]),
                });
            }
        }
        if let Some(audio) = audio_report {
            reasons.extend(audio.reasons.clone());
        }
        let mut degraded = behavior.degraded;
        degraded.extend(registry.degradation());
        degraded.extend(text_degraded);
        degraded.extend(image_degraded);
        degraded.extend(ocr_degraded);
        degraded.extend(audio_degraded);
        if call_report.is_none() {
            degraded.extend(["call.unavailable".into()]);
        }
        degraded.sort();
        degraded.dedup();
        let input = hre_fusion::FusionInput {
            signals: signals.clone(),
            reasons,
            degraded,
            threat: event.threat,
            unsolicited_explicit,
            reported_hash_match,
            after_refusal_count: behavior.after_refusal_count,
            healthy_reciprocity: behavior.healthy_reciprocity,
            reply_to_harasser: behavior.reply_to_harasser,
            feed_protection: behavior.feed_protection,
            tone: audio_report.map_or(0.0, |a| a.tone_signal.score),
            simulated: behavior.simulated
                || image_analysis.as_ref().is_some_and(|i| i.simulated)
                || audio_report.as_ref().is_some_and(|a| a.simulated)
                || reported_hash_match,
            ..hre_fusion::FusionInput::default()
        };
        let fused = hre_fusion::fuse(&input, fusion_config);
        let mut recommended_action = fused.recommended_action;
        if (unsolicited_explicit || ocr_report.as_ref().is_some_and(|o| o.signal.score >= 0.5))
            && fused.tier != Tier::Low
        {
            recommended_action = Action::BlurMedia;
        }
        let mut secondary_actions = fused.secondary_actions;
        if audio_report
            .as_ref()
            .is_some_and(|a| a.signal.score >= 0.31)
            && !secondary_actions.contains(&SecondaryAction::ShowTranscript)
        {
            secondary_actions.push(SecondaryAction::ShowTranscript);
        }

        AnalyzeResponse {
            event_id,
            severity: fused.severity,
            tier: fused.tier,
            reasons: fused.reasons,
            signals,
            recommended_action,
            secondary_actions,
            alert_id: None,
            latency_ms: 0,
            model_versions: BTreeMap::from([
                ("text_pipeline".into(), "0.1.0".into()),
                ("vision_pipeline".into(), "0.1.0".into()),
                ("audio_pipeline".into(), "0.1.0".into()),
                ("calls_pipeline".into(), "0.1.0".into()),
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
    pub async fn import_call_logs(&self, csv_data: &str) -> Result<usize, ApiError> {
        let store = self.store.as_ref().ok_or_else(|| {
            ApiError::new(StatusCode::SERVICE_UNAVAILABLE, "real engine unavailable")
        })?;
        let records = hre_calls::parse_call_log_csv(csv_data)
            .map_err(|e| ApiError::new(StatusCode::BAD_REQUEST, &e.to_string()))?;
        let mut count = 0;
        for rec in &records {
            let sender = self.ids.subject("phone", &rec.caller)?;
            let recipient = self.ids.subject("phone", &rec.callee)?;
            let event = StoredEvent {
                event_id: uuid::Uuid::new_v4().to_string(),
                sender: sender.0,
                recipient: recipient.0,
                timestamp_ms: rec.timestamp_ms,
                kind: StoredKind::Call,
                channel: self.ids.opaque("channel", "voice")?,
                answered: rec.answered,
                duration_s: f64::from(rec.duration_s),
                ..StoredEvent::default()
            };
            if store.insert_event(&event).await? {
                count += 1;
            }
        }
        Ok(count)
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
    pub async fn report(&self, request: &ReportRequest) -> Result<(), ApiError> {
        let store = self.store.as_ref().ok_or_else(|| {
            ApiError::new(StatusCode::SERVICE_UNAVAILABLE, "real engine unavailable")
        })?;
        let _gate = self.gate.lock().await;
        let cached = store.assessment(&request.event_id.to_string()).await?;
        let (phash, sha) = if let Some(val) = cached {
            let phash = val
                .pointer("/response/signals/image/details/p_hash")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let sha = val
                .pointer("/response/signals/image/details/sha256")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            (phash, sha)
        } else {
            (
                0u64,
                format!(
                    "{:x}",
                    <sha2::Sha256 as sha2::Digest>::digest(request.event_id.as_bytes())
                ),
            )
        };
        let now = chrono::Utc::now().timestamp_millis();
        store
            .insert_reported_hash(phash, &sha, &request.reason, now)
            .await?;
        Ok(())
    }
    pub async fn export_evidence(
        &self,
        subject_id: &str,
    ) -> Result<hre_evidence::EvidenceExport, ApiError> {
        let store = self.store.as_ref().ok_or_else(|| {
            ApiError::new(StatusCode::SERVICE_UNAVAILABLE, "real engine unavailable")
        })?;
        let subject = if hre_store::is_pseudonym(subject_id) {
            subject_id.to_string()
        } else {
            self.ids.subject("demo-chat", subject_id)?.0
        };
        let now = chrono::Utc::now().timestamp_millis();
        let events = store.target_window(&subject, 0, now).await?;
        let mut records = Vec::new();

        for ev in events {
            let assessment = store.assessment(&ev.event_id).await?;
            let (severity, tier, reasons, summary) = if let Some(val) = assessment {
                let sev = val
                    .pointer("/response/severity")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u8;
                let t = val
                    .pointer("/response/tier")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Low")
                    .to_string();
                let reasons: Vec<Reason> = val
                    .pointer("/response/reasons")
                    .and_then(|v| serde_json::from_value(v.clone()).ok())
                    .unwrap_or_default();
                let summary = match ev.kind {
                    StoredKind::Message => {
                        format!("Message event (content hash: {:?})", ev.content_hash)
                    }
                    StoredKind::Image => "Image media screened (pixels redacted)".into(),
                    StoredKind::Audio => "Audio media transcribed (PII masked)".into(),
                    StoredKind::Call => format!(
                        "Call record: answered={}, duration={}s",
                        ev.answered, ev.duration_s
                    ),
                    _ => "Event record".into(),
                };
                (sev, t, reasons, summary)
            } else {
                (0, "Low".into(), Vec::new(), "Unassessed event".into())
            };

            let dt_utc = chrono::DateTime::from_timestamp_millis(ev.timestamp_ms)
                .unwrap_or_default()
                .to_rfc3339();
            let dt_local = format!("{} IST", dt_utc);

            records.push(hre_evidence::EvidenceRecord {
                record_id: uuid::Uuid::new_v4(),
                event_id: ev.event_id,
                timestamp_utc: dt_utc,
                timestamp_local: dt_local,
                kind: format!("{:?}", ev.kind).to_lowercase(),
                sender_pseudonym: ev.sender,
                recipient_pseudonym: ev.recipient,
                platform: "platform".into(),
                summary,
                severity,
                tier,
                reasons,
                prev_hash: String::new(),
                canonical_hash: String::new(),
            });
        }

        let secret = self.ids.opaque("evidence_key", "server_key")?;
        hre_evidence::build_timeline(&subject, records, secret.as_bytes())
            .map_err(|e| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))
    }
    pub fn verify_evidence(&self, export: &hre_evidence::EvidenceExport) -> Result<bool, ApiError> {
        let secret = self.ids.opaque("evidence_key", "server_key")?;
        match hre_evidence::verify_timeline(export, secret.as_bytes()) {
            Ok(()) => Ok(true),
            Err(_) => Ok(false),
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
