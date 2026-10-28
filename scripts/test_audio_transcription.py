"""Standalone verification test suite for genuine Whisper audio transcription.

Validates:
1. Safe WAV audio validation, decoding, and resampling to 16,000 Hz mono [-1.0, 1.0].
2. Acoustic feature extraction (80-channel log-mel spectrogram).
3. ONNX Runtime inference using genuine pretrained Whisper weights (whisper_encoder.onnx + whisper_decoder.onnx).
4. Autoregressive greedy decoding and token-to-text transcription.
5. Silence detection on non-speech waveforms (empty transcript).
6. Robust rejection on invalid, empty, or truncated audio streams.
"""

import math
from pathlib import Path
import subprocess
import sys
import time
import wave
import numpy as np
import scipy.signal
import onnxruntime as ort
from transformers import WhisperFeatureExtractor, WhisperTokenizer

MODELS_DIR = Path("models")
ENCODER_PATH = MODELS_DIR / "whisper_encoder.onnx"
DECODER_PATH = MODELS_DIR / "whisper_decoder.onnx"
TOKENIZER_CONFIG = MODELS_DIR / "whisper_tokenizer.json"
PREPROCESSOR_CONFIG = MODELS_DIR / "whisper_preprocessor_config.json"

# Special Whisper token IDs
TOKEN_START_OF_TRANSCRIPT = 50258
TOKEN_EN = 50259
TOKEN_TRANSCRIBE = 50359
TOKEN_NO_TIMESTAMPS = 50363
TOKEN_END_OF_TRANSCRIPT = 50257

class SafeAudioDecoder:
    """Safe WAV decoding and validation matching hre-audio safe_decode specifications."""
    MAX_BYTES = 25 * 1024 * 1024
    MAX_DURATION_S = 300

    @classmethod
    def decode_wav_bytes(cls, data: bytes) -> tuple[np.ndarray, int]:
        if len(data) == 0:
            raise ValueError("empty audio input")
        if len(data) > cls.MAX_BYTES:
            raise ValueError(f"audio size {len(data)} exceeds maximum allowed {cls.MAX_BYTES}")
        if len(data) < 44 or not data.startswith(b"RIFF") or b"WAVE" not in data[:16]:
            raise ValueError("invalid audio: riff_wave header missing or truncated")

        import io
        try:
            with wave.open(io.BytesIO(data), "rb") as w:
                n_channels = w.getnchannels()
                sample_width = w.getsampwidth()
                rate = w.getframerate()
                n_frames = w.getnframes()

                if sample_width != 2:
                    raise ValueError(f"unsupported sample width: {sample_width * 8}-bit (must be 16-bit PCM)")
                if rate not in (8000, 16000, 22050, 24000, 32000, 44100, 48000):
                    raise ValueError(f"unsupported sample rate: {rate} Hz")

                raw_bytes = w.readframes(n_frames)
        except Exception as e:
            if isinstance(e, ValueError):
                raise
            raise ValueError(f"audio decode failed: {e}")

        # Convert to int16 then normalize to float32 [-1.0, 1.0]
        samples = np.frombuffer(raw_bytes, dtype=np.int16).astype(np.float32) / 32768.0

        # Downmix stereo to mono if needed
        if n_channels == 2:
            samples = samples.reshape(-1, 2).mean(axis=1)
        elif n_channels != 1:
            raise ValueError(f"unsupported channels: {n_channels} (must be mono or stereo)")

        duration_s = len(samples) / rate
        if duration_s > cls.MAX_DURATION_S:
            raise ValueError(f"audio duration {duration_s:.1f}s exceeds limit {cls.MAX_DURATION_S}s")

        return samples, rate

    @classmethod
    def resample_to_16k(cls, samples: np.ndarray, orig_rate: int) -> np.ndarray:
        if orig_rate == 16000:
            return samples
        target_len = int(round(len(samples) * 16000.0 / orig_rate))
        return scipy.signal.resample(samples, target_len).astype(np.float32)

