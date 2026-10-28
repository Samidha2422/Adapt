use super::*;
use proptest::prelude::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn chunk(id: &[u8; 4], body: &[u8]) -> Vec<u8> {
    let mut bytes = id.to_vec();
    bytes.extend_from_slice(&(body.len() as u32).to_le_bytes());
    bytes.extend_from_slice(body);
    if body.len() % 2 == 1 {
        bytes.push(0);
    }
    bytes
}

fn riff(chunks: &[Vec<u8>]) -> Vec<u8> {
    let mut bytes = b"RIFF".to_vec();
    let len = 4 + chunks.iter().map(Vec::len).sum::<usize>();
    bytes.extend_from_slice(&(len as u32).to_le_bytes());
    bytes.extend_from_slice(b"WAVE");
    for chunk in chunks {
        bytes.extend_from_slice(chunk);
    }
    bytes
}

fn format(rate: u32, channels: u16) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&channels.to_le_bytes());
    bytes.extend_from_slice(&rate.to_le_bytes());
    bytes.extend_from_slice(&(rate * u32::from(channels) * 2).to_le_bytes());
    bytes.extend_from_slice(&(channels * 2).to_le_bytes());
    bytes.extend_from_slice(&16u16.to_le_bytes());
    bytes
}

fn wav(rate: u32, channels: u16, samples: &[i16]) -> Vec<u8> {
    let samples: Vec<_> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
    riff(&[
        chunk(b"fmt ", &format(rate, channels)),
        chunk(b"data", &samples),
    ])
}

#[test]
fn decode_normalizes_pcm16_and_downmixes_stereo_without_changing_rate() -> TestResult {
    let mono = decode_wav(&wav(8000, 1, &[i16::MIN, 0, i16::MAX]), "audio/wav")?;
    assert_eq!(mono.samples(), &[-1.0, 0.0, 32767.0 / 32768.0]);
    assert_eq!(mono.metadata().sample_rate_hz, 8000);
    assert_eq!(mono.metadata().duration_s, 3.0 / 8000.0);
    let stereo = decode_wav(
        &wav(48000, 2, &[16384, -16384, i16::MAX, i16::MAX]),
        "AUDIO/X-WAV",
    )?;
    assert_eq!(stereo.samples(), &[0.0, 32767.0 / 32768.0]);
    assert_eq!(stereo.metadata().frames, 2);
    assert_eq!(stereo.metadata().source_channels, 2);
    assert!(!format!("{mono:?}").contains("samples"));
    Ok(())
}

#[test]
fn unavailable_asr_never_returns_transcript_or_safety_score_even_for_silence() -> TestResult {
    for samples in [&[0i16, 0][..], &[i16::MIN, i16::MAX][..]] {
        let real = analyze_wav(&wav(16000, 1, samples), "audio/wav", false)?;
        assert!(matches!(real.status, Status::Roadmap));
        assert!(matches!(real.decode_status, Status::Built));
        assert!(matches!(real.backend, Backend::Disabled));
        assert_eq!(real.transcript, None);
        assert_eq!(real.score, None);
        assert!(!real.simulated);
        assert_eq!(
            real.degraded,
            ["audio.asr_unavailable", "audio.tone_unavailable"]
        );
        let simulated = analyze_wav(&wav(16000, 1, samples), "audio/wav", true)?;
        assert!(simulated.simulated);
        assert!(matches!(simulated.decode_status, Status::Simulated));
        assert!(matches!(simulated.status, Status::Roadmap));
        assert_eq!(simulated.transcript, None);
    }
    assert!(analyze_wav(b"invalid", "audio/wav", false).is_err());
    Ok(())
}

#[test]
fn decode_accepts_all_documented_rates_and_empty_pcm_extension() -> TestResult {
    for rate in [8000, 16000, 22050, 24000, 32000, 44100, 48000] {
        let decoded = decode_wav(&wav(rate, 1, &[0]), "audio/wave")?;
        assert_eq!(decoded.metadata().sample_rate_hz, rate);
    }
    let mut fmt = format(16000, 1);
    fmt.extend_from_slice(&0u16.to_le_bytes());
    assert!(decode_wav(
        &riff(&[chunk(b"fmt ", &fmt), chunk(b"data", &[0, 0])]),
        "audio/wav"
    )
    .is_ok());
    Ok(())
}

#[test]
fn decode_checks_mime_magic_and_entire_riff_length() {
    let good = wav(16000, 1, &[0]);
    for mime in [
        "audio/mpeg",
        "audio/wav; codecs=1",
        "video/wav",
        "",
        "audio/ogg",
    ] {
        assert!(decode_wav(&good, mime).is_err());
    }
    for cut in 0..good.len() {
        assert!(decode_wav(&good[..cut], "audio/wav").is_err());
    }
    let mut trailing = good.clone();
    trailing.push(0);
    assert!(decode_wav(&trailing, "audio/wav").is_err());
    for magic in [b"RF64", b"RIFX", b"OggS"] {
        let mut invalid = good.clone();
        invalid[..4].copy_from_slice(magic);
        assert!(decode_wav(&invalid, "audio/wav").is_err());
    }
}

