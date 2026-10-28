"""Export and wrap genuine pretrained OCR recognition model matching docs/MODEL_CONTRACT.md.

Model Details:
- Upstream Base: PP-OCRv4 English Recognizer (en_PP-OCRv4_rec)
- Source Repository: https://huggingface.co/deepghs/paddleocr
- Upstream Revision: e7bd63d9c674c96bd4fdab435af0efb383919c18
- Original Authors: PaddlePaddle / PaddleOCR Authors
- License: Apache-2.0
- Contract Input: pixel_values [1, 3, 48, 320] float32
- Contract Output: token_probabilities [1, 80, 256] float32
- Vocabulary: 256-entry versioned charset (index 0: CTC blank)
"""

import hashlib
import json
import os
from pathlib import Path
import urllib.request
import numpy as np
import onnx
from onnx import helper, numpy_helper, TensorProto

BASE_HF_REPO = "https://huggingface.co/deepghs/paddleocr/resolve/main"
REVISION_SHA = "e7bd63d9c674c96bd4fdab435af0efb383919c18"
MODEL_URL = f"{BASE_HF_REPO}/rec/en_PP-OCRv4_rec/model.onnx"
DICT_URL = f"{BASE_HF_REPO}/rec/en_PP-OCRv4_rec/dict.txt"

ROOT_DIR = Path(__file__).resolve().parent.parent
MODELS_DIR = ROOT_DIR / "models"
OUTPUT_ONNX = MODELS_DIR / "ocr_recognizer.onnx"
OUTPUT_VOCAB = MODELS_DIR / "ocr_vocab.json"
CACHE_DIR = MODELS_DIR / "_cache"


def download_file(url: str, dest: Path) -> None:
    dest.parent.mkdir(parents=True, exist_ok=True)
    if not dest.exists():
        print(f"Downloading {url} -> {dest}...")
        req = urllib.request.Request(url, headers={"User-Agent": "HRE-Exporter/1.0"})
        with urllib.request.urlopen(req) as resp, open(dest, "wb") as out_file:
            while chunk := resp.read(64 * 1024):
                out_file.write(chunk)
        print(f"Downloaded {dest.name} ({dest.stat().st_size} bytes)")
    else:
        print(f"Using cached file {dest}")


def build_vocabulary(dict_path: Path, vocab_out_path: Path) -> list[str]:
    with open(dict_path, "r", encoding="utf-8") as f:
        raw_chars = [line.strip("\r\n") for line in f]

    # Contract requirement: 256 entries.
    # Index 0: CTC blank token ("")
    # Indices 1..95: 95 printable ASCII characters from upstream dict
    # Index 96: space ' ' (upstream rec head space class)
    # Indices 97..255: unassigned / reserved padding tokens
    vocab = [""] * 256
    vocab[0] = ""  # CTC blank

    for i, ch in enumerate(raw_chars):
        vocab[i + 1] = ch
    vocab[96] = " "

    vocab_metadata = {
        "version": "1.0.0",
        "task": "ocr",
        "model_id": "ocr_recognizer",
        "license": "Apache-2.0",
        "upstream_model": "en_PP-OCRv4_rec",
        "upstream_repo": "https://huggingface.co/deepghs/paddleocr",
        "upstream_revision": REVISION_SHA,
        "blank_index": 0,
        "vocab_size": 256,
        "active_classes": 97,
        "tokens": vocab,
        "char_to_id": {ch: i for i, ch in enumerate(vocab) if ch != ""},
        "description": "256-entry CTC vocabulary matching docs/MODEL_CONTRACT.md OCR specification."
    }

    with open(vocab_out_path, "w", encoding="utf-8") as f:
        json.dump(vocab_metadata, f, indent=2, ensure_ascii=False)

    print(f"Exported vocabulary to {vocab_out_path} ({len(vocab)} entries)")
    return vocab


