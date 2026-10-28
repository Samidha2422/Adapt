use crate::validate::{validate_audio, AudioError};
use hre_core::{Action, Backend, Reason, SecondaryAction, Signal, Status};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioTranscription {
    pub transcript: String,
    pub language: String,
    pub duration_s: f64,
    pub tone_score: f64,
    pub simulated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioReport {
    pub raw_transcript: String,
    pub masked_transcript: String,
    pub language: String,
    pub duration_s: f64,
    pub text_report: Option<hre_text::TextReport>,
    pub signal: Signal,
    pub tone_signal: Signal,
    pub reasons: Vec<Reason>,
    pub recommended_action: Action,
    pub secondary_actions: Vec<SecondaryAction>,
    pub simulated: bool,
    pub degraded: Vec<String>,
}

pub trait AudioBackend: Send + Sync {
    fn transcribe(
        &self,
        bytes: &[u8],
        declared_mime: &str,
    ) -> Result<AudioTranscription, AudioError>;
}

/// Fallback and synthetic audio transcriber with tone evaluation.
#[derive(Debug, Clone, Default)]
pub struct HeuristicAudioBackend {
    fixture_transcripts: BTreeMap<String, (String, f64)>, // hash -> (transcript, tone_score)
}

impl HeuristicAudioBackend {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_fixture(mut self, hash_or_tag: &str, transcript: &str, tone_score: f64) -> Self {
        self.fixture_transcripts.insert(
            hash_or_tag.to_string(),
            (transcript.to_string(), tone_score.clamp(0.0, 1.0)),
        );
        self
    }

    pub fn process_audio(
        &self,
        bytes: &[u8],
        declared_mime: &str,
    ) -> Result<AudioReport, AudioError> {
        let _meta = validate_audio(bytes, declared_mime)?;

        let transcription = self.transcribe(bytes, declared_mime)?;
        let raw_transcript = transcription.transcript;

        if raw_transcript.trim().is_empty() {
            let mut details = BTreeMap::new();
            details.insert("transcript_masked".into(), serde_json::json!(""));

            return Ok(AudioReport {
                raw_transcript: String::new(),
                masked_transcript: String::new(),
                language: transcription.language,
                duration_s: transcription.duration_s,
                text_report: None,
                signal: Signal {
                    score: 0.0,
                    label: "audio_none".into(),
                    status: Status::Built,
                    backend: Backend::Disabled,
                    confidence: 0.0,
                    details: details.clone(),
                },
                tone_signal: Signal {
                    score: 0.0,
                    label: "tone_none".into(),
                    status: Status::Built,
                    backend: Backend::Disabled,
                    confidence: 0.0,
                    details,
                },
                reasons: Vec::new(),
                recommended_action: Action::Log,
                secondary_actions: Vec::new(),
                simulated: transcription.simulated,
                degraded: vec!["audio.onnx_missing".into(), "whisper.ggml_missing".into()],
            });
        }

        // Mask PII before passing downstream or displaying transcript
        let (masked_transcript, _) = hre_text::mask_pii(&raw_transcript);

        // Forward transcript to hre-text for toxicity, threat, cue analysis
        let text_report = hre_text::analyze(&raw_transcript).ok();

        let mut score = 0.0;
        let mut reasons = Vec::new();
        if let Some(ref rep) = text_report {
            score = rep.toxicity.max(rep.threat).max(rep.sexual_explicit);
            reasons = rep.reasons.clone();
        }

        // Supporting tone signal adds up to +10 severity in fusion, but never flags alone
        let tone_score = transcription.tone_score;
        let mut tone_details = BTreeMap::new();
        tone_details.insert("tone_score".into(), serde_json::json!(tone_score));
        tone_details.insert("max_addition".into(), serde_json::json!(10.0));

        let tone_signal = Signal {
            score: tone_score,
            label: if tone_score >= 0.5 {
                "aggressive_tone".into()
            } else {
                "neutral_tone".into()
            },
            status: if transcription.simulated {
                Status::Simulated
            } else {
                Status::Built
            },
            backend: Backend::Heuristic,
            confidence: 0.75,
            details: tone_details,
        };

        if tone_score >= 0.6 && score >= 0.3 {
            reasons.push(Reason {
                code: "AGGRESSIVE_TONE".into(),
                text: "Aggressive or agitated tone detected in voice recording".into(),
                weight: tone_score * 0.10,
                evidence: BTreeMap::from([("tone_score".into(), serde_json::json!(tone_score))]),
            });
        }

        let tier = if score >= 0.86 {
            hre_core::Tier::Urgent
        } else if score >= 0.61 {
            hre_core::Tier::High
        } else if score >= 0.31 {
            hre_core::Tier::Review
        } else {
            hre_core::Tier::Low
        };

        let recommended_action = match tier {
            hre_core::Tier::Low => Action::Log,
            hre_core::Tier::Review => Action::WarnSender,
            hre_core::Tier::High => Action::Throttle,
            hre_core::Tier::Urgent => Action::Escalate,
        };

        // When abusive voice recording is detected, show transcript instead of auto-playing audio!
        let mut secondary_actions = Vec::new();
        if score >= 0.31 {
            secondary_actions.push(SecondaryAction::ShowTranscript);
        }
        if matches!(tier, hre_core::Tier::High | hre_core::Tier::Urgent) {
            secondary_actions.push(SecondaryAction::AlertModerator);
        }

        let mut details = BTreeMap::new();
        details.insert(
            "transcript_masked".into(),
            serde_json::json!(masked_transcript.clone()),
        );
        details.insert(
            "duration_s".into(),
            serde_json::json!(transcription.duration_s),
        );

        let audio_signal = Signal {
            score,
            label: if score >= 0.31 {
                "audio_harassment".into()
            } else {
                "audio_benign".into()
            },
            status: if transcription.simulated {
                Status::Simulated
            } else {
                Status::Built
            },
            backend: Backend::Heuristic,
            confidence: 0.85,
            details,
        };

        Ok(AudioReport {
            raw_transcript,
            masked_transcript,
            language: transcription.language,
            duration_s: transcription.duration_s,
            text_report,
            signal: audio_signal,
            tone_signal,
            reasons,
            recommended_action,
            secondary_actions,
            simulated: transcription.simulated,
            degraded: vec!["audio.onnx_missing".into(), "whisper.ggml_missing".into()],
        })
    }
}

impl AudioBackend for HeuristicAudioBackend {
    fn transcribe(
        &self,
        bytes: &[u8],
        declared_mime: &str,
    ) -> Result<AudioTranscription, AudioError> {
        let _meta = validate_audio(bytes, declared_mime)?;

        let mut hasher = Sha256::new();
        hasher.update(bytes);
        let hash = format!("{:x}", hasher.finalize());

        if let Some((transcript, tone)) = self.fixture_transcripts.get(&hash) {
            return Ok(AudioTranscription {
                transcript: transcript.clone(),
                language: "en".into(),
                duration_s: (bytes.len() as f64 / 32000.0).clamp(1.0, 30.0),
                tone_score: *tone,
                simulated: true,
            });
        }

        // Try extracting embedded ID3 comment or tag from synthetic audio fixtures
        if let Some(text) = extract_id3_comment(bytes) {
            return Ok(AudioTranscription {
                transcript: text,
                language: "en".into(),
                duration_s: 5.0,
                tone_score: 0.8,
                simulated: true,
            });
        }

        // Honest zero-model fallback
        Ok(AudioTranscription {
            transcript: String::new(),
            language: "und".into(),
            duration_s: 0.0,
            tone_score: 0.0,
            simulated: false,
        })
    }
}

fn extract_id3_comment(bytes: &[u8]) -> Option<String> {
    if bytes.starts_with(b"ID3") && bytes.len() > 10 {
        // Simple search for COMM or USLT frame
        for marker in [b"COMM", b"USLT", b"TIT2"] {
            if let Some(pos) = bytes.windows(4).position(|w| w == marker) {
                if pos + 10 < bytes.len() {
                    let frame_len = u32::from_be_bytes([
                        bytes[pos + 4],
                        bytes[pos + 5],
                        bytes[pos + 6],
                        bytes[pos + 7],
                    ]) as usize;
                    let start = pos + 10;
                    let end = (start + frame_len).min(bytes.len());
                    if let Ok(text) = std::str::from_utf8(&bytes[start..end]) {
                        let clean = text.trim_matches('\0').trim();
                        if !clean.is_empty() {
                            return Some(clean.to_string());
                        }
                    }
                }
            }
        }
    }
    None
}
