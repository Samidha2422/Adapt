pub mod jobs;
pub mod transcribe;
pub mod validate;

pub use jobs::{JobManager, JobRecord, JobStatus};
pub use transcribe::{AudioBackend, AudioReport, AudioTranscription, HeuristicAudioBackend};
pub use validate::{validate_audio, AudioError, AudioMetadata, MAX_AUDIO_SIZE};

pub mod fixtures {
    pub fn minimal_wav(duration_s: u32) -> Vec<u8> {
        let sample_rate = 16000u32;
        let num_samples = sample_rate * duration_s;
        let data_size = num_samples * 2; // 16-bit mono
        let file_size = 36 + data_size;

        let mut v = Vec::with_capacity(44 + data_size as usize);
        v.extend_from_slice(b"RIFF");
        v.extend_from_slice(&file_size.to_le_bytes());
        v.extend_from_slice(b"WAVE");
        v.extend_from_slice(b"fmt ");
        v.extend_from_slice(&16u32.to_le_bytes()); // subchunk size 16
        v.extend_from_slice(&1u16.to_le_bytes()); // PCM
        v.extend_from_slice(&1u16.to_le_bytes()); // mono
        v.extend_from_slice(&sample_rate.to_le_bytes());
        v.extend_from_slice(&(sample_rate * 2).to_le_bytes()); // byte rate
        v.extend_from_slice(&2u16.to_le_bytes()); // block align
        v.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
        v.extend_from_slice(b"data");
        v.extend_from_slice(&data_size.to_le_bytes());
        v.resize(44 + data_size as usize, 0);
        v
    }

    pub fn minimal_mp3_with_comment(comment: &str) -> Vec<u8> {
        let mut v = Vec::from(&b"ID3\x04\x00\x00"[..]);
        // Size of ID3 tag
        let frame_data = comment.as_bytes();
        let frame_size = 4 + frame_data.len() as u32; // 4 bytes encoding/lang + text
        let tag_size = 10 + frame_size;
        v.extend_from_slice(&[0, 0, 0, tag_size as u8]);

        // COMM frame
        v.extend_from_slice(b"COMM");
        v.extend_from_slice(&frame_size.to_be_bytes());
        v.extend_from_slice(&[0, 0]); // flags
        v.extend_from_slice(&[0, b'e', b'n', b'g']); // encoding 0 + lang eng
        v.extend_from_slice(frame_data);

        // Dummy MP3 audio frame
        v.extend_from_slice(&[0xFF, 0xFB, 0x90, 0x64]);
        v.resize(v.len() + 32, 0);
        v
    }
}
pub use fixtures::*;

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use hre_core::{Action, SecondaryAction};
    use sha2::Digest;

    #[test]
    fn test_validate_audio_wav_and_mp3() {
        let wav = minimal_wav(1);
        assert!(validate_audio(&wav, "audio/wav").is_ok());

        let mp3 = minimal_mp3_with_comment("test");
        assert!(validate_audio(&mp3, "audio/mpeg").is_ok());
    }

    #[test]
    fn test_rejects_audio_mime_mismatch() {
        let wav = minimal_wav(1);
        assert_eq!(
            validate_audio(&wav, "audio/mpeg"),
            Err(AudioError::MimeMismatch {
                declared: "audio/mpeg".into(),
                detected: "audio/wav".into(),
            })
        );
    }

    #[test]
    fn test_rejects_oversized_audio() {
        let oversize = vec![0u8; MAX_AUDIO_SIZE + 1];
        assert!(matches!(
            validate_audio(&oversize, "audio/wav"),
            Err(AudioError::Oversized { .. })
        ));
    }

    #[test]
    fn test_missing_whisper_model_is_degraded() {
        let backend = HeuristicAudioBackend::new();
        let wav = minimal_wav(1);
        let report = backend.process_audio(&wav, "audio/wav").unwrap();
        assert_eq!(report.signal.score, 0.0);
        assert_eq!(report.raw_transcript, "");
        assert!(report.degraded.iter().any(|d| d == "whisper.ggml_missing"));
    }

    #[test]
    fn test_scenario_5_synthetic_threatening_tts_clip() {
        let backend = HeuristicAudioBackend::new();
        let mp3 = minimal_mp3_with_comment("I will kill you and your family tonight");
        let report = backend.process_audio(&mp3, "audio/mpeg").unwrap();
        assert!(report.raw_transcript.contains("kill you"));
        assert!(report.signal.score >= 0.85);
        assert_eq!(report.recommended_action, Action::Escalate);
        assert!(report
            .secondary_actions
            .contains(&SecondaryAction::ShowTranscript));
        assert!(report
            .secondary_actions
            .contains(&SecondaryAction::AlertModerator));
    }

    #[test]
    fn test_tone_signal_can_never_flag_alone() {
        // Voice with aggressive tone (0.95) but completely benign words
        let backend = HeuristicAudioBackend::new().with_fixture(
            "dummy_hash",
            "Good morning, have a wonderful day",
            0.95,
        );
        let wav = minimal_wav(1);
        let mut hasher = sha2::Sha256::new();
        sha2::Digest::update(&mut hasher, &wav);
        let hash = format!("{:x}", sha2::Digest::finalize(hasher));
        let backend = backend.with_fixture(&hash, "Good morning, have a wonderful day", 0.95);

        let report = backend.process_audio(&wav, "audio/wav").unwrap();
        // Since words are benign, tone alone cannot push it into Review or High tier
        assert_eq!(report.signal.score, 0.0);
        assert_eq!(report.recommended_action, Action::Log);
    }

    #[test]
    fn test_job_manager_lifecycle() {
        let manager = JobManager::new();
        let id = manager.create_job();
        let job = manager.get_job(id).unwrap();
        assert_eq!(job.status, JobStatus::Pending);

        manager.fail_job(id, "processing timeout".into());
        let updated = manager.get_job(id).unwrap();
        assert_eq!(updated.status, JobStatus::Failed);
    }
}

pub mod safe_decode;
