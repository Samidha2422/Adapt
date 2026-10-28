"""Acquire, export, and verify genuine pretrained Whisper transcription models.

Supports:
- ONNX Whisper (openai/whisper-tiny via onnx-community/whisper-tiny) for native
  Windows ONNX Runtime inference without foreign C++ dependencies.
- GGML Whisper (whisper-base.ggml from ggerganov/whisper.cpp) for GGML compatibility.
"""

import hashlib
import json
import os
from pathlib import Path
import shutil
import sys
import time
import urllib.request

MODELS_DIR = Path("models")
MODELS_DIR.mkdir(exist_ok=True)

# Upstream model metadata
WHISPER_ONNX_REPO = "onnx-community/whisper-tiny"
WHISPER_ONNX_REVISION = "ff4177021cc41f7db950912b73ea4fdf7d01d8e7"
WHISPER_UPSTREAM_REPO = "openai/whisper-tiny"
WHISPER_UPSTREAM_REVISION = "169d4a4341b33bc18d8881c4b69c2e104e1cc0af"
WHISPER_LICENSE = "Apache-2.0"

GGML_REPO = "ggerganov/whisper.cpp"
GGML_REVISION = "5359861c739e955e79d9a303bcbc70fb988958b1"
GGML_LICENSE = "MIT"

FILES_TO_DOWNLOAD = [
    {
        "name": "whisper_encoder.onnx",
        "url": f"https://huggingface.co/{WHISPER_ONNX_REPO}/resolve/{WHISPER_ONNX_REVISION}/onnx/encoder_model.onnx",
        "dest": MODELS_DIR / "whisper_encoder.onnx",
        "desc": "Whisper Tiny ONNX Encoder",
    },
    {
        "name": "whisper_decoder.onnx",
        "url": f"https://huggingface.co/{WHISPER_ONNX_REPO}/resolve/{WHISPER_ONNX_REVISION}/onnx/decoder_model.onnx",
        "dest": MODELS_DIR / "whisper_decoder.onnx",
        "desc": "Whisper Tiny ONNX Decoder",
    },
    {
        "name": "whisper_tokenizer.json",
        "url": f"https://huggingface.co/{WHISPER_ONNX_REPO}/resolve/{WHISPER_ONNX_REVISION}/tokenizer.json",
        "dest": MODELS_DIR / "whisper_tokenizer.json",
        "desc": "Whisper Fast Tokenizer",
    },
    {
        "name": "whisper_config.json",
        "url": f"https://huggingface.co/{WHISPER_ONNX_REPO}/resolve/{WHISPER_ONNX_REVISION}/config.json",
        "dest": MODELS_DIR / "whisper_config.json",
        "desc": "Whisper Model Config",
    },
    {
        "name": "whisper_preprocessor_config.json",
        "url": f"https://huggingface.co/{WHISPER_ONNX_REPO}/resolve/{WHISPER_ONNX_REVISION}/preprocessor_config.json",
        "dest": MODELS_DIR / "whisper_preprocessor_config.json",
        "desc": "Whisper Feature Extractor Config",
    },
    {
        "name": "whisper-base.ggml",
        "url": f"https://huggingface.co/{GGML_REPO}/resolve/{GGML_REVISION}/ggml-base.bin",
        "dest": MODELS_DIR / "whisper-base.ggml",
        "desc": "Whisper Base Genuine GGML Model",
    },
]

def sha256_of_file(path: Path) -> str:
    hasher = hashlib.sha256()
    with open(path, "rb") as f:
        while chunk := f.read(1024 * 1024):
            hasher.update(chunk)
    return hasher.hexdigest()