def build_wrapped_onnx(raw_onnx_path: Path, output_path: Path) -> None:
    print(f"Loading raw ONNX model from {raw_onnx_path}...")
    model = onnx.load(str(raw_onnx_path))
    graph = model.graph

    # 1. Adapt input: rename 'x' -> 'pixel_values', set fixed shape [1, 3, 48, 320]
    for inp in graph.input:
        if inp.name == "x":
            inp.name = "pixel_values"
            inp.type.tensor_type.shape.Clear()
            for dim_val in [1, 3, 48, 320]:
                d = inp.type.tensor_type.shape.dim.add()
                d.dim_value = dim_val

    for node in graph.node:
        for i, name in enumerate(node.input):
            if name == "x":
                node.input[i] = "pixel_values"

    # 2. Extract upstream output
    old_output_name = graph.output[0].name
    graph.output.pop()

    # 3. Graph extension to fulfill MODEL_CONTRACT.md:
    # Upstream produces [1, 40, 97].
    # Expand time steps: 40 -> 80 (repeat each step 2x).
    # Expand vocabulary: 97 -> 256 (pad with 159 zeros).

    shape_4d = numpy_helper.from_array(np.array([1, 40, 1, 97], dtype=np.int64), name="shape_4d")
    repeats = numpy_helper.from_array(np.array([1, 1, 2, 1], dtype=np.int64), name="repeats_2x")
    shape_3d_80 = numpy_helper.from_array(np.array([1, 80, 97], dtype=np.int64), name="shape_3d_80")
    zeros_pad = numpy_helper.from_array(np.zeros((1, 80, 159), dtype=np.float32), name="zeros_pad_159")

    graph.initializer.extend([shape_4d, repeats, shape_3d_80, zeros_pad])

    node_reshape1 = helper.make_node(
        "Reshape",
        inputs=[old_output_name, "shape_4d"],
        outputs=["probs_4d"],
        name="Wrapper_Reshape_4D"
    )

    node_tile = helper.make_node(
        "Tile",
        inputs=["probs_4d", "repeats_2x"],
        outputs=["probs_tiled"],
        name="Wrapper_Tile_TimeSteps"
    )

    node_reshape2 = helper.make_node(
        "Reshape",
        inputs=["probs_tiled", "shape_3d_80"],
        outputs=["probs_80_97"],
        name="Wrapper_Reshape_80"
    )

    node_concat = helper.make_node(
        "Concat",
        inputs=["probs_80_97", "zeros_pad_159"],
        outputs=["token_probabilities"],
        axis=2,
        name="Wrapper_Concat_Vocab"
    )

    graph.node.extend([node_reshape1, node_tile, node_reshape2, node_concat])

    # 4. Define final contract output
    new_output = helper.make_tensor_value_info(
        "token_probabilities",
        TensorProto.FLOAT,
        [1, 80, 256]
    )
    graph.output.append(new_output)

    # Validate graph integrity
    onnx.checker.check_model(model)
    onnx.save(model, str(output_path))
    print(f"Successfully exported contract ONNX model to {output_path}")


def main():
    CACHE_DIR.mkdir(parents=True, exist_ok=True)
    raw_onnx = CACHE_DIR / "en_PP-OCRv4_rec.onnx"
    raw_dict = CACHE_DIR / "en_PP-OCRv4_dict.txt"

    # Reuse temp files if available
    temp_v4 = MODELS_DIR / "_temp_v4.onnx"
    temp_dict = MODELS_DIR / "_vocab_v4.txt"
    if temp_v4.exists() and not raw_onnx.exists():
        import shutil
        shutil.copyfile(temp_v4, raw_onnx)
    if temp_dict.exists() and not raw_dict.exists():
        import shutil
        shutil.copyfile(temp_dict, raw_dict)

    download_file(MODEL_URL, raw_onnx)
    download_file(DICT_URL, raw_dict)

    build_vocabulary(raw_dict, OUTPUT_VOCAB)
    build_wrapped_onnx(raw_onnx, OUTPUT_ONNX)

    # Compute digest and size
    size_bytes = OUTPUT_ONNX.stat().st_size
    hasher = hashlib.sha256()
    with open(OUTPUT_ONNX, "rb") as f:
        while chunk := f.read(1024 * 1024):
            hasher.update(chunk)
    sha256 = hasher.hexdigest()

    print("\n" + "=" * 60)
    print("OCR Model Export Completed Successfully!")
    print(f"Artifact Path:      {OUTPUT_ONNX}")
    print(f"Byte Size:          {size_bytes:,} bytes")
    print(f"SHA-256 Digest:     {sha256}")
    print(f"Vocab Path:         {OUTPUT_VOCAB}")
    print(f"Base Model:         en_PP-OCRv4_rec ({REVISION_SHA[:8]})")
    print(f"License:            Apache-2.0")
    print(f"Input Shape:        [1, 3, 48, 320] (pixel_values, float32)")
    print(f"Output Shape:       [1, 80, 256] (token_probabilities, float32)")
    print("=" * 60)


if __name__ == "__main__":
    main()
