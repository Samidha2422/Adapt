#!/usr/bin/env python3
"""CLI runner for Synthetic Generator A."""

import argparse
import json
import os
import sys

# Ensure src is on sys.path
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "src"))

from hre_training.features import (
    BehaviorConfig,
    StoredEvent,
    extract_features,
    feature_order,
    feature_vector,
)
from hre_training.generator_a import SyntheticGeneratorA, ALL_SCENARIO_TYPES


def export_golden_vector(fixtures_dir: str):
    """Exports the canonical 47-feature golden vector fixture."""
    events = [
        StoredEvent(
            event_id="00000",
            sender="a",
            recipient="b",
            timestamp_ms=0,
            kind="message",
            channel="dm",
            toxicity=0.1,
            content_hash="a" * 64,
        ),
        StoredEvent(
            event_id="00001",
            sender="a",
            recipient="b",
            timestamp_ms=60_000,
            kind="message",
            channel="dm",
            toxicity=0.2,
            content_hash="a" * 64,
        ),
        StoredEvent(
            event_id="00002",
            sender="b",
            recipient="a",
            timestamp_ms=120_000,
            kind="message",
            channel="dm",
            toxicity=0.0,
            content_hash="a" * 64,
        ),
        StoredEvent(
            event_id="00003",
            sender="a",
            recipient="b",
            timestamp_ms=180_000,
            kind="message",
            channel="dm",
            toxicity=0.3,
            content_hash="a" * 64,
        ),
    ]

    cfg = BehaviorConfig(
        recipient_utc_offset_minutes=0,
        night_start_hour=22,
        night_end_hour=6,
    )

    f_dict = extract_features(events, "a", "b", 180_000, cfg)
    vec = feature_vector(f_dict)
    order = feature_order()

    payload = {
        "version": "1.0.0",
        "dimension": len(vec),
        "feature_order": order,
        "golden_vector": vec,
        "named_vector": {name: val for name, val in zip(order, vec)},
        "provenance": {
            "source": "docs/FEATURES.md",
            "eval_time_ms": 180_000,
            "events_count": len(events),
        },
    }

    out_path = os.path.join(fixtures_dir, "golden_vector.json")
    with open(out_path, "w", encoding="utf-8") as f:
        json.dump(payload, f, indent=2)
    print(f"Exported golden vector fixture ({len(vec)} features) to {out_path}")


def main():
    parser = argparse.ArgumentParser(description="Synthetic Generator A")
    parser.add_argument("--seed", type=int, default=42, help="RNG seed")
    parser.add_argument(
        "--output",
        type=str,
        default=os.path.join(os.path.dirname(__file__), "fixtures", "synthetic_samples.json"),
        help="Output JSON path",
    )
    args = parser.parse_args()

    fixtures_dir = os.path.dirname(args.output)
    os.makedirs(fixtures_dir, exist_ok=True)

    gen = SyntheticGeneratorA(seed=args.seed)
    scenarios = gen.generate_all_scenarios()

    output_data = {
        "generator": "generator_a",
        "seed": args.seed,
        "partition": "training",
        "synthetic": True,
        "scenarios_count": len(scenarios),
        "scenarios": [
            {
                "id": s.scenario_id,
                "type": s.scenario_type,
                "target_id": s.target_id,
                "sender_id": s.sender_id,
                "expected_tier": s.expected_tier,
                "expected_severity_range": s.expected_severity_range,
                "events_count": len(s.events),
                "events": s.events,
                "features": s.features,
                "provenance": s.provenance,
            }
            for s in scenarios
        ],
    }

    with open(args.output, "w", encoding="utf-8") as f:
        json.dump(output_data, f, indent=2)
    print(f"Generated {len(scenarios)} synthetic scenarios to {args.output}")

    export_golden_vector(fixtures_dir)


if __name__ == "__main__":
    main()
