"""Verify model hashes and declared ONNX graph output metadata."""

import hashlib
import json
from pathlib import Path, PureWindowsPath
import re
import sys


# Keep in sync with the output specifications in docs/MODEL_CONTRACT.md.
TASK_OUTPUTS = {
    "text_fast": {"probabilities": ("float32", [1, 7])},
    "text_accurate": {"probabilities": ("float32", [1, 2])},
    "text_fine_tuned": {"probabilities": ("float32", [1, 11])},
    "sentiment": {"probabilities": ("float32", [1, 3])},
    "embeddings": {"embedding": ("float32", [1, 384])},
    "nsfw": {"probabilities": ("float32", [1, 2])},
    "deepfake": {"probabilities": ("float32", [1, 2])},
    "face": {"boxes": ("float32", [1, 100, 4]), "scores": ("float32", [1, 100]), "valid_count": ("int64", [1])},
    "ocr": {"token_probabilities": ("float32", [1, 80, 256])},
    "tone": {"probabilities": ("float32", [1, 4])},
    "behavior_messages": {"probabilities": ("float32", [1, 2])},
    "behavior_calls": {"probabilities": ("float32", [1, 2])},
}


def fail(message):
    raise ValueError(message)


def model_path(root, value):
    if not isinstance(value, str) or not value or "\\" in value or ":" in value:
        fail("unsafe model path")
    path = Path(value)
    if path.is_absolute() or PureWindowsPath(value).drive or any(p in ("", ".", "..") for p in value.split("/")):
        fail("unsafe model path")
    current = root
    stat = current.lstat()
    if current.is_symlink() or getattr(stat, "st_file_attributes", 0) & 0x400:
        fail("models directory is a symlink or reparse point")
    for part in path.parts:
        current = current / part
        stat = current.lstat()
        if current.is_symlink() or getattr(stat, "st_file_attributes", 0) & 0x400:
            fail("model path contains a symlink or reparse point")
    if not current.is_file():
        fail("model path is not a file")
    return current


def verify(root):
    root = Path(root)
    manifest = json.loads((root / "manifest.json").read_text(encoding="utf-8"))
    models = manifest.get("models") if isinstance(manifest, dict) else None
    if not isinstance(models, list) or not models:
        fail("manifest must contain a nonempty models list")
    tasks = set()
    for model in models:
        if not isinstance(model, dict) or not isinstance(model.get("task"), str) or not model["task"].strip():
            fail("model task is required")
        identifier = model["task"]
        if identifier in tasks:
            fail(f"{identifier}: duplicate task")
        tasks.add(identifier)
    for model in models:
        identifier = model["task"]
        digest = model.get("sha256")
        if not isinstance(digest, str) or not re.fullmatch(r"[0-9a-fA-F]{64}", digest):
            fail(f"{identifier}: valid SHA256 is required")
        path = model_path(root, model.get("path"))
        hasher = hashlib.sha256()
        with path.open("rb") as file:
            for chunk in iter(lambda: file.read(1024 * 1024), b""):
                hasher.update(chunk)
        actual = hasher.hexdigest()
        if actual != digest.lower():
            fail(f"{identifier}: SHA256 mismatch")
        if model.get("backend") != "onnx" or path.suffix.lower() != ".onnx":
            fail(f"{identifier}: output shape verification requires an ONNX model")
        if identifier not in TASK_OUTPUTS:
            fail(f"{identifier}: unknown ONNX task")
        outputs = model.get("outputs")
        if not isinstance(outputs, list) or not outputs:
            fail(f"{identifier}: outputs must be a nonempty list")
        declared = {}
        for output in outputs:
            if not isinstance(output, dict) or set(output) != {"name", "dtype", "shape"}:
                fail(f"{identifier}: invalid output declaration")
            name, dtype, shape = output["name"], output["dtype"], output["shape"]
            if (not isinstance(name, str) or not name or name in declared
                    or not isinstance(dtype, str) or not isinstance(shape, list)
                    or any(type(dim) is not int or dim <= 0 for dim in shape)):
                fail(f"{identifier}: invalid output declaration or dynamic dimension")
            declared[name] = (dtype, shape)
        if declared != TASK_OUTPUTS[identifier]:
            fail(f"{identifier}: declared outputs do not match task contract")
        try:
            import onnx
        except ImportError as exc:
            raise ValueError("ONNX metadata verification requires the Python onnx package") from exc
        graph = onnx.load_model(str(path), load_external_data=False).graph
        actual_outputs = {}
        for output in graph.output:
            if not output.name or output.name in actual_outputs or not output.type.HasField("tensor_type"):
                fail(f"{identifier}: invalid ONNX output metadata")
            tensor = output.type.tensor_type
            if not tensor.HasField("shape") or not tensor.elem_type:
                fail(f"{identifier}: ONNX output type or shape is unavailable")
            dtype = {onnx.TensorProto.FLOAT: "float32", onnx.TensorProto.INT64: "int64"}.get(tensor.elem_type)
            if any(not dim.HasField("dim_value") or dim.dim_value <= 0 for dim in tensor.shape.dim):
                fail(f"{identifier}: ONNX output dimension is dynamic or unavailable")
            shape = [dim.dim_value for dim in tensor.shape.dim]
            actual_outputs[output.name] = (dtype, shape)
        if declared != actual_outputs:
            fail(f"{identifier}: ONNX output metadata does not match task contract")
        print(f"{identifier}: SHA256 and ONNX output metadata verified")


def main():
    try:
        verify(sys.argv[1] if len(sys.argv) > 1 else Path(__file__).resolve().parent.parent / "models")
    except Exception as exc:
        print(f"Model verification failed: {exc}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
