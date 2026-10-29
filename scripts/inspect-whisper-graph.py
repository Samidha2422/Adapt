from pathlib import Path
import onnx

for name in ("whisper_encoder.onnx", "whisper_decoder.onnx"):
    graph = onnx.load_model(str(Path("models") / name), load_external_data=False).graph
    for value in list(graph.input) + list(graph.output):
        tensor = value.type.tensor_type
        dimensions = [d.dim_value if d.HasField("dim_value") else d.dim_param for d in tensor.shape.dim]
        print(name, value.name, tensor.elem_type, dimensions)
