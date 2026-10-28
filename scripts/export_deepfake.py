"""Export pretrained Hugging Face deepfake detection model to ONNX.

Model: dima806/deepfake_vs_real_image_detection
Base architecture: ViT (google/vit-base-patch16-224-in21k)
License: Apache-2.0
Contract:
  - Input: 'pixel_values' [1, 3, 224, 224] float32 NCHW
  - Preprocessing: ViT normalization: (pixel / 255.0 - 0.5) / 0.5
  - Output: 'probabilities' [1, 2] float32 [authentic, manipulated]
"""

import argparse
import hashlib
import os
from pathlib import Path
import sys

# Ensure UTF-8 output on Windows console
if sys.platform == "win32":
    try:
        sys.stdout.reconfigure(encoding="utf-8")
        sys.stderr.reconfigure(encoding="utf-8")
    except Exception:
        pass

import torch
import torch.nn as nn
from transformers import AutoConfig, AutoImageProcessor, AutoModelForImageClassification
import onnx
import onnxruntime as ort

MODEL_ID = "dima806/deepfake_vs_real_image_detection"
REVISION = "29e4cf9efc543845610045f6ba7e88e5cf9d9301"

class DeepfakeClassifierWrapper(nn.Module):
    """Wrapper that applies Softmax over model logits to produce probabilities."""
    def __init__(self, model: nn.Module):
        super().__init__()
        self.model = model

    def forward(self, pixel_values: torch.Tensor) -> torch.Tensor:
        outputs = self.model(pixel_values=pixel_values)
        logits = outputs.logits
        probabilities = torch.softmax(logits, dim=-1)
        return probabilities

def export_model(output_path: Path) -> dict:
    print(f"Loading pretrained model '{MODEL_ID}' (revision {REVISION})...")
    config = AutoConfig.from_pretrained(MODEL_ID, revision=REVISION)
    print(f"Original id2label: {config.id2label}")
    # Verify label mapping: 0 -> Real (authentic), 1 -> Fake (manipulated)
    assert config.id2label.get(0, "").lower() == "real", f"Unexpected label 0: {config.id2label.get(0)}"
    assert config.id2label.get(1, "").lower() == "fake", f"Unexpected label 1: {config.id2label.get(1)}"

    processor = AutoImageProcessor.from_pretrained(MODEL_ID, revision=REVISION)
    print(f"Image processor: size={processor.size}, mean={processor.image_mean}, std={processor.image_std}")

    hf_model = AutoModelForImageClassification.from_pretrained(MODEL_ID, revision=REVISION)
    hf_model.eval()

    wrapper = DeepfakeClassifierWrapper(hf_model)
    wrapper.eval()

    dummy_input = torch.zeros((1, 3, 224, 224), dtype=torch.float32)

    # Verify PyTorch output
    with torch.no_grad():
        pt_prob = wrapper(dummy_input)
    print(f"PyTorch dummy inference probabilities: {pt_prob.numpy().tolist()}")
    assert pt_prob.shape == (1, 2), f"Expected shape (1, 2), got {pt_prob.shape}"

    output_path.parent.mkdir(parents=True, exist_ok=True)
    tmp_path = output_path.with_suffix(".tmp.onnx")

    print(f"Exporting ONNX model to {tmp_path}...")
    torch.onnx.export(
        wrapper,
        dummy_input,
        str(tmp_path),
        export_params=True,
        opset_version=18,
        do_constant_folding=True,
        input_names=["pixel_values"],
        output_names=["probabilities"],
        dynamic_axes=None,  # Static [1, 3, 224, 224] -> [1, 2]
        dynamo=False,
    )

    # Check model integrity
    print("Checking ONNX model integrity...")
    onnx_model = onnx.load(str(tmp_path), load_external_data=False)
    onnx.checker.check_model(onnx_model)

    # Verify external data was NOT created
    external_data_files = list(output_path.parent.glob("*.data"))
    print(f"Existing external data files in models dir: {external_data_files}")
    assert len(external_data_files) == 0, f"Error: Found external .data files: {external_data_files}"

    # Move tmp to target
    if output_path.exists():
        backup_path = output_path.with_suffix(".onnx.bak")
        if not backup_path.exists():
            print(f"Backing up existing {output_path} to {backup_path}...")
            os.replace(output_path, backup_path)
        else:
            output_path.unlink()

    os.replace(tmp_path, output_path)

    # Compute sha256 and size
    size_bytes = output_path.stat().st_size
    hasher = hashlib.sha256()
    with open(output_path, "rb") as f:
        while chunk := f.read(65536):
            hasher.update(chunk)
    sha256_hex = hasher.hexdigest()

    print(f"Export complete:")
    print(f"  Path: {output_path}")
    print(f"  Size: {size_bytes} bytes ({size_bytes / (1024*1024):.2f} MB)")
    print(f"  SHA-256: {sha256_hex}")

    return {
        "model_id": MODEL_ID,
        "revision": REVISION,
        "license": "Apache-2.0",
        "path": str(output_path),
        "size_bytes": size_bytes,
        "sha256": sha256_hex,
        "input_name": "pixel_values",
        "input_shape": [1, 3, 224, 224],
        "input_dtype": "float32",
        "output_name": "probabilities",
        "output_shape": [1, 2],
        "output_dtype": "float32",
        "labels": ["authentic", "manipulated"],
    }

if __name__ == "__main__":
    target = Path("models/deepfake.onnx")
    export_model(target)
