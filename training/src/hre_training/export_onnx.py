"""ONNX model export contracts, metadata specifications, and parity verification.

Card A2 deliverable:
- Records unverified native candidate tensor specifications and proposed label orders.
- Enforces license admission checks before export.
- Provides numeric comparison only; upstream/runtime parity is not implemented.
- Ensures offline fallback and explicit skips when model weights are not present.
"""

from __future__ import annotations

import argparse
import json
import copy
import math
import os
import sys
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

# Unverified native candidate specifications; not deployment manifests.
MODEL_SPECS: Dict[str, Dict[str, Any]] = {
    "unitary/unbiased-toxic-roberta": {
        "task": "toxicity_multilabel",
        "publisher": "unitary",
        "license": "Apache-2.0",
        "license_cleared": False,
        "license_url": "https://huggingface.co/unitary/unbiased-toxic-roberta/raw/main/README.md",
        "labels": [
            "toxicity",
            "severe_toxicity",
            "obscene",
            "identity_attack",
            "insult",
            "threat",
            "sexual_explicit",
        ],
        "input_names": ["input_ids", "attention_mask"],
        "output_names": ["logits"],
        "dynamic_axes": {
            "input_ids": {0: "batch_size", 1: "sequence_length"},
            "attention_mask": {0: "batch_size", 1: "sequence_length"},
            "logits": {0: "batch_size"},
        },
        "tolerance": 1e-4,
        "max_seq_len": 512,
    },
    "unitary/multilingual-toxic-xlm-roberta": {
        "task": "multilingual_toxicity_binary",
        "publisher": "unitary",
        "license": "Apache-2.0",
        "license_cleared": False,
        "license_url": "https://huggingface.co/unitary/multilingual-toxic-xlm-roberta/raw/main/README.md",
        "labels": ["toxic"],
        "input_names": ["input_ids", "attention_mask"],
        "output_names": ["logits"],
        "dynamic_axes": {
            "input_ids": {0: "batch_size", 1: "sequence_length"},
            "attention_mask": {0: "batch_size", 1: "sequence_length"},
            "logits": {0: "batch_size"},
        },
        "tolerance": 1e-4,
        "max_seq_len": 512,
    },
    "textdetox/xlmr-large-toxicity-classifier-v2": {
        "task": "binary_toxicity_large",
        "publisher": "textdetox",
        "license": "openrail++",
        "license_cleared": False,
        "license_url": "https://huggingface.co/textdetox/xlmr-large-toxicity-classifier-v2/raw/main/README.md",
        "labels": ["neutral", "toxic"],
        "input_names": ["input_ids", "attention_mask"],
        "output_names": ["logits"],
        "dynamic_axes": {
            "input_ids": {0: "batch_size", 1: "sequence_length"},
            "attention_mask": {0: "batch_size", 1: "sequence_length"},
            "logits": {0: "batch_size"},
        },
        "tolerance": 1e-4,
        "max_seq_len": 512,
    },
    "cardiffnlp/twitter-xlm-roberta-base-sentiment": {
        "task": "sentiment",
        "publisher": "cardiffnlp",
        "license": "unknown",
        "license_cleared": False,
        "license_url": "https://huggingface.co/cardiffnlp/twitter-xlm-roberta-base-sentiment",
        "labels": ["negative", "neutral", "positive"],
        "input_names": ["input_ids", "attention_mask"],
        "output_names": ["logits"],
        "dynamic_axes": {
            "input_ids": {0: "batch_size", 1: "sequence_length"},
            "attention_mask": {0: "batch_size", 1: "sequence_length"},
            "logits": {0: "batch_size"},
        },
        "tolerance": 1e-4,
        "max_seq_len": 256,
    },
    "sentence-transformers/paraphrase-multilingual-MiniLM-L12-v2": {
        "task": "embeddings",
        "publisher": "sentence-transformers",
        "license": "Apache-2.0",
        "license_cleared": False,
        "license_url": "https://huggingface.co/sentence-transformers/paraphrase-multilingual-MiniLM-L12-v2",
        "labels": [],
        "input_names": ["input_ids", "attention_mask"],
        "output_names": ["sentence_embedding"],
        "dynamic_axes": {
            "input_ids": {0: "batch_size", 1: "sequence_length"},
            "attention_mask": {0: "batch_size", 1: "sequence_length"},
            "sentence_embedding": {0: "batch_size"},
        },
        "tolerance": 1e-4,
        "max_seq_len": 128,
    },
}


class LicenseNotClearedError(Exception):
    """Raised when an unverified or restricted license blocks model export."""


class MissingArtifactError(Exception):
    """Raised when model weights or artifacts are not present on local disk."""


class ParityError(Exception):
    """Raised when exported ONNX model outputs diverge from reference by > tolerance."""


def get_model_spec(model_id: str) -> Dict[str, Any]:
    """Retrieves contract specifications for a candidate model."""
    if model_id not in MODEL_SPECS:
        raise KeyError(f"Unknown model candidate: {model_id}. Admitted models: {list(MODEL_SPECS.keys())}")
    return copy.deepcopy(MODEL_SPECS[model_id])


