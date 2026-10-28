//! Transient, bounded PCM16 WAV decoding with an explicit unavailable ASR result.
//! No subprocesses, file access, models, transcripts, tone scores or persistence.

use hre_core::{Backend, Status};
use serde::Serialize;
use std::fmt;
use thiserror::Error;

/// Maximum encoded upload size (25 MiB, including WAV chunks).
pub const MAX_AUDIO_BYTES: usize = 25 * 1024 * 1024;
/// Maximum decoded duration, in seconds (five minutes).
pub const MAX_DURATION_S: u32 = 300;

/// Safe boundary errors; no audio bytes, paths or caller-supplied strings.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum AudioError {
    /// Fixed resource bound exceeded.
    #[error("audio exceeds {0}")]
    Limit(&'static str),
    /// Container or PCM metadata is malformed.
    #[error("invalid audio: {0}")]
    Invalid(&'static str),
    /// Recognized but unsupported MIME/encoding/channel/rate.
    #[error("unsupported audio: {0}")]
    Unsupported(&'static str),
}

/// Safe metadata only; samples are never serialized.
#[derive(Debug, Clone, Copy, Serialize, PartialEq)]
pub struct AudioMetadata {
    /// Native sample rate. No resampling is performed.
    pub sample_rate_hz: u32,
    /// Original number of channels (1 or 2); output is always mono.
    pub source_channels: u16,
    /// Mono sample count after averaging channels.
    pub frames: u32,
    /// Exact frame-count duration in seconds.
    pub duration_s: f64,
}

/// Validated mono float PCM at its native rate. Debug prints metadata only.
/// The owner must drop samples after analysis; this type provides no persistence.
pub struct DecodedAudio {
    metadata: AudioMetadata,
    samples: Vec<f32>,
}

impl fmt::Debug for DecodedAudio {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DecodedAudio")
            .field("metadata", &self.metadata)
            .finish_non_exhaustive()
    }
}

impl DecodedAudio {
    /// Native-rate, finite mono samples in [-1, 1); never log or persist these.
    pub fn samples(&self) -> &[f32] {
        &self.samples
    }
    /// Sample rate, source channels and duration; contains no media body.
    pub fn metadata(&self) -> AudioMetadata {
        self.metadata
    }
}

/// Validate RIFF/WAVE and decode little-endian, signed 16-bit integer PCM.
///
/// Accepts MIME `audio/wav`, `audio/x-wav`, `audio/wave` (case insensitive), no
/// parameters; checks magic independently. Supports mono/stereo and native rates
/// 8,000/16,000/22,050/24,000/32,000/44,100/48,000 Hz. Stereo is averaged.
/// Rejects RF64/RIFX, compressed/float/extensible WAV, empty/partial frames,
/// inconsistent byte rates/alignment, duplicate fmt/data, data before fmt,
/// truncated chunks/padding, trailing bytes and >1,024 chunks. Unknown chunks
/// are validated then skipped; embedded metadata is never returned.
/// Caps: 25 MiB encoded, 300 seconds decoded; no truncation or resampling.
/// In async handlers run in spawn_blocking after enforcing upload limits.
pub fn decode_wav(input: &[u8], mime: &str) -> Result<DecodedAudio, AudioError> {
    if input.len() > MAX_AUDIO_BYTES {
        return Err(AudioError::Limit("encoded_bytes"));
    }
    if !["audio/wav", "audio/x-wav", "audio/wave"]
        .iter()
        .any(|m| mime.eq_ignore_ascii_case(m))
    {
        return Err(AudioError::Unsupported("mime"));
    }
    if input.len() < 12 || &input[..4] != b"RIFF" || &input[8..12] != b"WAVE" {
        return Err(AudioError::Invalid("riff_wave"));
    }
    if u64::from(le32(input, 4)?) + 8 != input.len() as u64 {
        return Err(AudioError::Invalid("riff_size"));
    }
    let mut position = 12usize;
    let mut format = None;
    let mut data = None;
    let mut chunks = 0;
    while position < input.len() {
        chunks += 1;
        if chunks > 1024 {
            return Err(AudioError::Limit("chunks"));
        }
        let header = input
            .get(position..position + 8)
            .ok_or(AudioError::Invalid("chunk_header"))?;
        let size = le32(header, 4)? as usize;
        let start = position + 8;
        let end = start
            .checked_add(size)
            .ok_or(AudioError::Invalid("chunk_size"))?;
        let chunk = input
            .get(start..end)
            .ok_or(AudioError::Invalid("chunk_size"))?;
        match &header[..4] {
            b"fmt " => {
                if format.is_some() {
                    return Err(AudioError::Invalid("duplicate_format"));
                }
                // PCM commonly uses either a 16-byte fmt or an 18-byte fmt with cbSize=0.
                if chunk.len() != 16 && !(chunk.len() == 18 && le16(chunk, 16)? == 0) {
                    return Err(AudioError::Unsupported("format_extension"));
                }
                if le16(chunk, 0)? != 1 || le16(chunk, 14)? != 16 {
                    return Err(AudioError::Unsupported("pcm16_required"));
                }
                let channels = le16(chunk, 2)?;
                let rate = le32(chunk, 4)?;
                if channels != 1 && channels != 2 {
                    return Err(AudioError::Unsupported("channels"));
                }
                if ![8000, 16000, 22050, 24000, 32000, 44100, 48000].contains(&rate) {
                    return Err(AudioError::Unsupported("sample_rate"));
                }
                let alignment = channels * 2;
                if le16(chunk, 12)? != alignment || le32(chunk, 8)? != rate * u32::from(alignment) {
                    return Err(AudioError::Invalid("pcm_alignment_or_byte_rate"));
                }
                format = Some((channels, rate, alignment));
            }
            b"data" => {
                if format.is_none() || data.is_some() {
                    return Err(AudioError::Invalid("data_order_or_duplicate"));
                }
                data = Some(chunk);
            }
            _ => {}
        }
        position = end
            .checked_add(size % 2)
            .ok_or(AudioError::Invalid("chunk_padding"))?;
        if position > input.len() {
            return Err(AudioError::Invalid("chunk_padding"));
        }
    }
    let (channels, rate, alignment) = format.ok_or(AudioError::Invalid("missing_format"))?;
    let data = data.ok_or(AudioError::Invalid("missing_data"))?;
    if data.is_empty() || data.len() % usize::from(alignment) != 0 {
        return Err(AudioError::Invalid("empty_or_partial_frames"));
    }
    let frames = data.len() / usize::from(alignment);
    if frames as u64 > u64::from(rate) * u64::from(MAX_DURATION_S) {
        return Err(AudioError::Limit("duration"));
    }
    let samples = data
        .chunks_exact(usize::from(alignment))
        .map(|frame| {
            frame
                .chunks_exact(2)
                .map(|sample| f32::from(i16::from_le_bytes([sample[0], sample[1]])) / 32768.0)
                .sum::<f32>()
                / f32::from(channels)
        })
        .collect();
    Ok(DecodedAudio {
        metadata: AudioMetadata {
            sample_rate_hz: rate,
            source_channels: channels,
            frames: frames as u32,
            duration_s: frames as f64 / f64::from(rate),
        },
        samples,
    })
}

fn le16(bytes: &[u8], position: usize) -> Result<u16, AudioError> {
    let value = bytes
        .get(position..position + 2)
        .ok_or(AudioError::Invalid("truncated_integer"))?;
    Ok(u16::from_le_bytes([value[0], value[1]]))
}
fn le32(bytes: &[u8], position: usize) -> Result<u32, AudioError> {
    let value = bytes
        .get(position..position + 4)
        .ok_or(AudioError::Invalid("truncated_integer"))?;
    Ok(u32::from_le_bytes([value[0], value[1], value[2], value[3]]))
}

/// Decode outcome with unavailable ASR/tone. Null transcript/score means unknown,
/// never clean/safe. Model absence is not simulated inference.
#[derive(Debug, Clone, Serialize)]
pub struct AudioReport {
    /// Metadata only; raw samples are omitted.
    pub metadata: AudioMetadata,
    /// Built/Simulated describes successful decoding only.
    pub decode_status: Status,
    /// ASR status is always Roadmap for this implementation.
    pub status: Status,
    /// Always Disabled: no ASR runtime is invoked.
    pub backend: Backend,
    /// Always None; no transcript is fabricated, including for silent input.
    pub transcript: Option<String>,
    /// Always None; absence of ASR cannot establish low risk.
    pub score: Option<f64>,
    /// Input provenance supplied by the adapter, propagated even when ASR is absent.
    pub simulated: bool,
    /// Stable unavailable reason codes, including missing ASR and tone.
    pub degraded: Vec<String>,
}

/// Deterministic fallback for already validated PCM. Never examines samples to
/// invent speech, language, emotion, or a safety decision. No models are loaded.
pub fn unavailable_asr(audio: &DecodedAudio, simulated: bool) -> AudioReport {
    AudioReport {
        metadata: audio.metadata(),
        decode_status: if simulated {
            Status::Simulated
        } else {
            Status::Built
        },
        status: Status::Roadmap,
        backend: Backend::Disabled,
        transcript: None,
        score: None,
        simulated,
        degraded: vec![
            "audio.asr_unavailable".into(),
            "audio.tone_unavailable".into(),
        ],
    }
}

/// Validate/decode a WAV, then return metadata plus honest unavailable ASR/tone.
/// Invalid uploads return an error, never a successful unavailable analysis.
pub fn analyze_wav(input: &[u8], mime: &str, simulated: bool) -> Result<AudioReport, AudioError> {
    Ok(unavailable_asr(&decode_wav(input, mime)?, simulated))
}

#[cfg(test)]
mod tests;
