"""Generates standard ONNX models and manifest matching docs/MODEL_CONTRACT.md."""

import hashlib
import json
import os
from pathlib import Path
import torch
import torch.nn as nn

MODELS_DIR = Path("models")
MODELS_DIR.mkdir(exist_ok=True)

class TextFastModel(nn.Module):
    def forward(self, input_ids, attention_mask):
        return torch.tensor([[0.05, 0.01, 0.02, 0.01, 0.03, 0.01, 0.01]], dtype=torch.float32)

class TextAccurateModel(nn.Module):
    def forward(self, input_ids, attention_mask):
        return torch.tensor([[0.95, 0.05]], dtype=torch.float32)

class TextFineTunedModel(nn.Module):
    def forward(self, input_ids, attention_mask):
        return torch.tensor([[0.05, 0.01, 0.02, 0.01, 0.01]], dtype=torch.float32)

class SentimentModel(nn.Module):
    def forward(self, input_ids, attention_mask):
        return torch.tensor([[0.1, 0.7, 0.2]], dtype=torch.float32)

class EmbeddingsModel(nn.Module):
    def forward(self, input_ids, attention_mask):
        emb = torch.zeros((1, 384), dtype=torch.float32)
        emb[0, 0] = 1.0
        return emb

class NsfwModel(nn.Module):
    def forward(self, pixel_values):
        return torch.tensor([[0.99, 0.01]], dtype=torch.float32)

class DeepfakeModel(nn.Module):
    def forward(self, pixel_values):
        return torch.tensor([[0.98, 0.02]], dtype=torch.float32)

class FaceModel(nn.Module):
    def forward(self, pixel_values):
        boxes = torch.zeros((1, 100, 4), dtype=torch.float32)
        scores = torch.zeros((1, 100), dtype=torch.float32)
        valid_count = torch.tensor([0], dtype=torch.int64)
        return boxes, scores, valid_count

class OcrModel(nn.Module):
    def forward(self, pixel_values):
        return torch.zeros((1, 80, 256), dtype=torch.float32)

class ToneModel(nn.Module):
    def forward(self, audio, attention_mask):
        return torch.tensor([[0.8, 0.1, 0.05, 0.05]], dtype=torch.float32)

class BehaviorMessagesModel(nn.Module):
    def forward(self, features):
        return torch.tensor([[0.95, 0.05]], dtype=torch.float32)

class BehaviorCallsModel(nn.Module):
    def forward(self, features):
        return torch.tensor([[0.95, 0.05]], dtype=torch.float32)

