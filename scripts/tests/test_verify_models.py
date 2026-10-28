import hashlib
import importlib.util
import json
from pathlib import Path
import re
import tempfile
import unittest
from unittest import mock


SCRIPT = Path(__file__).resolve().parents[1] / "verify-models.py"
spec = importlib.util.spec_from_file_location("verify_models", SCRIPT)
verifier = importlib.util.module_from_spec(spec)
spec.loader.exec_module(verifier)
try:
    import onnx
    from onnx import TensorProto, helper
except ImportError:
    onnx = None


class VerificationTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.model = {"task": "text_accurate", "backend": "onnx", "path": "tiny.onnx", "outputs": [{"name": "probabilities", "dtype": "float32", "shape": [1, 2]}]}

    def write(self, data=b"not an onnx model"):
        (self.root / "tiny.onnx").write_bytes(data)
        self.model["sha256"] = hashlib.sha256(data).hexdigest()
        self.manifest()

    def manifest(self):
        (self.root / "manifest.json").write_text(json.dumps({"models": [self.model]}), encoding="utf-8")

    def tiny(self, shape, dtype=None, name="probabilities"):
        dtype = dtype or TensorProto.FLOAT
        input_value = helper.make_tensor_value_info("x", dtype, shape)
        output_value = helper.make_tensor_value_info(name, dtype, shape)
        graph = helper.make_graph([helper.make_node("Identity", ["x"], [name])], "tiny", [input_value], [output_value])
        self.write(helper.make_model(graph).SerializeToString())

    def test_hash_before_onnx_parsing(self):
        self.write()
        self.model["sha256"] = "0" * 64
        self.manifest()
        with self.assertRaisesRegex(ValueError, "SHA256 mismatch"):
            verifier.verify(self.root)

    def test_missing_hash_and_unsafe_paths(self):
        self.write()
        del self.model["sha256"]
        self.manifest()
        with self.assertRaisesRegex(ValueError, "SHA256"):
            verifier.verify(self.root)
        for path in ("../tiny.onnx", "C:/tiny.onnx", "sub/../tiny.onnx", "sub\\tiny.onnx"):
            with self.subTest(path=path), self.assertRaisesRegex(ValueError, "unsafe"):
                verifier.model_path(self.root, path)

    def test_non_onnx_fails_after_hash(self):
        self.write()
        self.model["backend"] = "ggml"
        self.manifest()
        with self.assertRaisesRegex(ValueError, "requires an ONNX"):
            verifier.verify(self.root)

    def test_missing_tooling_fails_closed(self):
        self.write()
        with mock.patch.dict("sys.modules", {"onnx": None}):
            with self.assertRaisesRegex(ValueError, "requires the Python onnx package"):
                verifier.verify(self.root)

    def test_contract_examples_match_task_table(self):
        contract = (SCRIPT.parents[1] / "docs" / "MODEL_CONTRACT.md").read_text(encoding="utf-8")
        examples = [json.loads(block) for block in re.findall(r"```json\s*\n(.*?)\n```", contract, re.DOTALL)]
        specs = {item["task"]: {out["name"]: (out["dtype"], out["shape"]) for out in item["outputs"]}
                 for item in examples if item["backend"] == "onnx"}
        self.assertEqual(specs, verifier.TASK_OUTPUTS)

    def test_task_contract_and_schema_rejected_before_tooling(self):
        self.write()
        for field, value in (("task", "unknown"), ("outputs", [{"name": "probabilities", "dtype": "float32", "shape": [1, 7]}]),
                             ("outputs", [{"name": "probabilities", "dtype": "float32", "shape": [None, 2]}]),
                             ("outputs", [{"name": "probabilities", "dtype": "float32", "shape": [-1, 2]}]),
                             ("outputs", [{"name": "probabilities", "dtype": "float32", "shape": [True, 2]}]),
                             ("outputs", [{"name": "probabilities", "dtype": "float32", "shape": [1, 2], "extra": 1}])):
            with self.subTest(field=field, value=value):
                original = self.model[field]
                self.model[field] = value
                self.manifest()
                with self.assertRaises(ValueError):
                    verifier.verify(self.root)
                self.model[field] = original

    def test_duplicate_task_rejected(self):
        self.write()
        (self.root / "manifest.json").write_text(json.dumps({"models": [self.model, self.model]}), encoding="utf-8")
        # Check duplicates even if tooling is unavailable for the first entry.
        with mock.patch.dict("sys.modules", {"onnx": None}):
            with self.assertRaisesRegex(ValueError, "duplicate task"):
                verifier.verify(self.root)

    @unittest.skipIf(onnx is None, "onnx package unavailable")
    def test_real_graph_matches_and_rejects_mismatches(self):
        self.tiny([1, 2])
        verifier.verify(self.root)
        for field, value in (("shape", [1, 3]), ("shape", [2]), ("dtype", "int64"), ("name", "missing")):
            with self.subTest(field=field, value=value):
                original = self.model["outputs"][0][field]
                self.model["outputs"][0][field] = value
                self.manifest()
                with self.assertRaises(ValueError):
                    verifier.verify(self.root)
                self.model["outputs"][0][field] = original

    @unittest.skipIf(onnx is None, "onnx package unavailable")
    def test_dynamic_actual_and_declaration_fail(self):
        self.tiny(["batch", 2])
        with self.assertRaisesRegex(ValueError, "dynamic"):
            verifier.verify(self.root)
        self.model["outputs"][0]["shape"] = [None, 2]
        self.manifest()
        with self.assertRaisesRegex(ValueError, "dynamic"):
            verifier.verify(self.root)

    @unittest.skipIf(onnx is None, "onnx package unavailable")
    def test_graph_type_and_shape_cannot_disagree_with_contract(self):
        for shape, dtype, name in (([1, 7], TensorProto.FLOAT, "probabilities"),
                                   ([1, 2], TensorProto.INT64, "probabilities"),
                                   ([1, 2], TensorProto.FLOAT, "other")):
            with self.subTest(shape=shape, dtype=dtype, name=name):
                self.tiny(shape, dtype, name)
                with self.assertRaisesRegex(ValueError, "ONNX output metadata"):
                    verifier.verify(self.root)

    @unittest.skipIf(onnx is None, "onnx package unavailable")
    def test_face_multi_output_and_int64(self):
        self.model["task"] = "face"
        self.model["outputs"] = [
            {"name": "boxes", "dtype": "float32", "shape": [1, 100, 4]},
            {"name": "scores", "dtype": "float32", "shape": [1, 100]},
            {"name": "valid_count", "dtype": "int64", "shape": [1]},
        ]
        nodes, inputs, outputs = [], [], []
        for i, item in enumerate(self.model["outputs"]):
            dtype = TensorProto.INT64 if item["dtype"] == "int64" else TensorProto.FLOAT
            input_name = f"x{i}"
            inputs.append(helper.make_tensor_value_info(input_name, dtype, item["shape"]))
            outputs.append(helper.make_tensor_value_info(item["name"], dtype, item["shape"]))
            nodes.append(helper.make_node("Identity", [input_name], [item["name"]]))
        graph = helper.make_graph(nodes, "face", inputs, outputs)
        self.write(helper.make_model(graph).SerializeToString())
        verifier.verify(self.root)
        self.model["outputs"].pop()
        self.manifest()
        with self.assertRaisesRegex(ValueError, "task contract"):
            verifier.verify(self.root)


if __name__ == "__main__":
    unittest.main()