#[test]
fn decode_rejects_compressed_float_extensible_and_inconsistent_pcm() {
    let good = format(16000, 1);
    for (offset, replacement) in [
        (0, 3u16),
        (0, 6),
        (0, 0xfffe),
        (2, 0),
        (2, 3),
        (12, 0),
        (14, 8),
        (14, 32),
    ] {
        let mut invalid = good.clone();
        invalid[offset..offset + 2].copy_from_slice(&replacement.to_le_bytes());
        assert!(decode_wav(
            &riff(&[chunk(b"fmt ", &invalid), chunk(b"data", &[0, 0])]),
            "audio/wav"
        )
        .is_err());
    }
    for offset in [4, 8] {
        let mut invalid = good.clone();
        invalid[offset..offset + 4].copy_from_slice(&0u32.to_le_bytes());
        assert!(decode_wav(
            &riff(&[chunk(b"fmt ", &invalid), chunk(b"data", &[0, 0])]),
            "audio/wav"
        )
        .is_err());
    }
    assert!(decode_wav(&wav(96000, 1, &[0]), "audio/wav").is_err());
}

#[test]
fn decode_rejects_missing_duplicate_out_of_order_empty_or_partial_chunks() {
    let fmt = chunk(b"fmt ", &format(16000, 2));
    let data = chunk(b"data", &[0, 0, 0, 0]);
    for chunks in [
        vec![],
        vec![fmt.clone()],
        vec![data.clone()],
        vec![data.clone(), fmt.clone()],
        vec![fmt.clone(), fmt.clone(), data.clone()],
        vec![fmt.clone(), data.clone(), data],
        vec![fmt.clone(), chunk(b"data", &[])],
        vec![fmt, chunk(b"data", &[0, 0])],
    ] {
        assert!(decode_wav(&riff(&chunks), "audio/wav").is_err());
    }
}

#[test]
fn decode_skips_metadata_but_validates_chunk_sizes_and_odd_padding() -> TestResult {
    let good = riff(&[
        chunk(b"LIST", b"private-metadata"),
        chunk(b"fmt ", &format(16000, 1)),
        chunk(b"data", &[0, 0]),
        chunk(b"JUNK", &[1]),
    ]);
    let report = analyze_wav(&good, "audio/wav", false)?;
    assert!(!format!("{report:?}").contains("private-metadata"));
    let mut missing_pad = good.clone();
    missing_pad.pop();
    let len = (missing_pad.len() - 8) as u32;
    missing_pad[4..8].copy_from_slice(&len.to_le_bytes());
    assert!(decode_wav(&missing_pad, "audio/wav").is_err());
    let mut huge_chunk = good;
    huge_chunk[16..20].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(decode_wav(&huge_chunk, "audio/wav").is_err());
    Ok(())
}

#[test]
fn decode_enforces_encoded_duration_and_chunk_limits() -> TestResult {
    assert!(matches!(
        decode_wav(&vec![0; MAX_AUDIO_BYTES + 1], "audio/wav"),
        Err(AudioError::Limit("encoded_bytes"))
    ));
    let samples = vec![0; (8000 * MAX_DURATION_S) as usize];
    assert_eq!(
        decode_wav(&wav(8000, 1, &samples), "audio/wav")?
            .metadata()
            .duration_s,
        300.0
    );
    let oversized = vec![0; (8000 * MAX_DURATION_S + 1) as usize];
    assert!(matches!(
        decode_wav(&wav(8000, 1, &oversized), "audio/wav"),
        Err(AudioError::Limit("duration"))
    ));
    let mut chunks = vec![chunk(b"fmt ", &format(16000, 1)), chunk(b"data", &[0, 0])];
    chunks.extend((0..1023).map(|_| chunk(b"JUNK", &[])));
    assert!(matches!(
        decode_wav(&riff(&chunks), "audio/wav"),
        Err(AudioError::Limit("chunks"))
    ));
    Ok(())
}

proptest! {
    #[test]
    fn arbitrary_wav_bytes_never_panic(bytes in proptest::collection::vec(any::<u8>(), 0..8192)) {
        let _ = decode_wav(&bytes, "audio/wav");
    }

    #[test]
    fn pcm16_decoding_preserves_every_generated_sample(samples in proptest::collection::vec(any::<i16>(), 1..1024)) {
        let result = decode_wav(&wav(16000, 1, &samples), "audio/wav");
        prop_assert!(result.is_ok());
        if let Ok(decoded) = result {
            prop_assert_eq!(decoded.samples().len(), samples.len());
            for (decoded, original) in decoded.samples().iter().zip(samples) {
                prop_assert_eq!(*decoded, f32::from(original) / 32768.0);
            }
        }
    }
}
