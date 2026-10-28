use serde::{Deserialize, Serialize};

pub const MAX_AUDIO_SIZE: usize = 25 * 1024 * 1024; // 25 MiB

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum AudioError {
    #[error("empty audio input")]
    EmptyInput,
    #[error("audio size {size} bytes exceeds maximum allowed {max} bytes")]
    Oversized { size: usize, max: usize },
    #[error("audio truncated or invalid header")]
    TruncatedAudio,
    #[error("unsupported audio format")]
    UnsupportedFormat,
    #[error("declared MIME '{declared}' does not match detected format '{detected}'")]
    MimeMismatch { declared: String, detected: String },
    #[error("transcription error: {0}")]
    Transcription(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioMetadata {
    pub mime: String,
    pub size_bytes: usize,
}

/// Sniffs magic bytes and validates audio format and byte limits.
pub fn validate_audio(bytes: &[u8], declared_mime: &str) -> Result<AudioMetadata, AudioError> {
    if bytes.is_empty() {
        return Err(AudioError::EmptyInput);
    }
    if bytes.len() > MAX_AUDIO_SIZE {
        return Err(AudioError::Oversized {
            size: bytes.len(),
            max: MAX_AUDIO_SIZE,
        });
    }

    let detected_mime = sniff_audio_mime(bytes)?;
    let normalized = declared_mime.trim().to_lowercase();

    let mime_matches = match detected_mime {
        "audio/wav" => matches!(
            normalized.as_str(),
            "audio/wav" | "audio/x-wav" | "audio/wave"
        ),
        "audio/mpeg" => matches!(normalized.as_str(), "audio/mpeg" | "audio/mp3"),
        "audio/ogg" => matches!(normalized.as_str(), "audio/ogg" | "audio/vorbis"),
        "audio/flac" => matches!(normalized.as_str(), "audio/flac" | "audio/x-flac"),
        _ => normalized == detected_mime,
    };

    if !mime_matches && !normalized.is_empty() {
        return Err(AudioError::MimeMismatch {
            declared: declared_mime.to_string(),
            detected: detected_mime.to_string(),
        });
    }

    Ok(AudioMetadata {
        mime: detected_mime.to_string(),
        size_bytes: bytes.len(),
    })
}

fn sniff_audio_mime(bytes: &[u8]) -> Result<&'static str, AudioError> {
    if bytes.len() < 4 {
        return Err(AudioError::TruncatedAudio);
    }

    // WAV: RIFF....WAVE
    if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WAVE" {
        return Ok("audio/wav");
    }

    // MP3: ID3 or sync header
    if bytes.starts_with(b"ID3") {
        return Ok("audio/mpeg");
    }
    if bytes.len() >= 2 && bytes[0] == 0xFF && (bytes[1] & 0xE0) == 0xE0 {
        return Ok("audio/mpeg");
    }

    // Ogg: OggS
    if bytes.starts_with(b"OggS") {
        return Ok("audio/ogg");
    }

    // FLAC: fLaC
    if bytes.starts_with(b"fLaC") {
        return Ok("audio/flac");
    }

    Err(AudioError::UnsupportedFormat)
}
