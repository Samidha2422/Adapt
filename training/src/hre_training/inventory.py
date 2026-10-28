"""Dataset provenance inventory and license admission verification.

Card A1 deliverable:
- Tracks candidate training and evaluation datasets without downloading unapproved media.
- Validates declared metadata, not license clearance, language coverage, or sample independence.
- Records partition assignments; content-level leakage checking is not implemented.
"""

from __future__ import annotations

import argparse
import json
import sys
import urllib.parse
from pathlib import Path
from typing import Any, Dict, List, Optional

DEFAULT_DATASETS_FILE = Path(__file__).resolve().parent.parent.parent / "datasets.json"
ALLOWED_RAW_DIR = (Path(__file__).resolve().parents[3] / "data" / "raw").resolve()


class ProvenanceError(Exception):
    """Raised when dataset record fails provenance or license checks."""


class DownloadNotPermittedError(Exception):
    """Raised when download is attempted without explicit opt-in or valid destination."""


def load_inventory(path: Optional[Path | str] = None) -> List[Dict[str, Any]]:
    """Loads dataset provenance inventory from JSON file."""
    target_path = Path(path) if path else DEFAULT_DATASETS_FILE
    if not target_path.exists():
        raise FileNotFoundError(f"Dataset inventory file not found: {target_path}")

    with open(target_path, "r", encoding="utf-8") as f:
        data = json.load(f)

    if not isinstance(data, list):
        raise ProvenanceError("Inventory root must be a list of dataset entries")

    return data


def verify_record(record: Dict[str, Any]) -> None:
    """Validates an individual dataset provenance record."""
    if not isinstance(record, dict):
        raise ProvenanceError("Dataset record must be an object")
    required_keys = {
        "id",
        "name",
        "source_url",
        "license_state",
        "intended_split",
        "language",
        "synthetic",
        "status",
    }
    missing = required_keys - set(record.keys())
    if missing:
        raise ProvenanceError(f"Dataset {record.get('id', 'unknown')} missing required keys: {sorted(missing)}")

    for key in ("id", "name", "source_url", "license_state", "intended_split", "status"):
        if not isinstance(record[key], str) or not record[key].strip():
            raise ProvenanceError(f"Dataset {key} must be a nonempty string")
    if not isinstance(record["synthetic"], bool):
        raise ProvenanceError("Dataset synthetic must be a boolean")

    # Check URL format
    url = record["source_url"]
    parsed_url = urllib.parse.urlparse(url)
    if parsed_url.scheme not in {"http", "https", "internal"} or not parsed_url.netloc:
        raise ProvenanceError(f"Dataset {record['id']} has invalid source URL: {url}")

    # Check license state
    license_state = record["license_state"].strip()
    if not license_state:
        raise ProvenanceError(f"Dataset {record['id']} has empty license state")

    # Check split
    split = record["intended_split"]
    if split not in {"train", "val", "eval_held_out", "eval_test"}:
        raise ProvenanceError(f"Dataset {record['id']} has invalid split: {split}")

    # Check language
    lang = record["language"]
    if not isinstance(lang, list) or not lang or any(not isinstance(x, str) or not x.strip() for x in lang):
        raise ProvenanceError(f"Dataset {record['id']} must define non-empty language list")


def verify_all_records(records: List[Dict[str, Any]]) -> Dict[str, int]:
    """Verifies all records and returns summary counts."""
    counts = {"total": len(records), "train": 0, "eval": 0, "synthetic": 0}
    seen_ids = set()

    for r in records:
        verify_record(r)
        if r["id"] in seen_ids:
            raise ProvenanceError(f"Duplicate dataset id: {r['id']}")
        seen_ids.add(r["id"])

        if r["intended_split"] == "train":
            counts["train"] += 1
        else:
            counts["eval"] += 1

        if r.get("synthetic", False):
            counts["synthetic"] += 1

    return counts


def download_dataset(
    dataset_id: str,
    dest_dir: Path | str = "data/raw",
    opt_in: bool = False,
    inventory: Optional[List[Dict[str, Any]]] = None,
) -> Dict[str, Any]:
    """Attempts to download an admitted dataset.

    Downloads are strictly opt-in and restricted to data/raw/.
    In offline mode or when credentials/network are missing, raises a clear skip.
    """
    if not opt_in:
        raise DownloadNotPermittedError(
            f"Cannot download dataset '{dataset_id}': explicit --opt-in flag is required."
        )

    dest_path = Path(dest_dir).resolve()
    # Ensure destination is within or equal to data/raw
    try:
        dest_path.relative_to(ALLOWED_RAW_DIR)
    except ValueError:
        if dest_path != ALLOWED_RAW_DIR:
            raise DownloadNotPermittedError(
                f"Destination path {dest_path} is outside allowed data directory {ALLOWED_RAW_DIR}"
            )

    records = inventory if inventory is not None else load_inventory()
    verify_all_records(records)
    match = next((r for r in records if r["id"] == dataset_id), None)
    if not match:
        raise KeyError(f"Dataset id not found in inventory: {dataset_id}")

    verify_record(match)
    if match.get("status") != "verified" or not match.get("license_review"):
        raise DownloadNotPermittedError(
            f"Dataset {dataset_id} has no verified admission and license review; download blocked."
        )

    # Offline simulation / stub - downloads are disabled in automated test environments
    return {
        "dataset_id": dataset_id,
        "status": "offline_skipped",
        "message": f"Dataset {dataset_id} requires manual credentialed download to {dest_path}",
    }


def format_inventory_table(records: List[Dict[str, Any]]) -> str:
    """Formats records as a human-readable table."""
    headers = ["ID", "Split", "License", "Synthetic", "Source URL"]
    rows = []
    for r in records:
        rows.append([
            r["id"],
            r["intended_split"],
            r["license_state"][:20],
            "Yes" if r.get("synthetic") else "No",
            r["source_url"][:40],
        ])

    col_widths = [len(h) for h in headers]
    for row in rows:
        for i, val in enumerate(row):
            col_widths[i] = max(col_widths[i], len(val))

    fmt = " | ".join(f"{{:<{w}}}" for w in col_widths)
    sep = "-+-".join("-" * w for w in col_widths)

    lines = [fmt.format(*headers), sep]
    for row in rows:
        lines.append(fmt.format(*row))
    return "\n".join(lines)


def main(argv: Optional[List[str]] = None) -> int:
    parser = argparse.ArgumentParser(description="Dataset Provenance Inventory Manager")
    parser.add_argument("--file", type=str, help="Path to datasets.json", default=None)
    parser.add_argument("--json", action="store_true", help="Emit JSON output")
    parser.add_argument("--verify", action="store_true", help="Validate provenance rules for all entries")
    parser.add_argument("--download", type=str, help="Dataset ID to download", default=None)
    parser.add_argument("--opt-in", action="store_true", help="Explicit confirmation for external dataset downloads")
    parser.add_argument("--dest", type=str, help="Destination directory", default="data/raw")

    args = parser.parse_args(argv)

    try:
        records = load_inventory(args.file)

        if args.verify:
            counts = verify_all_records(records)
            print(f"Validated {counts['total']} metadata records ({counts['train']} train, {counts['eval']} eval, {counts['synthetic']} synthetic); licenses and corpus contents remain unverified.")
            return 0

        if args.download:
            res = download_dataset(args.download, dest_dir=args.dest, opt_in=args.opt_in, inventory=records)
            print(f"Download result: {res['status']} - {res['message']}")
            return 0

        if args.json:
            print(json.dumps(records, indent=2))
        else:
            print(format_inventory_table(records))

        return 0
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
