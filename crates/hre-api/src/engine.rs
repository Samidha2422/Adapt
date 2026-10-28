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
        let demo_mode = config.demo_mode;
        let image = tokio::task::spawn_blocking(move || {
            if demo_mode {
                hre_vision::HeuristicImageBackend::from_config_file(std::path::Path::new(
                    "config/demo_overrides.toml",
                ))
            } else {
                hre_vision::HeuristicImageBackend::new()
            }
        })
        .await?;
        let ocr = hre_vision::HeuristicOcrBackend::new();
        let audio = hre_audio::HeuristicAudioBackend::new();
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
        self.analyze_image(request, None).await
    }
    pub async fn analyze_image(
        &self,
        request: &AnalyzeRequest,
        image_bytes: Option<Vec<u8>>,
    ) -> Result<(AnalyzeResponse, bool), ApiError> {
        self.analyze_inner(request, image_bytes, None).await
    }
    pub async fn parse_calls(
        &self,
        csv: String,
        platform: String,
    ) -> Result<Vec<hre_calls::safe_import::CallRecord>, ApiError> {
        let ids = self.ids.clone();
        tokio::task::spawn_blocking(move || hre_calls::import_csv(csv.as_bytes(), &ids, &platform))
            .await
            .map_err(|_| {
                ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "CSV worker unavailable")
            })?
            .map_err(|error| ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, &error.to_string()))
    }
    pub async fn analyze_call(
        &self,
        record: &hre_calls::safe_import::CallRecord,
        platform: &str,
        conversation_id: &str,
        draft: bool,
    ) -> Result<(AnalyzeResponse, bool), ApiError> {
        let request = AnalyzeRequest {
            event_id: record.event_id,
            kind: EventKind::CallEvent,
            occurred_at: record.occurred_at,
            draft,
            // The parser already HMACed these; analyze_inner must not hash them again.
            sender_id: record.caller.0.clone(),
            recipient_id: record.callee.0.clone(),
            platform: platform.into(),
            channel: Channel {
                kind: ChannelType::Call,
                conversation_id: conversation_id.into(),
                group_id: None,
                post_id: None,
            },
            context: EventContext {
                stop_requested: record.after_refusal,
                ..Default::default()
            },
            message: None,
            media: None,
            call: Some(Call {
                direction: CallDirection::Outgoing,
                duration_s: record.duration_s,
                answered: record.answered,
                voicemail: record.voicemail,
            }),
        };
        request.validate()?;
        self.analyze_inner(&request, None, Some(record)).await
    }
    async fn analyze_inner(
        &self,
        request: &AnalyzeRequest,
        image_bytes: Option<Vec<u8>>,
        imported_call: Option<&hre_calls::safe_import::CallRecord>,
    ) -> Result<(AnalyzeResponse, bool), ApiError> {
        let started = Instant::now();
        let envelope = request.clone();
        let image = self.image.clone();
        let ocr = self.ocr.clone();
        let (mut text_report, image_analysis, ocr_report) =
            tokio::task::spawn_blocking(move || {
                let text = envelope
                    .message
                    .as_ref()
                    .map(|m| m.text.as_str())
                    .or_else(|| envelope.media.as_ref().and_then(|m| m.caption.as_deref()));
                let text_report = text
                    .filter(|s| !s.trim().is_empty())
                    .map(hre_text::analyze)
                    .transpose()
                    .map_err(|_| {
                        ApiError::new(
                            StatusCode::UNPROCESSABLE_ENTITY,
                            "invalid or oversized text",
                        )
                    })?;
                let (image_analysis, ocr_report) = if let Some(bytes) = image_bytes {
                    let mime = envelope.media.as_ref().map_or("", |m| m.mime.as_str());
                    let mut analysis = image.analyze(&bytes, mime).map_err(|_| {
                        ApiError::new(
                            StatusCode::UNPROCESSABLE_ENTITY,
                            "invalid image or MIME mismatch",
                        )
                    })?;
                    // Encoded-byte sampling is not a perceptual image hash.
                    analysis.signal.details.remove("p_hash");
                    analysis
                        .signal
                        .details
                        .insert("source".into(), serde_json::json!("uploaded_bytes"));
                    analysis
                        .degraded
                        .push("image.perceptual_hash_unavailable".into());
                    if !analysis.simulated {
                        analysis.signal.status = Status::Roadmap;
                        analysis.signal.backend = Backend::Disabled;
                        analysis.signal.label = "image_model_unavailable".into();
                    }
                    let mut report = ocr.process_image(&bytes, mime).map_err(|_| {
                        ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, "invalid image for OCR")
                    })?;
                    if !report.raw_text.is_empty() {
                        report.simulated = true;
                        report.signal.status = Status::Simulated;
                        report.signal.label = "embedded_metadata_text".into();
                        report
                            .degraded
                            .push("ocr.metadata_extraction_simulated".into());
                    } else {
                        report.signal.status = Status::Roadmap;
                        report.signal.backend = Backend::Disabled;
                    }
                    // Keep derived counts/scores, never persist or expose media text bodies.
                    report.signal.details.remove("ocr_text_masked");
                    (Some(analysis), Some(report))
                } else {
                    (None, None)
                };
                Ok::<_, ApiError>((text_report, image_analysis, ocr_report))
            })
            .await
            .map_err(|_| {
                ApiError::new(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "analysis worker unavailable",
                )
            })??;
        if let (Some(message), Some(report)) = (&request.message, &mut text_report) {
            if let Ok(matches) = self.slang_lexicon.find_matches(&message.text) {
                if let Some(term) = matches.first() {
                    report.primary_signal.score = report.primary_signal.score.max(term.severity);
                    report.primary_signal.label = "slang_harassment".into();
                    report.primary_signal.backend = Backend::Heuristic;
                    report.primary_signal.status = Status::Built;
                    report.toxicity = report.toxicity.max(term.severity);
                    report.reasons.push(Reason {
                        code: "SLANG_MATCH".into(),
                        text: format!(
                            "Confirmed slang term '{}' detected: {}",
                            term.term, term.meaning
                        ),
                        weight: term.severity,
                        evidence: BTreeMap::from([
                            ("term".into(), serde_json::json!(term.term)),
                            ("meaning".into(), serde_json::json!(term.meaning)),
                        ]),
                    });
                }
            }
        }
        let _gate = self.gate.lock().await;
        let store = self.store.as_ref().ok_or_else(|| {
            ApiError::new(StatusCode::SERVICE_UNAVAILABLE, "real engine unavailable")
        })?;
        let encoded_request = if let Some(image) = &image_analysis {
            serde_json::to_string(&(request, &image.sha256))
        } else if let Some(call) = imported_call {
            serde_json::to_string(&(request, call.simulated))
        } else {
            serde_json::to_string(request)
        }
        .map_err(|_| ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, "cannot encode request"))?;
        let request_hash = self.ids.opaque("request", &encoded_request)?;
        if !request.draft {
            if let Some(cached) = store.assessment(&request.event_id.to_string()).await? {
                return cached_response(&cached, &request_hash).map(|response| (response, false));
            }
        }
        if matches!(request.kind, EventKind::Audio) {
            return Err(ApiError::new(
                StatusCode::NOT_IMPLEMENTED,
                "audio analysis requires the audio pipeline; use --mock for client fixtures",
            ));
        }
        let (sender, recipient) = if let Some(call) = imported_call {
            (call.caller.clone(), call.callee.clone())
        } else {
            (
                self.ids.subject(&request.platform, &request.sender_id)?,
                self.ids.subject(&request.platform, &request.recipient_id)?,
            )
        };
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
                .map(|m| m.text.as_str())
                .or_else(|| request.media.as_ref().and_then(|m| m.caption.as_deref()))
                .map(|text| self.ids.opaque("content", text))
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
        let mut reported_hash_match = false;
        let mut unsolicited_explicit = false;
        if let Some(rep) = &text_report {
            event.toxicity = rep.toxicity;
            event.threat = rep.threat;
            event.sexual = rep.sexual_explicit;
            event.pii_mentions = rep.pii_matches.len() as u32;
            event.location_cues = rep.location_cues as u32;
        }
        if let Some(ocr) = &ocr_report {
            if let Some(rep) = &ocr.text_report {
                event.toxicity = event.toxicity.max(rep.toxicity);
                event.threat = event.threat.max(rep.threat);
                event.sexual = event.sexual.max(rep.sexual_explicit);
                event.pii_mentions += rep.pii_matches.len() as u32;
                event.location_cues += rep.location_cues as u32;
            }
        }
        if let Some(analysis) = &image_analysis {
            // Exact SHA comparison prevents encoded-byte hash collisions becoming verdicts.
            let key = image_hash_key(&analysis.sha256)?;
            reported_hash_match = store
                .find_reported_hash(key, 0)
                .await?
                .is_some_and(|matched| matched.sha256 == analysis.sha256);
            unsolicited_explicit = analysis.signal.score >= 0.5 && !request.context.is_contact;
        }
        event.unsolicited_media = unsolicited_explicit;
        event.simulated = image_analysis.as_ref().is_some_and(|i| i.simulated)
            || ocr_report.as_ref().is_some_and(|o| o.simulated)
            || reported_hash_match
            || imported_call.is_some_and(|c| c.simulated);
        let audio_report: Option<hre_audio::AudioReport> = None;
        events.retain(|e| e.event_id != event.event_id);
        events.push(event.clone());
        let behavior_config = self.behavior.clone();
        let fusion_config = self.fusion.clone();
        let registry = self.registry.clone();
        let call_config = hre_calls::CallFeatureConfig {
            night_start_hour: self.behavior.night_start_hour,
            night_end_hour: self.behavior.night_end_hour,
            utc_offset_minutes: self.behavior.recipient_utc_offset_minutes,
        };
        let voicemail = request.call.as_ref().is_some_and(|call| call.voicemail);
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
                reported_hash_match,
                unsolicited_explicit,
                &behavior_config,
                &fusion_config,
                &registry,
                &call_config,
                voicemail,
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
        reported_hash_match: bool,
        unsolicited_explicit: bool,
        behavior_config: &hre_behavior::BehaviorConfig,
        fusion_config: &hre_fusion::FusionConfig,
        registry: &hre_ml::ModelRegistry,
        call_config: &hre_calls::CallFeatureConfig,
        voicemail: bool,
    ) -> AnalyzeResponse {
        let behavior = hre_behavior::analyze(
            events,
            &event.sender,
            &event.recipient,
            event.timestamp_ms,
            behavior_config,
        );
        let calls: Vec<_> = events
            .iter()
            .filter(|item| {
                item.kind == StoredKind::Call
                    && item.sender == event.sender
                    && item.recipient == event.recipient
            })
            .map(|item| hre_calls::CallRecord {
                timestamp_ms: item.timestamp_ms,
                caller: item.sender.clone(),
                callee: item.recipient.clone(),
                duration_s: item.duration_s as u32,
                answered: item.answered,
                direction: CallDirection::Incoming,
                voicemail: item.event_id == event.event_id && voicemail,
            })
            .collect();
        let call_report = if calls.is_empty() {
            None
        } else {
            let features = hre_calls::extract_call_features(
                &calls,
                &event.sender,
                &event.recipient,
                event.timestamp_ms,
                call_config,
            );
            let after_refusal = events
                .iter()
                .filter(|item| {
                    item.kind == StoredKind::Call
                        && item.sender == event.sender
                        && item.recipient == event.recipient
                        && item.after_refusal
                        && item.timestamp_ms <= event.timestamp_ms
                })
                .count();
            let mut report = hre_calls::evaluate_call_risk(&features, after_refusal);
            if events.iter().any(|item| {
                item.kind == StoredKind::Call
                    && item.sender == event.sender
                    && item.recipient == event.recipient
                    && item.simulated
                    && item.timestamp_ms <= event.timestamp_ms
                    && item.timestamp_ms >= event.timestamp_ms.saturating_sub(7 * 86_400_000)
            }) {
                report.signal.status = Status::Simulated;
            }
            Some(report)
        };
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
                    score: 0.0,
                    label: "text_none".into(),
                    status: Status::Roadmap,
                    backend: Backend::Disabled,
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
                    status: Status::Roadmap,
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
                    status: Status::Roadmap,
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
                    status: Status::Roadmap,
                    backend: Backend::Disabled,
                    confidence: 0.0,
                    details: BTreeMap::new(),
                },
                vec!["audio.unavailable".into()],
            )
        };

        let mut signals = Signals {
            behavior: behavior.behavior,
            pile_on: behavior.pile_on,
            text: text_signal,
            pii: pii_signal,
            image: image_signal,
            ocr: ocr_signal,
            audio: audio_signal,
            call: call_report
                .as_ref()
                .map_or_else(Signal::default, |report| report.signal.clone()),
        };
        let mut reasons = behavior.reasons;
        reasons.extend(text_reasons);
        if let Some(report) = &call_report {
            reasons.extend(report.reasons.clone());
        }
        if let Some(ocr) = ocr_report {
            if let Some(report) = &ocr.text_report {
                if report.primary_signal.score > signals.text.score {
                    signals.text = report.primary_signal.clone();
                    signals.text.status = Status::Simulated;
                }
                if report.pii_signal.score > signals.pii.score {
                    signals.pii = report.pii_signal.clone();
                    signals.pii.status = Status::Simulated;
                }
                reasons.extend(report.reasons.clone());
            }
            if ocr.signal.score >= 0.5 {
                reasons.push(Reason {
                    code: "OCR_ABUSE".into(),
                    text: "Abusive text in image metadata (simulated OCR)".into(),
                    weight: ocr.signal.score,
                    evidence: BTreeMap::new(),
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
            degraded.push("call.unavailable".into());
        }
        degraded.sort();
        degraded.dedup();
        let input = hre_fusion::FusionInput {
            signals: signals.clone(),
            reasons,
            degraded,
            threat: event.threat,
            doxxing_with_toxicity: event.pii_mentions > 0 && event.toxicity >= 0.6,
            unsolicited_explicit,
            reported_hash_match,
            after_refusal_count: behavior.after_refusal_count,
            healthy_reciprocity: behavior.healthy_reciprocity,
            reply_to_harasser: behavior.reply_to_harasser,
            feed_protection: behavior.feed_protection,
            tone: audio_report.map_or(0.0, |a| a.tone_signal.score),
            simulated: event.simulated
                || behavior.simulated
                || image_analysis.as_ref().is_some_and(|i| i.simulated)
                || audio_report.as_ref().is_some_and(|a| a.simulated)
                || ocr_report.is_some_and(|o| o.simulated)
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
    pub async fn import_call_logs(&self, csv: String) -> Result<usize, ApiError> {
        if csv.len() > hre_calls::MAX_CSV_BYTES {
            return Err(ApiError::new(
                StatusCode::PAYLOAD_TOO_LARGE,
                "CSV exceeds two MiB",
            ));
        }
        let records = tokio::task::spawn_blocking(move || hre_calls::parse_call_log_csv(&csv))
            .await
            .map_err(|_| {
                ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "CSV worker unavailable")
            })?
            .map_err(|_| ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, "invalid call CSV"))?;
        let store = self.store.as_ref().ok_or_else(|| {
            ApiError::new(StatusCode::SERVICE_UNAVAILABLE, "real engine unavailable")
        })?;
        let _gate = self.gate.lock().await;
        let mut imported = 0;
        for record in records {
            let event = StoredEvent {
                event_id: uuid::Uuid::new_v4().to_string(),
                sender: self.ids.subject("phone", &record.caller)?.0,
                recipient: self.ids.subject("phone", &record.callee)?.0,
                timestamp_ms: record.timestamp_ms,
                kind: StoredKind::Call,
                channel: self.ids.opaque("channel", "voice")?,
                answered: record.answered,
                duration_s: f64::from(record.duration_s),
                ..StoredEvent::default()
            };
            if store.insert_event(&event).await? {
                imported += 1;
            }
        }
        Ok(imported)
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
        if request.reason.trim().is_empty() || request.reason.len() > 1024 {
            return Err(ApiError::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "reason must contain 1..1024 bytes",
            ));
        }
        let cached = store
            .assessment(&request.event_id.to_string())
            .await?
            .ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "analyzed event not found"))?;
        if cached
            .pointer("/response/signals/image/details/source")
            .and_then(|v| v.as_str())
            != Some("uploaded_bytes")
        {
            return Err(ApiError::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "event has no uploaded image",
            ));
        }
        let sha = cached
            .pointer("/response/signals/image/details/sha256")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                ApiError::new(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "event has no uploaded image hash",
                )
            })?;
        let key = image_hash_key(sha)?;
        store
            .insert_reported_hash(
                key,
                sha,
                "user_reported_media",
                chrono::Utc::now().timestamp_millis(),
            )
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
fn image_hash_key(sha: &str) -> Result<u64, ApiError> {
    if sha.len() != 64 || !sha.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid image hash",
        ));
    }
    u64::from_str_radix(&sha[..16], 16)
        .map_err(|_| ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, "invalid image hash"))
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
