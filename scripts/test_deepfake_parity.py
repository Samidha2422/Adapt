"""Validate parity between PyTorch Hugging Face model and exported ONNX model."""

import hashlib
from pathlib import Path
import numpy as np
from PIL import Image
import torch
import torch.nn as nn
from transformers import AutoModelForImageClassification
import onnxruntime as ort

MODEL_ID = "dima806/deepfake_vs_real_image_detection"
REVISION = "29e4cf9efc543845610045f6ba7e88e5cf9d9301"
ONNX_PATH = Path("models/deepfake.onnx")

class DeepfakeClassifierWrapper(nn.Module):
    def __init__(self, model: nn.Module):
        super().__init__()
        self.model = model

    def forward(self, pixel_values: torch.Tensor) -> torch.Tensor:
        outputs = self.model(pixel_values=pixel_values)
        logits = outputs.logits
        probabilities = torch.softmax(logits, dim=-1)
        return probabilities

def preprocess_pil_image(img: Image.Image) -> np.ndarray:
    """Preprocess PIL image into NCHW float32 normalized [-1, 1] tensor."""
    img_resized = img.convert("RGB").resize((224, 224), Image.Resampling.BILINEAR)
    arr = np.array(img_resized, dtype=np.float32) / 255.0
    # ViT normalization: (x - 0.5) / 0.5
    arr = (arr - 0.5) / 0.5
    # HWC -> CHW -> NCHW
    arr = np.transpose(arr, (2, 0, 1))
    return np.expand_dims(arr, axis=0)

def run_parity_tests():
    print(f"=== Deepfake Model Parity Test ===")
    assert ONNX_PATH.exists(), f"ONNX model not found at {ONNX_PATH}"

    # Verify ONNX metadata
    session = ort.InferenceSession(str(ONNX_PATH), providers=["CPUExecutionProvider"])
    inputs = session.get_inputs()
    outputs = session.get_outputs()
    print(f"ONNX Inputs: {[inp.name + ' ' + str(inp.shape) + ' ' + inp.type for inp in inputs]}")
    print(f"ONNX Outputs: {[out.name + ' ' + str(out.shape) + ' ' + out.type for out in outputs]}")
    assert inputs[0].name == "pixel_values"
    assert inputs[0].shape == [1, 3, 224, 224]
    assert outputs[0].name == "probabilities"
    assert outputs[0].shape == [1, 2]

    print("Loading PyTorch model for parity comparison...")
    hf_model = AutoModelForImageClassification.from_pretrained(MODEL_ID, revision=REVISION)
    hf_model.eval()
    wrapper = DeepfakeClassifierWrapper(hf_model)
    wrapper.eval()

    test_cases = []

    # 1. Zeros input
    test_cases.append(("Zeros Input", torch.zeros((1, 3, 224, 224), dtype=torch.float32)))

    # 2. Fixed seeded random uniform input in [-1.0, 1.0]
    torch.manual_seed(42)
    rand_input = torch.rand((1, 3, 224, 224), dtype=torch.float32) * 2.0 - 1.0
    test_cases.append(("Seeded Uniform Random [-1, 1]", rand_input))

    # 3. Seeded normal distribution
    torch.manual_seed(1234)
    norm_input = torch.randn((1, 3, 224, 224), dtype=torch.float32).clamp(-2.0, 2.0)
    test_cases.append(("Seeded Normal Distribution", norm_input))

    # 4. Synthetic pattern image (gradient/color test)
    img_data = np.zeros((224, 224, 3), dtype=np.uint8)
    for y in range(224):
        for x in range(224):
            img_data[y, x, 0] = (x * 255) // 224
            img_data[y, x, 1] = (y * 255) // 224
            img_data[y, x, 2] = 128
    synth_img = Image.fromarray(img_data)
    synth_tensor = torch.from_numpy(preprocess_pil_image(synth_img))
    test_cases.append(("Synthetic Color Gradient", synth_tensor))

    # 5. Check if models/synthetic_test.png exists
    sample_png = Path("models/synthetic_test.png")
    if sample_png.exists():
        loaded_img = Image.open(sample_png)
        img_tensor = torch.from_numpy(preprocess_pil_image(loaded_img))
        test_cases.append(("models/synthetic_test.png", img_tensor))

    max_overall_diff = 0.0
    parity_summary = []

    for name, tensor in test_cases:
        with torch.no_grad():
            pt_out = wrapper(tensor).numpy()
        ort_out = session.run(["probabilities"], {"pixel_values": tensor.numpy()})[0]

        abs_diff = np.abs(pt_out - ort_out)
        max_diff = np.max(abs_diff)
        max_overall_diff = max(max_overall_diff, float(max_diff))

        # Check probability properties
        pt_sum = float(np.sum(pt_out))
        ort_sum = float(np.sum(ort_out))

        print(f"\nTest case: '{name}'")
        print(f"  PyTorch output: [authentic={pt_out[0, 0]:.6f}, manipulated={pt_out[0, 1]:.6f}] (sum={pt_sum:.6f})")
        print(f"  ONNX output:    [authentic={ort_out[0, 0]:.6f}, manipulated={ort_out[0, 1]:.6f}] (sum={ort_sum:.6f})")
        print(f"  Max absolute difference: {max_diff:.8e}")

        assert max_diff < 0.01, f"Parity failure for '{name}': max diff {max_diff} >= 0.01"
        parity_summary.append({
            "case": name,
            "pytorch": pt_out[0].tolist(),
            "onnx": ort_out[0].tolist(),
            "max_abs_diff": float(max_diff),
            "passed": bool(max_diff < 0.01),
        })

    print(f"\n==========================================")
    print(f"ALL PARITY TESTS PASSED!")
    print(f"Maximum absolute difference across all test cases: {max_overall_diff:.8e} (threshold: 0.01)")
    print(f"==========================================")
    return max_overall_diff, parity_summary

if __name__ == "__main__":
    run_parity_tests()
