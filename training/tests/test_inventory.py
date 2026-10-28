"""Unit tests for dataset provenance inventory (Card A1)."""

import json
import socket
import unittest
from pathlib import Path
from unittest.mock import patch

from hre_training.inventory import (
    ALLOWED_RAW_DIR,
    DownloadNotPermittedError,
    ProvenanceError,
    download_dataset,
    load_inventory,
    verify_all_records,
    verify_record,
)


class TestDatasetInventory(unittest.TestCase):
    def setUp(self):
        self.inventory = load_inventory()

    def test_inventory_has_provenance(self):
        """Asserts every row has source URL, valid license state, and split."""
        self.assertGreater(len(self.inventory), 0, "Inventory must contain dataset records")
        for record in self.inventory:
            self.assertIn("source_url", record, f"Missing source_url in {record.get('id')}")
            self.assertTrue(
                record["source_url"].startswith("http://")
                or record["source_url"].startswith("https://")
                or record["source_url"].startswith("internal://"),
                f"Invalid source_url: {record['source_url']}",
            )
            self.assertIn("license_state", record, f"Missing license_state in {record.get('id')}")
            self.assertTrue(bool(record["license_state"].strip()), f"Empty license_state in {record.get('id')}")
            self.assertIn("intended_split", record)
            self.assertIn("language", record)
            self.assertIsInstance(record["language"], list)
            self.assertGreater(len(record["language"]), 0)

    def test_inventory_does_not_download(self):
        """Patches network access to fail and verifies offline inventory operations succeed."""
        def block_network(*args, **kwargs):
            raise socket.error("Network access strictly disabled in offline tests")

        with patch("socket.socket", side_effect=block_network):
            records = load_inventory()
            self.assertGreater(len(records), 0)
            counts = verify_all_records(records)
            self.assertEqual(counts["total"], len(records))

    def test_train_and_eval_split_names_differ(self):
        """Prevents accidental reuse of evaluation partition name in training split."""
        train_datasets = [r for r in self.inventory if r["intended_split"] == "train"]
        eval_datasets = [r for r in self.inventory if r["intended_split"] != "train"]

        self.assertGreater(len(train_datasets), 0, "Must have training datasets")
        self.assertGreater(len(eval_datasets), 0, "Must have evaluation datasets")

        train_splits = {r["intended_split"] for r in train_datasets}
        eval_splits = {r["intended_split"] for r in eval_datasets}

        # The training split name ('train') must not intersect with evaluation partition names
        overlap = train_splits.intersection(eval_splits)
        self.assertEqual(overlap, set(), f"Evaluation split leaked into training: {overlap}")

        # Specifically ensure held-out benchmark is marked as eval
        held_out = next((r for r in self.inventory if r["id"] == "held_out_regional_eval"), None)
        self.assertIsNotNone(held_out)
        self.assertIn(held_out["intended_split"], {"eval_held_out", "eval_test"})

    def test_download_requires_opt_in(self):
        """Verifies dataset download cannot proceed without explicit opt-in."""
        with self.assertRaises(DownloadNotPermittedError):
            download_dataset("jigsaw_toxic_comment", opt_in=False, inventory=self.inventory)

    def test_download_enforces_safe_directory(self):
        """Verifies downloads outside data/raw are strictly rejected."""
        with self.assertRaises(DownloadNotPermittedError):
            download_dataset(
                "jigsaw_toxic_comment",
                dest_dir="C:/Windows/System32",
                opt_in=True,
                inventory=self.inventory,
            )

    def test_all_records_pass_verification(self):
        """Verifies all records in datasets.json pass the provenance schema."""
        counts = verify_all_records(self.inventory)
        self.assertGreaterEqual(counts["total"], 6)
        self.assertGreaterEqual(counts["synthetic"], 1)


if __name__ == "__main__":
    unittest.main()
