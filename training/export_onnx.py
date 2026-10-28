#!/usr/bin/env python3
"""CLI entry point for ONNX model export and contract verification."""

import sys
from pathlib import Path

# Ensure src is on sys.path
SRC_DIR = Path(__file__).resolve().parent / "src"
if str(SRC_DIR) not in sys.path:
    sys.path.insert(0, str(SRC_DIR))

from hre_training.export_onnx import main

if __name__ == "__main__":
    sys.exit(main())