def verify_license(model_id: str) -> bool:
    """Verifies that model license has been reviewed and cleared for export."""
    spec = get_model_spec(model_id)
    # A URL or license identifier alone is not a reviewed, revision-pinned record.
    if not spec.get("license_cleared", False) or not spec.get("license_review"):
        raise LicenseNotClearedError(
            f"Model '{model_id}' license is marked '{spec.get('license')}'. "
            f"Export blocked until official terms are verified and cleared."
        )
    return True


def check_dynamic_batch_shapes(spec: Dict[str, Any]) -> bool:
    """Verifies that all input and output tensors specify dynamic batch_size at axis 0."""
    dynamic_axes = spec.get("dynamic_axes", {})
    tensors = spec.get("input_names", []) + spec.get("output_names", [])
    if not tensors or set(tensors) != set(dynamic_axes):
        return False
    for tensor_name, axes in dynamic_axes.items():
        if 0 not in axes or axes[0] != "batch_size":
            return False
    return True


def export_model_to_onnx(
    model_id: str,
    output_dir: Path | str,
    weights_path: Optional[Path | str] = None,
    force: bool = False,
) -> Dict[str, Any]:
    """Prepares and validates ONNX export for a candidate model.

    Enforces license verification and checks for local model weights.
    Returns export metadata without committing binary weights to git.
    """
    # 1. License clearance gate
    verify_license(model_id)

    # 2. Check for local weights
    if weights_path is None or not Path(weights_path).is_file():
        raise MissingArtifactError(
            f"Model weights for '{model_id}' not found at '{weights_path}'. "
            f"Cannot export without local PyTorch/Safetensors weights. "
            f"Candidate status remains Heuristic / Degraded."
        )

    raise NotImplementedError(
        "ONNX export and deployment adapter are not implemented; no artifact was written."
    )


def compare_numerical_parity(
    reference_outputs: List[float],
    onnx_outputs: List[float],
    tolerance: float = 1e-4,
) -> Tuple[bool, float]:
    """Compares reference PyTorch output against ONNX Runtime output.

    Returns (is_within_tolerance, max_absolute_diff).
    """
    if not math.isfinite(tolerance) or tolerance < 0:
        raise ParityError("Tolerance must be finite and nonnegative")
    if not reference_outputs or len(reference_outputs) != len(onnx_outputs):
        raise ParityError(
            f"Shape mismatch: reference has {len(reference_outputs)} elements, ONNX has {len(onnx_outputs)}"
        )

    max_diff = 0.0
    for r, o in zip(reference_outputs, onnx_outputs):
        if not math.isfinite(r) or not math.isfinite(o):
            raise ParityError("Parity requires finite reference and runtime outputs")
        diff = abs(r - o)
        if diff > max_diff:
            max_diff = diff

    return (max_diff <= tolerance, max_diff)


def format_specs_table() -> str:
    """Formats candidate models as a human-readable table."""
    headers = ["Model ID", "Task", "License", "Cleared", "Labels", "Tolerance"]
    rows = []
    for model_id, spec in MODEL_SPECS.items():
        rows.append([
            model_id[:35],
            spec["task"][:22],
            spec["license"],
            "Yes" if spec["license_cleared"] else "NO (Blocked)",
            str(len(spec["labels"])) if spec["labels"] else "embed(384)",
            f"{spec['tolerance']:.0e}",
        ])

    col_widths = [len(h) for h in headers]
    for row in rows:
        for i, val in enumerate(row):
            col_widths[i] = max(col_widths[i], len(val))

    fmt = " | ".join(f"{{:<{w}}}" for w in col_widths)
    sep = "-+-".join("-" * w for w in col_widths)

    lines = [fmt.format(*headers), sep]
    for row in rows:
        lines.append(fmt.format(*row))
    return "\n".join(lines)


def main(argv: Optional[List[str]] = None) -> int:
    parser = argparse.ArgumentParser(description="ONNX Model Export & Parity Manager")
    parser.add_argument("--list", action="store_true", help="List model candidates and export contracts")
    parser.add_argument("--verify-license", type=str, help="Verify license clearance for candidate model ID")
    parser.add_argument("--export", type=str, help="Model ID to export to ONNX")
    parser.add_argument("--weights", type=str, help="Path to local PyTorch weights", default=None)
    parser.add_argument("--out-dir", type=str, help="Output directory for ONNX model", default="models")

    args = parser.parse_args(argv)

    try:
        if args.verify_license:
            cleared = verify_license(args.verify_license)
            print(f"Model '{args.verify_license}' license is cleared for export: {cleared}")
            return 0

        if args.export:
            res = export_model_to_onnx(args.export, args.out_dir, weights_path=args.weights)
            print(f"Export result: {res}")
            return 0

        print(format_specs_table())
        return 0
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