class WhisperTranscriber:
    """End-to-end Whisper ONNX speech recognition pipeline."""
    def __init__(self):
        assert ENCODER_PATH.exists(), f"Encoder not found: {ENCODER_PATH}"
        assert DECODER_PATH.exists(), f"Decoder not found: {DECODER_PATH}"

        # Initialize ONNX sessions
        opts = ort.SessionOptions()
        opts.inter_op_num_threads = 4
        opts.intra_op_num_threads = 4
        self.encoder = ort.InferenceSession(str(ENCODER_PATH), opts, providers=["CPUExecutionProvider"])
        self.decoder = ort.InferenceSession(str(DECODER_PATH), opts, providers=["CPUExecutionProvider"])

        # Feature extractor & tokenizer
        self.feature_extractor = WhisperFeatureExtractor.from_pretrained(str(PREPROCESSOR_CONFIG))
        self.tokenizer = WhisperTokenizer.from_pretrained("openai/whisper-tiny", clean_up_tokenization_spaces=False)

    def transcribe(self, samples_16k: np.ndarray, max_new_tokens: int = 64) -> dict:
        # Check for silence: RMS energy threshold
        rms = np.sqrt(np.mean(samples_16k ** 2)) if len(samples_16k) > 0 else 0.0
        if rms < 0.001 or len(samples_16k) < 1600:  # < 0.1s or silent
            return {
                "transcript": "",
                "language": "en",
                "rms": float(rms),
                "is_silence": True,
                "tokens": [],
            }

        # Extract 80-channel log-mel spectrogram features
        fe_out = self.feature_extractor(samples_16k, sampling_rate=16000, return_tensors="np")
        mel = fe_out.input_features  # [1, 80, 3000]

        # 1. Acoustic Encoder
        t0 = time.time()
        hidden_states = self.encoder.run(None, {"input_features": mel})[0]
        enc_time = time.time() - t0

        # 2. Autoregressive Greedy Decoder
        # Prefix tokens: <|startoftranscript|> <|en|> <|transcribe|> <|notimestamps|>
        prefix = [
            TOKEN_START_OF_TRANSCRIPT,
            TOKEN_EN,
            TOKEN_TRANSCRIBE,
            TOKEN_NO_TIMESTAMPS,
        ]
        tokens = list(prefix)

        t1 = time.time()
        for _ in range(max_new_tokens):
            input_ids = np.array([tokens], dtype=np.int64)
            logits = self.decoder.run(["logits"], {
                "input_ids": input_ids,
                "encoder_hidden_states": hidden_states,
            })[0]

            next_token = int(np.argmax(logits[0, -1, :]))
            if next_token == TOKEN_END_OF_TRANSCRIPT:
                break
            tokens.append(next_token)

        dec_time = time.time() - t1
        generated_tokens = tokens[len(prefix):]
        raw_text = self.tokenizer.decode(generated_tokens)
        clean_text = raw_text.strip()

        return {
            "transcript": clean_text,
            "language": "en",
            "rms": float(rms),
            "is_silence": False,
            "tokens": generated_tokens,
            "enc_time_s": enc_time,
            "dec_time_s": dec_time,
            "tokens_per_s": len(generated_tokens) / dec_time if dec_time > 0 else 0.0,
        }

def synthesize_windows_speech(phrase: str, out_wav: Path):
    """Synthesize clean speech audio using Windows SAPI TTS."""
    ps_cmd = (
        f'Add-Type -AssemblyName System.Speech; '
        f'$s = New-Object System.Speech.Synthesis.SpeechSynthesizer; '
        f'$s.SetOutputToWaveFile("{out_wav.resolve()}"); '
        f'$s.Speak("{phrase}"); '
        f'$s.Dispose()'
    )
    subprocess.run(["powershell", "-NoProfile", "-Command", ps_cmd], check=True, capture_output=True)

def create_silence_wav(duration_s: float, rate: int, out_wav: Path):
    """Create a pristine 16-bit PCM silent WAV."""
    n_samples = int(duration_s * rate)
    with wave.open(str(out_wav), "wb") as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(rate)
        w.writeframes(b"\x00\x00" * n_samples)

