"""Standalone validation suite for genuine ONNX OCR recognition model.

Verifies:
1. Input/output tensor shapes and dtypes match docs/MODEL_CONTRACT.md.
2. Pixel-level recognition without container metadata (PNG tEXt/iTXt, JPEG COM).
3. Preprocessing: aspect-ratio preserving line resize + right-side padding + channel normalization.
4. Postprocessing: CTC greedy decoding (argmax, collapse consecutive repeats, blank removal).
5. Quantitative evaluation: Character Error Rate (CER) and Exact Match Accuracy across
   representative harassment and safety alert phrases.
"""

import io
import json
from pathlib import Path
import sys
from typing import List, Tuple

import numpy as np
import onnx
import onnxruntime as ort
from PIL import Image, ImageDraw, ImageFont, PngImagePlugin

ROOT_DIR = Path(__file__).resolve().parent.parent
MODEL_PATH = ROOT_DIR / "models" / "ocr_recognizer.onnx"
VOCAB_PATH = ROOT_DIR / "models" / "ocr_vocab.json"


def load_vocabulary(vocab_file: Path) -> List[str]:
    with open(vocab_file, "r", encoding="utf-8") as f:
        data = json.load(f)
    tokens = data["tokens"]
    if len(tokens) != 256:
        raise ValueError(f"Expected 256 vocabulary tokens, found {len(tokens)}")
    if tokens[0] != "":
        raise ValueError("Token index 0 must be reserved for CTC blank ('')")
    return tokens


def preprocess_image(
    image: Image.Image,
    target_h: int = 48,
    target_w: int = 320,
    pad_color: Tuple[int, int, int] = (255, 255, 255),
) -> np.ndarray:
    """Resize text line crop preserving aspect ratio into target_h x target_w with right-side padding."""
    image = image.convert("RGB")
    orig_w, orig_h = image.size

    # Scale to height 48 preserving aspect ratio
    scale = target_h / orig_h
    new_w = min(int(round(orig_w * scale)), target_w)
    resized = image.resize((new_w, target_h), Image.Resampling.BILINEAR)

    # Pad on right-hand side up to target_w
    canvas = Image.new("RGB", (target_w, target_h), color=pad_color)
    canvas.paste(resized, (0, 0))

    # RGB scale to [0, 1], normalize with mean/std [0.5, 0.5, 0.5]
    arr = (np.array(canvas, dtype=np.float32) / 255.0 - 0.5) / 0.5
    # HWC -> CHW, add batch dimension -> [1, 3, 48, 320]
    tensor = np.transpose(arr, (2, 0, 1))[np.newaxis, ...]
    return tensor


def ctc_greedy_decode(probs: np.ndarray, vocab: List[str]) -> Tuple[str, float]:
    """CTC greedy decode: argmax each step, collapse consecutive repeats, remove blank (index 0)."""
    # probs: [80, 256]
    indices = np.argmax(probs, axis=-1)
    step_confidences = np.max(probs, axis=-1)

    decoded_tokens = []
    selected_confidences = []
    prev_idx = -1

    for idx, conf in zip(indices, step_confidences):
        if idx != prev_idx:
            if idx != 0 and idx < len(vocab) and vocab[idx]:
                decoded_tokens.append(vocab[idx])
                selected_confidences.append(conf)
            prev_idx = idx

    text = "".join(decoded_tokens).strip()
    mean_conf = float(np.mean(selected_confidences)) if selected_confidences else 0.0
    return text, mean_conf


def compute_cer(reference: str, hypothesis: str) -> float:
    """Character Error Rate = Levenshtein distance / reference character count."""
    r = list(reference)
    h = list(hypothesis)
    if not r:
        return 0.0 if not h else 1.0

    d = [[0] * (len(h) + 1) for _ in range(len(r) + 1)]
    for i in range(len(r) + 1):
        d[i][0] = i
    for j in range(len(h) + 1):
        d[0][j] = j

    for i in range(1, len(r) + 1):
        for j in range(1, len(h) + 1):
            cost = 0 if r[i - 1] == h[j - 1] else 1
            d[i][j] = min(
                d[i - 1][j] + 1,       # deletion
                d[i][j - 1] + 1,       # insertion
                d[i - 1][j - 1] + cost # substitution
            )

    return d[len(r)][len(h)] / len(r)


def create_pixel_only_png(text: str, font: ImageFont.ImageFont) -> bytes:
    """Creates a PNG image with rendered text pixels and strictly NO metadata chunks."""
    dummy = Image.new("RGB", (10, 10))
    d_dummy = ImageDraw.Draw(dummy)
    bbox = d_dummy.textbbox((0, 0), text, font=font)
    text_w = bbox[2] - bbox[0]
    text_h = bbox[3] - bbox[1]

    # Crop tightly around text line
    w = max(text_w + 24, 60)
    h = max(text_h + 12, 32)
    img = Image.new("RGB", (w, h), color=(255, 255, 255))
    draw = ImageDraw.Draw(img)
    draw.text((12, 6), text, font=font, fill=(0, 0, 0))

    # Save to memory buffer strictly without metadata
    buf = io.BytesIO()
    empty_info = PngImagePlugin.PngInfo()
    img.save(buf, format="PNG", pnginfo=empty_info)
    png_bytes = buf.getvalue()

    # Validate that no textual metadata is embedded
    assert b"tEXt" not in png_bytes, "Security check failed: tEXt chunk found in test fixture!"
    assert b"iTXt" not in png_bytes, "Security check failed: iTXt chunk found in test fixture!"
    return png_bytes


