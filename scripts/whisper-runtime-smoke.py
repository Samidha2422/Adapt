"""Verify the transferred Whisper models against synthetic speech and silence."""
from pathlib import Path
from test_audio_transcription import SafeAudioDecoder, WhisperTranscriber


def transcribe_fixture(transcriber, filename):
    data = (Path("models") / "_test_fixtures" / filename).read_bytes()
    samples, rate = SafeAudioDecoder.decode_wav_bytes(data)
    return transcriber.transcribe(SafeAudioDecoder.resample_to_16k(samples, rate))


def main():
    transcriber = WhisperTranscriber(Path("models"))
    speech = transcribe_fixture(transcriber, "speech_stop_calling_me.wav")
    if "stop calling" not in speech["transcript"].lower():
        raise RuntimeError("Synthetic speech did not produce the expected phrase")
    silence = transcribe_fixture(transcriber, "silence.wav")
    if silence["transcript"] or not silence["is_silence"]:
        raise RuntimeError("Silent audio did not produce an empty transcript")
    print("Whisper synthetic-speech and silence inference passed")


if __name__ == "__main__":
    main()
