"""Tests for Synthetic Generator A and behavioral feature parity."""

import unittest
from hre_training.generator_a import SyntheticGeneratorA, ALL_SCENARIO_TYPES
from hre_training.features import (
    StoredEvent,
    BehaviorConfig,
    extract_features,
    feature_vector,
    feature_order,
)


class TestSyntheticGeneratorA(unittest.TestCase):

    def test_same_seed_same_events(self):
        gen1 = SyntheticGeneratorA(seed=1337)
        scen1 = gen1.generate_scenario("polite_stalker")

        gen2 = SyntheticGeneratorA(seed=1337)
        scen2 = gen2.generate_scenario("polite_stalker")

        self.assertEqual(len(scen1.events), len(scen2.events))
        self.assertEqual(scen1.features, scen2.features)
        self.assertEqual(scen1.events, scen2.events)

    def test_seed_changes_sample(self):
        gen1 = SyntheticGeneratorA(seed=111)
        scen1 = gen1.generate_scenario("polite_stalker")

        gen2 = SyntheticGeneratorA(seed=222)
        scen2 = gen2.generate_scenario("polite_stalker")

        # Distinct seeds must produce different sample values
        self.assertNotEqual(scen1.target_id, scen2.target_id)
        self.assertNotEqual(scen1.events[0]["timestamp_ms"], scen2.events[0]["timestamp_ms"])

    def test_chronology_and_matching_payloads(self):
        gen = SyntheticGeneratorA(seed=42)
        for scen_type in ALL_SCENARIO_TYPES:
            scen = gen.generate_scenario(scen_type)
            events = scen.events
            self.assertTrue(len(events) > 0, f"Scenario {scen_type} has empty events")

            # Assert non-decreasing timestamps
            for i in range(len(events) - 1):
                self.assertLessEqual(
                    events[i]["timestamp_ms"],
                    events[i + 1]["timestamp_ms"],
                    f"Timestamp out of order in {scen_type}",
                )

            # Assert payload consistency
            for e in events:
                self.assertIn(e["kind"], ["message", "call", "refusal", "image", "audio"])
                if e["kind"] == "call":
                    self.assertIn("duration_s", e)
                    self.assertIn("answered", e)
                elif e["kind"] == "refusal":
                    self.assertIn(e["channel"], ["block", "unblock", "mute", "stop_request"])

    def test_healthy_reciprocity_and_refusal_sequence(self):
        gen = SyntheticGeneratorA(seed=999)
        heavy = gen.generate_scenario("heavy_benign")

        # In heavy_benign, reciprocity should be high
        reply_ratio_24h_idx = feature_order().index("reply_ratio_24h")
        reply_ratio = heavy.features[reply_ratio_24h_idx]
        self.assertGreater(
            reply_ratio,
            0.35,
            f"heavy_benign must have healthy reciprocity (got {reply_ratio})",
        )

        # In post_block_contacter, refusal must precede after_refusal events
        post_block = gen.generate_scenario("post_block_contacter")
        block_seen = False
        for e in post_block.events:
            if e["kind"] == "refusal" and e["channel"] == "block":
                block_seen = True
            if e.get("after_refusal"):
                self.assertTrue(block_seen, "after_refusal cannot happen before block refusal")

    def test_no_real_identifiers(self):
        gen = SyntheticGeneratorA(seed=777)
        for scen_type in ALL_SCENARIO_TYPES:
            scen = gen.generate_scenario(scen_type)
            self.assertTrue(scen.target_id.startswith("synth_"))
            self.assertTrue(scen.sender_id.startswith("synth_"))

            for e in scen.events:
                self.assertTrue(e["sender"].startswith("synth_"))
                self.assertTrue(e["recipient"].startswith("synth_"))
                # No real email / phone formats in synthetic identifiers
                self.assertNotIn("@", e["sender"])
                self.assertNotIn("+91", e["sender"])

    def test_golden_vector_order_and_values(self):
        """Validates feature extraction against the documented fixture in docs/FEATURES.md."""
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
        actual = feature_vector(f_dict)

        # Expected vector from docs/FEATURES.md:
        # [3,0.25,1,0.2,0.3,0,0,0,0,0,0, 3,0.25,1,0.2,0.3,0,0,0,0,0,0, 3,0.25,1,0.2,0.3,0,0,0,0,0,0,
        #  1,1,90,3,0,-1,1,0.1,0,1,-1,0,1,0]
        expected = []
        for _ in range(3):
            expected.extend([3.0, 0.25, 1.0, 0.2, 0.3, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0])
        expected.extend([
            1.0, 1.0, 90.0, 3.0, 0.0, -1.0, 1.0, 0.1, 0.0, 1.0, -1.0, 0.0, 1.0, 0.0
        ])

        self.assertEqual(len(actual), 47)
        self.assertEqual(len(expected), 47)

        for i, (act, exp) in enumerate(zip(actual, expected)):
            name = feature_order()[i]
            self.assertAlmostEqual(
                act,
                exp,
                places=4,
                msg=f"Feature {i} ({name}) mismatch: actual {act} != expected {exp}",
            )

    def test_generator_a_training_only(self):
        """Verifies generator A produces training partition metadata only."""
        gen = SyntheticGeneratorA(seed=555)
        scen = gen.generate_scenario("bully")

        self.assertEqual(scen.provenance["generator"], "generator_a")
        self.assertEqual(scen.provenance["partition"], "training")
        self.assertTrue(scen.provenance["synthetic"])
        self.assertNotIn("eval", scen.provenance["partition"])


if __name__ == "__main__":
    unittest.main()