def main() -> int:
    print("=" * 70)
    print("  HRE Phase 13 - Genuine ONNX OCR Recognition Model Validation")
    print("=" * 70)

    if not MODEL_PATH.exists():
        print(f"ERROR: Model file not found at {MODEL_PATH}", file=sys.stderr)
        return 1
    if not VOCAB_PATH.exists():
        print(f"ERROR: Vocab file not found at {VOCAB_PATH}", file=sys.stderr)
        return 1

    # 1. Verify model metadata against MODEL_CONTRACT.md
    print("\n[Step 1] Verifying ONNX Graph Signature...")
    m = onnx.load_model(str(MODEL_PATH))
    inputs = {inp.name: [d.dim_value for d in inp.type.tensor_type.shape.dim] for inp in m.graph.input}
    outputs = {out.name: [d.dim_value for d in out.type.tensor_type.shape.dim] for out in m.graph.output}

    print(f"  Inputs:  {inputs}")
    print(f"  Outputs: {outputs}")

    assert "pixel_values" in inputs, "Missing 'pixel_values' input"
    assert inputs["pixel_values"] == [1, 3, 48, 320], f"Bad input shape: {inputs['pixel_values']}"
    assert "token_probabilities" in outputs, "Missing 'token_probabilities' output"
    assert outputs["token_probabilities"] == [1, 80, 256], f"Bad output shape: {outputs['token_probabilities']}"
    print("  => Signature check PASSED: exactly matches [1, 3, 48, 320] -> [1, 80, 256]")

    # 2. Load vocabulary
    print("\n[Step 2] Loading Pinned 256-Entry Vocabulary...")
    vocab = load_vocabulary(VOCAB_PATH)
    active_tokens = [t for t in vocab if t]
    print(f"  Total tokens: {len(vocab)}, Active non-blank tokens: {len(active_tokens)}")
    print(f"  Sample tokens: {vocab[1:11]} ... {vocab[85:97]}")

    # 3. Load ONNX Runtime session
    print("\n[Step 3] Initializing ONNX Runtime Session...")
    opts = ort.SessionOptions()
    opts.graph_optimization_level = ort.GraphOptimizationLevel.ORT_ENABLE_ALL
    session = ort.InferenceSession(str(MODEL_PATH), opts)

    # 4. Run Pixel-Level Recognition Tests
    print("\n[Step 4] Running Pixel Recognition Tests on Pure Images (No Metadata)...")
    try:
        font = ImageFont.truetype("arial.ttf", 26)
    except Exception:
        print("  Warning: Arial font not found, using default PIL font")
        font = ImageFont.load_default()

    test_corpus = [
        "HELP ME",
        "HARASSMENT STOP",
        "CALL 911",
        "I WILL FIND YOU",
        "DOXXING ALERT",
        "STALKING CUE",
        "GET OUT NOW",
        "PLEASE LEAVE ME ALONE",
        "REPORT ABUSE",
        "THREAT DETECTED",
        "URGENT POLICE 100",
        "STOP MESSAGING ME",
    ]

    exact_matches = 0
    total_cer = 0.0

    print(f"  {'Status':<8} | {'Conf':<6} | {'CER':<6} | {'Reference':<22} | {'Recognized Hypothesis'}")
    print("  " + "-" * 68)

    for ref in test_corpus:
        # Generate pure PNG pixels without metadata
        png_bytes = create_pixel_only_png(ref, font)
        img = Image.open(io.BytesIO(png_bytes))

        # Preprocess
        tensor = preprocess_image(img, target_h=48, target_w=320)

        # Infer
        raw_out = session.run(["token_probabilities"], {"pixel_values": tensor})[0]
        hyp, conf = ctc_greedy_decode(raw_out[0], vocab)

        cer = compute_cer(ref, hyp)
        total_cer += cer
        is_exact = (ref == hyp)
        if is_exact:
            exact_matches += 1

        status = "PASS" if is_exact else ("CLOSE" if cer <= 0.15 else "FAIL")
        print(f"  {status:<8} | {conf:<6.4f} | {cer:<6.4f} | {ref:<22} | {hyp}")

    mean_cer = total_cer / len(test_corpus)
    accuracy = (exact_matches / len(test_corpus)) * 100

    print("  " + "-" * 68)
    print(f"  Results: Exact Match = {exact_matches}/{len(test_corpus)} ({accuracy:.1f}%), Mean CER = {mean_cer:.4f}")

    # Assertions
    assert accuracy >= 90.0, f"Accuracy too low: {accuracy:.1f}%"
    assert mean_cer <= 0.05, f"Mean CER too high: {mean_cer:.4f}"
    print("\n[Step 5] All Recognition Tests PASSED Successfully!")
    return 0


if __name__ == "__main__":
    sys.exit(main())
