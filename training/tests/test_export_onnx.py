"""Unit and contract tests for ONNX model export and parity verification (Card A2)."""

import os
import subprocess
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory
from unittest.mock import patch

from hre_training.export_onnx import (
    MODEL_SPECS,
    LicenseNotClearedError,
    MissingArtifactError,
    ParityError,
    check_dynamic_batch_shapes,
    compare_numerical_parity,
    export_model_to_onnx,
    get_model_spec,
    verify_license,
)


class TestExportOnnx(unittest.TestCase):
    def test_metadata_label_order(self):
        """Freezes proposed native label ordering; upstream verification is still absent."""
        # 1. unitary/unbiased-toxic-roberta must have the canonical 7 Detoxify labels in exact order
        roberta_spec = get_model_spec("unitary/unbiased-toxic-roberta")
        expected_labels = [
            "toxicity",
            "severe_toxicity",
            "obscene",
            "identity_attack",
            "insult",
            "threat",
            "sexual_explicit",
        ]
        self.assertEqual(roberta_spec["labels"], expected_labels)

        # 2. unitary/multilingual-toxic-xlm-roberta has only 1 binary label
        multi_spec = get_model_spec("unitary/multilingual-toxic-xlm-roberta")
        self.assertEqual(multi_spec["labels"], ["toxic"])

        # 3. textdetox/xlmr-large-toxicity-classifier-v2 has neutral/toxic binary labels
        xlmr_spec = get_model_spec("textdetox/xlmr-large-toxicity-classifier-v2")
        self.assertEqual(xlmr_spec["labels"], ["neutral", "toxic"])

        # 4. cardiffnlp sentiment has 3 classes
        sentiment_spec = get_model_spec("cardiffnlp/twitter-xlm-roberta-base-sentiment")
        self.assertEqual(sentiment_spec["labels"], ["negative", "neutral", "positive"])

    def test_dynamic_batch_shapes(self):
        """Asserts dynamic batch shape metadata for input and output tensors (batch_size at dim 0)."""
        for model_id, spec in MODEL_SPECS.items():
            self.assertTrue(
                check_dynamic_batch_shapes(spec),
                f"Model '{model_id}' dynamic axes do not support dynamic batch_size at axis 0: {spec.get('dynamic_axes')}",
            )
            # Ensure input_ids and attention_mask have dynamic sequence_length at axis 1
            dynamic_axes = spec["dynamic_axes"]
            if "input_ids" in dynamic_axes:
                self.assertEqual(dynamic_axes["input_ids"].get(1), "sequence_length")
            if "attention_mask" in dynamic_axes:
                self.assertEqual(dynamic_axes["attention_mask"].get(1), "sequence_length")

    def test_missing_model_skip_is_explicit(self):
        """Tests that attempting export without local weights produces an explicit MissingArtifactError."""
        # When local weights path does not exist, export raises an explicit MissingArtifactError
        with patch("hre_training.export_onnx.verify_license", return_value=True), self.assertRaises(MissingArtifactError) as ctx:
            export_model_to_onnx(
                "unitary/unbiased-toxic-roberta",
                output_dir="models",
                weights_path="nonexistent/model.bin",
            )
        self.assertIn("not found", str(ctx.exception).lower())

    def test_unverified_license_blocks_export(self):
        """Tests that unverified or unknown licenses strictly block export."""
        # Cardiff sentiment candidate license is marked 'unknown'
        with self.assertRaises(LicenseNotClearedError):
            verify_license("cardiffnlp/twitter-xlm-roberta-base-sentiment")

        with self.assertRaises(LicenseNotClearedError):
            export_model_to_onnx(
                "cardiffnlp/twitter-xlm-roberta-base-sentiment",
                output_dir="models",
            )

    def test_every_candidate_requires_license_evidence(self):
        for model_id in MODEL_SPECS:
            with self.subTest(model_id=model_id), self.assertRaises(LicenseNotClearedError):
                verify_license(model_id)
        with patch.dict(MODEL_SPECS["unitary/unbiased-toxic-roberta"], {"license_cleared": True}):
            with self.assertRaises(LicenseNotClearedError):
                verify_license("unitary/unbiased-toxic-roberta")

    def test_existing_weights_do_not_claim_export(self):
        with TemporaryDirectory() as directory:
            weights = Path(directory) / "weights.bin"
            weights.write_bytes(b"not a model")
            with patch("hre_training.export_onnx.verify_license", return_value=True):
                with self.assertRaises(NotImplementedError):
                    export_model_to_onnx("unitary/unbiased-toxic-roberta", directory, weights)
            self.assertEqual(list(Path(directory).glob("*.onnx")), [])

    def test_invalid_parity_inputs_are_rejected(self):
        for value in (float("nan"), float("inf"), -float("inf")):
            with self.assertRaises(ParityError):
                compare_numerical_parity([value], [0.0])
            with self.assertRaises(ParityError):
                compare_numerical_parity([0.0], [value])
            with self.assertRaises(ParityError):
                compare_numerical_parity([0.0], [0.0], tolerance=value)
        with self.assertRaises(ParityError):
            compare_numerical_parity([], [])
        with self.assertRaises(ParityError):
            compare_numerical_parity([0.0], [0.0], tolerance=-1)

    def test_native_specs_are_not_deployment_contracts(self):
        spec = get_model_spec("unitary/unbiased-toxic-roberta")
        self.assertEqual((spec["task"], spec["output_names"], spec["max_seq_len"]),
                         ("toxicity_multilabel", ["logits"], 512))
        self.assertFalse(check_dynamic_batch_shapes({}))
        spec["dynamic_axes"].pop("logits")
        self.assertFalse(check_dynamic_batch_shapes(spec))
        self.assertIn("logits", get_model_spec("unitary/unbiased-toxic-roberta")["dynamic_axes"])

    def test_numerical_parity_tolerance(self):
        """Verifies parity comparison logic and 1e-4 numerical tolerance bound."""
        ref = [0.12345, 0.67890, 0.00010]
        # Exact match
        ok, diff = compare_numerical_parity(ref, [0.12345, 0.67890, 0.00010], tolerance=1e-4)
        self.assertTrue(ok)
        self.assertEqual(diff, 0.0)

        # Within 1e-4 tolerance
        close = [0.12347, 0.67892, 0.00012]
        ok, diff = compare_numerical_parity(ref, close, tolerance=1e-4)
        self.assertTrue(ok)
        self.assertLessEqual(diff, 1e-4)

        # Exceeds 1e-4 tolerance
        diverged = [0.12345, 0.67950, 0.00010]
        ok, diff = compare_numerical_parity(ref, diverged, tolerance=1e-4)
        self.assertFalse(ok)
        self.assertGreater(diff, 1e-4)

    def test_export_parity_with_fixed_inputs(self):
        """Tests numerical parity with fixed inputs when model weights exist, else skips explicitly."""
        weights_file = Path("models/unbiased-toxic-roberta.onnx")
        if not weights_file.exists():
            raise unittest.SkipTest(
                "Model artifact 'models/unbiased-toxic-roberta.onnx' not present locally. "
                "Offline skip is explicit per Card A2 instructions."
            )
        raise unittest.SkipTest("Runtime/reference parity harness is not implemented; artifact existence is not parity.")

    def test_no_model_artifacts_committed(self):
        """Asserts that no binary model weights are tracked in git repository."""
        root_dir = Path(__file__).resolve().parent.parent.parent
        res = subprocess.run(
            ["git", "ls-files", "*.onnx", "*.bin", "*.safetensors", "*.pt"],
            cwd=root_dir,
            capture_output=True,
            text=True,
            check=True,
        )
        tracked_files = [line.strip() for line in res.stdout.splitlines() if line.strip()]
        self.assertEqual(
            tracked_files,
            [],
            f"Binary model weights must NOT be committed to git: {tracked_files}",
        )


if __name__ == "__main__":
    unittest.main()
