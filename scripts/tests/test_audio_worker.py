"""Offline worker regression tests; no model or third-party package required."""

import importlib.util
import io
import json
from pathlib import Path
import sys
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import MagicMock, Mock, patch
import wave


spec = importlib.util.spec_from_file_location(
    "audio_worker_under_test", Path(__file__).resolve().parents[1] / "test_audio_transcription.py"
)
worker = importlib.util.module_from_spec(spec)
spec.loader.exec_module(worker)


class LocalAssetsTests(unittest.TestCase):
    def test_explicit_directory_supplies_all_assets_without_hub_calls(self):
        with tempfile.TemporaryDirectory(prefix="whisper ' local ") as directory:
            root = Path(directory)
            for name in ("whisper_encoder.onnx", "whisper_decoder.onnx", "whisper_tokenizer.json"):
                (root / name).touch()
            config = {"feature_size": 80, "sampling_rate": 16000}
            (root / "whisper_preprocessor_config.json").write_text(json.dumps(config))
            ort = SimpleNamespace(SessionOptions=Mock(), InferenceSession=Mock())
            tokenizer = Mock()
            extractor = Mock()
            with patch.dict(sys.modules, {
                "onnxruntime": ort,
                "tokenizers": SimpleNamespace(Tokenizer=tokenizer),
                "transformers": SimpleNamespace(WhisperFeatureExtractor=extractor),
            }), patch.object(worker, "MODELS_DIR", Path("missing-default")):
                worker.WhisperTranscriber(root)
            self.assertEqual(
                [Path(call.args[0]) for call in ort.InferenceSession.call_args_list],
                [root.resolve() / "whisper_encoder.onnx", root.resolve() / "whisper_decoder.onnx"],
            )
            tokenizer.from_file.assert_called_once_with(str(root.resolve() / "whisper_tokenizer.json"))
            extractor.assert_called_once_with(**config)

    def test_default_directory_preserves_standalone_entrypoint(self):
        with tempfile.TemporaryDirectory() as directory, patch.object(worker, "MODELS_DIR", Path(directory)):
            with self.assertRaisesRegex(FileNotFoundError, "missing local Whisper asset"):
                worker.WhisperTranscriber()

    def test_missing_explicit_assets_do_not_fall_back_to_default(self):
        with tempfile.TemporaryDirectory() as directory:
            with self.assertRaisesRegex(FileNotFoundError, "missing local Whisper asset"):
                worker.WhisperTranscriber(directory)


class DecoderValidationTests(unittest.TestCase):
    def test_invalid_headers_rejected_without_numpy(self):
        for data in (b"", b"RIFF", b"not a wav" * 8):
            with self.subTest(data=data), self.assertRaises(ValueError):
                worker.SafeAudioDecoder.decode_wav_bytes(data)

    def test_truncated_pcm_rejected_without_numpy(self):
        buffer = io.BytesIO()
        with wave.open(buffer, "wb") as wav:
            wav.setnchannels(2)
            wav.setsampwidth(2)
            wav.setframerate(16000)
            wav.writeframes(b"\x00" * 16)
        with self.assertRaisesRegex(ValueError, "truncated PCM frames"):
            worker.SafeAudioDecoder.decode_wav_bytes(buffer.getvalue()[:-2])

    def test_empty_pcm_rejected_without_numpy(self):
        buffer = io.BytesIO()
        with wave.open(buffer, "wb") as wav:
            wav.setnchannels(1)
            wav.setsampwidth(2)
            wav.setframerate(16000)
            wav.writeframes(b"")
        with self.assertRaisesRegex(ValueError, "empty audio"):
            worker.SafeAudioDecoder.decode_wav_bytes(buffer.getvalue())


class SilenceInferenceTests(unittest.TestCase):
    def test_silence_executes_encoder_and_decoder_before_returning_empty_text(self):
        transcriber = worker.WhisperTranscriber.__new__(worker.WhisperTranscriber)
        transcriber.feature_extractor = Mock(return_value=SimpleNamespace(input_features="mel"))
        transcriber.encoder = Mock()
        transcriber.decoder = Mock()
        transcriber.tokenizer = Mock()
        transcriber.encoder.run.return_value = ["hidden"]
        transcriber.decoder.run.return_value = [MagicMock()]
        numpy = SimpleNamespace(
            sqrt=lambda value: value, mean=lambda value: 0.0,
            array=Mock(), int64=int, argmax=lambda value: worker.TOKEN_END_OF_TRANSCRIPT,
        )
        samples = MagicMock()
        samples.__len__ = Mock(return_value=16000)
        samples.__pow__ = Mock(return_value=0.0)
        with patch.dict(sys.modules, {"numpy": numpy}):
            result = transcriber.transcribe(samples)
        transcriber.encoder.run.assert_called_once_with(None, {"input_features": "mel"})
        transcriber.decoder.run.assert_called_once()
        self.assertTrue(result["is_silence"])
        self.assertEqual(result["transcript"], "")


if __name__ == "__main__":
    unittest.main()