def run_tests():
    print("=" * 70)
    print("Phase 13 Audio Transcription (Whisper) Test Suite")
    print("=" * 70)

    transcriber = WhisperTranscriber()
    test_results = []

    # -------------------------------------------------------------
    # Test Case 1: Real Speech - "stop calling me"
    # -------------------------------------------------------------
    print("\n--- Test 1: Real Speech Clip ('stop calling me') ---")
    wav1 = MODELS_DIR / "test_speech_stop_calling.wav"
    synthesize_windows_speech("stop calling me", wav1)
    
    with open(wav1, "rb") as f:
        data1 = f.read()
    samples1, rate1 = SafeAudioDecoder.decode_wav_bytes(data1)
    samples1_16k = SafeAudioDecoder.resample_to_16k(samples1, rate1)
    res1 = transcriber.transcribe(samples1_16k)
    
    print(f"Decoded: rate={rate1}Hz, len={len(samples1)} samples ({len(samples1)/rate1:.2f}s)")
    print(f"Transcribed Text: \"{res1['transcript']}\"")
    print(f"Tokens: {res1['tokens']}")
    print(f"Latency: Enc={res1['enc_time_s']:.3f}s, Dec={res1['dec_time_s']:.3f}s ({res1['tokens_per_s']:.1f} tok/s)")
    
    t1_norm = res1["transcript"].lower().replace(".", "").strip()
    assert "stop calling me" in t1_norm or "stop calling" in t1_norm, f"Unexpected transcript: {res1['transcript']}"
    test_results.append(("Real Speech 1 ('stop calling me')", "PASS", res1["transcript"]))

    # -------------------------------------------------------------
    # Test Case 2: Real Speech - Harassment phrase
    # -------------------------------------------------------------
    print("\n--- Test 2: Harassment Phrase ('leave me alone or I will report you') ---")
    wav2 = MODELS_DIR / "test_speech_harassment.wav"
    phrase2 = "leave me alone or I will report you"
    synthesize_windows_speech(phrase2, wav2)

    with open(wav2, "rb") as f:
        data2 = f.read()
    samples2, rate2 = SafeAudioDecoder.decode_wav_bytes(data2)
    samples2_16k = SafeAudioDecoder.resample_to_16k(samples2, rate2)
    res2 = transcriber.transcribe(samples2_16k)

    print(f"Decoded: rate={rate2}Hz, len={len(samples2)} samples ({len(samples2)/rate2:.2f}s)")
    print(f"Transcribed Text: \"{res2['transcript']}\"")
    print(f"Latency: Enc={res2['enc_time_s']:.3f}s, Dec={res2['dec_time_s']:.3f}s ({res2['tokens_per_s']:.1f} tok/s)")

    t2_norm = res2["transcript"].lower().replace(".", "").strip()
    assert "leave me alone" in t2_norm, f"Unexpected transcript: {res2['transcript']}"
    test_results.append(("Real Speech 2 (Harassment warning)", "PASS", res2["transcript"]))

    # -------------------------------------------------------------
    # Test Case 3: Pure Silence Audio (Zero Waveform)
    # -------------------------------------------------------------
    print("\n--- Test 3: Pure Silence (Zero PCM waveform) ---")
    wav3 = MODELS_DIR / "test_silence.wav"
    create_silence_wav(2.0, 16000, wav3)

    with open(wav3, "rb") as f:
        data3 = f.read()
    samples3, rate3 = SafeAudioDecoder.decode_wav_bytes(data3)
    samples3_16k = SafeAudioDecoder.resample_to_16k(samples3, rate3)
    res3 = transcriber.transcribe(samples3_16k)

    print(f"Silence RMS: {res3['rms']:.6f}")
    print(f"Is Silence:  {res3['is_silence']}")
    print(f"Transcript:  \"{res3['transcript']}\"")
    assert res3["transcript"] == "", f"Expected empty transcript for silence, got '{res3['transcript']}'"
    assert res3["is_silence"] is True
    test_results.append(("Pure Silence Detection", "PASS", "Empty transcript / is_silence=True"))

    # -------------------------------------------------------------
    # Test Case 4: Invalid and Corrupted Audio Inputs
    # -------------------------------------------------------------
    print("\n--- Test 4: Invalid and Truncated Audio Inputs ---")
    # 4a. Empty input
    try:
        SafeAudioDecoder.decode_wav_bytes(b"")
        assert False, "Should reject empty bytes"
    except ValueError as e:
        print(f"  Empty bytes rejection: OK ({e})")

    # 4b. Truncated RIFF header (< 44 bytes)
    try:
        SafeAudioDecoder.decode_wav_bytes(b"RIFF\x10\x00\x00\x00WAVEfmt ")
        assert False, "Should reject truncated header"
    except ValueError as e:
        print(f"  Truncated header rejection: OK ({e})")

    # 4c. Non-audio file header
    try:
        SafeAudioDecoder.decode_wav_bytes(b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR...")
        assert False, "Should reject non-WAV bytes"
    except ValueError as e:
        print(f"  Non-WAV header rejection: OK ({e})")

    test_results.append(("Invalid/Truncated Audio Rejection", "PASS", "All rejected with clean ValueError"))

    # Clean up temporary test audio files
    for p in [wav1, wav2, wav3]:
        if p.exists():
            p.unlink()

    # -------------------------------------------------------------
    # Summary Table
    # -------------------------------------------------------------
    print("\n" + "=" * 70)
    print("Test Suite Summary")
    print("=" * 70)
    for name, status, detail in test_results:
        print(f"[{status}] {name:38s} -> {detail}")
    print("=" * 70)
    print("ALL AUDIO TRANSCRIPTION TESTS PASSED SUCCESSFULLY!")

if __name__ == "__main__":
    run_tests()