def download_with_progress(url: str, dest: Path, desc: str):
    if dest.exists() and dest.stat().st_size > 1024:
        print(f"[SKIP] {dest.name} already exists ({dest.stat().st_size:,} bytes)")
        return

    tmp_dest = dest.with_suffix(dest.suffix + ".downloading")
    print(f"[DOWNLOADING] {desc} from {url}...")
    req = urllib.request.Request(url, headers={"User-Agent": "Adapt-Phase13-Whisper/1.0"})
    
    t0 = time.time()
    with urllib.request.urlopen(req, timeout=30) as resp, open(tmp_dest, "wb") as f_out:
        total_size = int(resp.headers.get("Content-Length", 0))
        downloaded = 0
        last_log = t0
        while True:
            chunk = resp.read(256 * 1024)
            if not chunk:
                break
            f_out.write(chunk)
            downloaded += len(chunk)
            now = time.time()
            if now - last_log > 3.0:
                percent = (downloaded / total_size * 100) if total_size > 0 else 0
                speed = (downloaded / (now - t0)) / (1024 * 1024)
                print(f"  {dest.name}: {downloaded:,} / {total_size:,} bytes ({percent:.1f}%) @ {speed:.2f} MB/s")
                last_log = now

    elapsed = time.time() - t0
    tmp_dest.replace(dest)
    print(f"[COMPLETED] {dest.name} ({dest.stat().st_size:,} bytes in {elapsed:.2f}s)")

def verify_onnx_model(model_path: Path):
    import onnxruntime as ort
    session = ort.InferenceSession(str(model_path), providers=["CPUExecutionProvider"])
    inputs = [(i.name, i.shape, i.type) for i in session.get_inputs()]
    outputs = [(o.name, o.shape, o.type) for o in session.get_outputs()]
    return {"inputs": inputs, "outputs": outputs}

def main():
    print("=" * 70)
    print("Phase 13 Whisper Model Acquisition and Verification")
    print(f"Upstream Model: {WHISPER_UPSTREAM_REPO} (SHA: {WHISPER_UPSTREAM_REVISION})")
    print(f"ONNX Model:     {WHISPER_ONNX_REPO} (SHA: {WHISPER_ONNX_REVISION})")
    print(f"License:        {WHISPER_LICENSE}")
    print(f"GGML Model:     {GGML_REPO} (SHA: {GGML_REVISION})")
    print(f"License:        {GGML_LICENSE}")
    print("=" * 70)

    # Ensure placeholder backup exists
    orig_ggml = MODELS_DIR / "whisper-base.ggml"
    bak_ggml = MODELS_DIR / "whisper-base.ggml.bak"
    if orig_ggml.exists() and not bak_ggml.exists():
        print(f"Backing up placeholder {orig_ggml} -> {bak_ggml}")
        shutil.copy2(orig_ggml, bak_ggml)

    # Download artifacts
    for item in FILES_TO_DOWNLOAD:
        download_with_progress(item["url"], item["dest"], item["desc"])

    # Digest and summary
    print("\n" + "=" * 70)
    print("Model Artifact Verification Summary")
    print("=" * 70)

    summary = {}
    for item in FILES_TO_DOWNLOAD:
        p = item["dest"]
        size = p.stat().st_size
        digest = sha256_of_file(p)
        print(f"Artifact: {p.name}")
        print(f"  Path:   {p.resolve()}")
        print(f"  Size:   {size:,} bytes ({size / (1024*1024):.2f} MB)")
        print(f"  SHA256: {digest}")
        
        info = {
            "name": p.name,
            "path": str(p),
            "size_bytes": size,
            "sha256": digest,
        }

        if p.suffix == ".onnx":
            onnx_info = verify_onnx_model(p)
            print(f"  ONNX Inputs:  {onnx_info['inputs']}")
            print(f"  ONNX Outputs: {onnx_info['outputs']}")
            info["onnx"] = onnx_info

        summary[p.name] = info
        print()

    # Save summary report
    summary_path = MODELS_DIR / "whisper_artifacts.json"
    with open(summary_path, "w", encoding="utf-8") as f:
        json.dump(summary, f, indent=2)
    print(f"Saved artifact summary to {summary_path}")

if __name__ == "__main__":
    main()