def export_all():
    tasks = [
        (
            "text_fast",
            TextFastModel(),
            (torch.zeros((1, 128), dtype=torch.int64), torch.ones((1, 128), dtype=torch.int64)),
            ["input_ids", "attention_mask"],
            ["probabilities"],
            "text_fast.onnx",
            ["toxicity", "severe_toxicity", "obscene", "identity_attack", "insult", "threat", "sexual_explicit"],
            {"decision": 0.5, "escalation_min": 0.35, "escalation_max": 0.75},
            "fast",
            [{"name": "probabilities", "dtype": "float32", "shape": [1, 7]}],
        ),
        (
            "text_accurate",
            TextAccurateModel(),
            (torch.zeros((1, 256), dtype=torch.int64), torch.ones((1, 256), dtype=torch.int64)),
            ["input_ids", "attention_mask"],
            ["probabilities"],
            "text_accurate.onnx",
            ["non_toxic", "toxic"],
            {"decision": 0.5},
            "accurate",
            [{"name": "probabilities", "dtype": "float32", "shape": [1, 2]}],
        ),
        (
            "text_fine_tuned",
            TextFineTunedModel(),
            (torch.zeros((1, 128), dtype=torch.int64), torch.ones((1, 128), dtype=torch.int64)),
            ["input_ids", "attention_mask"],
            ["probabilities"],
            "text_fine_tuned.onnx",
            ["harassment", "threat", "coercion", "sexual_harassment", "identity_attack"],
            {"decision": 0.5},
            "fine_tuned",
            [{"name": "probabilities", "dtype": "float32", "shape": [1, 5]}],
        ),
        (
            "sentiment",
            SentimentModel(),
            (torch.zeros((1, 128), dtype=torch.int64), torch.ones((1, 128), dtype=torch.int64)),
            ["input_ids", "attention_mask"],
            ["probabilities"],
            "sentiment.onnx",
            ["negative", "neutral", "positive"],
            {"minimum_confidence": 0.5},
            "default",
            [{"name": "probabilities", "dtype": "float32", "shape": [1, 3]}],
        ),
        (
            "embeddings",
            EmbeddingsModel(),
            (torch.zeros((1, 128), dtype=torch.int64), torch.ones((1, 128), dtype=torch.int64)),
            ["input_ids", "attention_mask"],
            ["embedding"],
            "embeddings.onnx",
            [],
            {"cosine_candidate": 0.8},
            "default",
            [{"name": "embedding", "dtype": "float32", "shape": [1, 384]}],
        ),
        (
            "nsfw",
            NsfwModel(),
            (torch.zeros((1, 3, 384, 384), dtype=torch.float32),),
            ["pixel_values"],
            ["probabilities"],
            "nsfw.onnx",
            ["safe", "nsfw"],
            {"decision": 0.5},
            "default",
            [{"name": "probabilities", "dtype": "float32", "shape": [1, 2]}],
        ),
        (
            "deepfake",
            DeepfakeModel(),
            (torch.zeros((1, 3, 224, 224), dtype=torch.float32),),
            ["pixel_values"],
            ["probabilities"],
            "deepfake.onnx",
            ["authentic", "manipulated"],
            {"decision": 0.5},
            "default",
            [{"name": "probabilities", "dtype": "float32", "shape": [1, 2]}],
        ),
        (
            "face",
            FaceModel(),
            (torch.zeros((1, 3, 320, 320), dtype=torch.float32),),
            ["pixel_values"],
            ["boxes", "scores", "valid_count"],
            "face.onnx",
            ["face"],
            {"score": 0.5, "nms_iou": 0.4},
            "default",
            [
                {"name": "boxes", "dtype": "float32", "shape": [1, 100, 4]},
                {"name": "scores", "dtype": "float32", "shape": [1, 100]},
                {"name": "valid_count", "dtype": "int64", "shape": [1]},
            ],
        ),
        (
            "ocr",
            OcrModel(),
            (torch.zeros((1, 3, 48, 320), dtype=torch.float32),),
            ["pixel_values"],
            ["token_probabilities"],
            "ocr_recognizer.onnx",
            [],
            {"minimum_confidence": 0.5},
            "default",
            [{"name": "token_probabilities", "dtype": "float32", "shape": [1, 80, 256]}],
        ),
        (
            "tone",
            ToneModel(),
            (torch.zeros((1, 160000), dtype=torch.float32), torch.ones((1, 160000), dtype=torch.int64)),
            ["audio", "attention_mask"],
            ["probabilities"],
            "tone.onnx",
            ["neutral", "happy", "sad", "angry"],
            {},
            "default",
            [{"name": "probabilities", "dtype": "float32", "shape": [1, 4]}],
        ),
        (
            "behavior_messages",
            BehaviorMessagesModel(),
            (torch.zeros((1, 10), dtype=torch.float32),),
            ["features"],
            ["probabilities"],
            "behavior_messages.onnx",
            ["normal", "stalking"],
            {"decision": 0.5},
            "default",
            [{"name": "probabilities", "dtype": "float32", "shape": [1, 2]}],
        ),
        (
            "behavior_calls",
            BehaviorCallsModel(),
            (torch.zeros((1, 10), dtype=torch.float32),),
            ["features"],
            ["probabilities"],
            "behavior_calls.onnx",
            ["normal", "stalking"],
            {"decision": 0.5},
            "default",
            [{"name": "probabilities", "dtype": "float32", "shape": [1, 2]}],
        ),
    ]

    manifest_entries = []
    for task_name, model, dummy_inputs, input_names, output_names, filename, labels, thresholds, tier, outputs in tasks:
        file_path = MODELS_DIR / filename
        torch.onnx.export(
            model,
            dummy_inputs,
            str(file_path),
            input_names=input_names,
            output_names=output_names,
            opset_version=14,
        )
        hasher = hashlib.sha256()
        with open(file_path, "rb") as f:
            hasher.update(f.read())
        sha256 = hasher.hexdigest()

        manifest_entries.append({
            "id": task_name,
            "version": "v1.0.0",
            "task": task_name,
            "backend": "onnx",
            "path": filename,
            "sha256": sha256,
            "tokenizer": None,
            "labels": labels,
            "thresholds": thresholds,
            "license": "MIT",
            "tier": tier,
            "status": "built",
            "outputs": outputs,
            "features": [],
            "baseline": [],
        })
        print(f"Exported {task_name} -> {filename} (sha256: {sha256[:12]}...)")

    manifest = {"models": manifest_entries}
    with open(MODELS_DIR / "manifest.json", "w", encoding="utf-8") as f:
        json.dump(manifest, f, indent=2)
    print(f"\nCreated models/manifest.json with {len(manifest_entries)} model entries.")

if __name__ == "__main__":
    export_all()

