"""Synthetic training data generator A.

Generates deterministic synthetic message and call scenarios for training behavioral
models and baseline classifiers. Labeled explicitly as training-only.
"""

from dataclasses import dataclass, asdict
from typing import Any, Dict, List, Optional
import random
import uuid

from hre_training.features import DAY_MS, StoredEvent, extract_features, feature_vector

ALL_SCENARIO_TYPES = [
    "normal",
    "heavy_benign",
    "polite_stalker",
    "escalating_stalker",
    "post_block_contacter",
    "pile_on",
    "bully",
    "cyberflasher",
    "sextortion",
    "silent_caller",
    "night_caller",
    "post_block_caller",
    "new_number_caller",
    "noisy_ambiguous",
]


@dataclass
class SyntheticScenario:
    scenario_id: str
    scenario_type: str
    target_id: str
    sender_id: str
    events: List[Dict[str, Any]]
    expected_tier: str
    expected_severity_range: tuple
    features: List[float]
    provenance: Dict[str, Any]


class SyntheticGeneratorA:
    """Deterministic synthetic conversation and call history generator."""

    def __init__(self, seed: int = 42):
        self.seed = seed
        self.rng = random.Random(seed)
        self.base_time_ms = 1_700_000_000_000  # deterministic epoch base

    def generate_scenario(self, scenario_type: str, custom_id: Optional[str] = None) -> SyntheticScenario:
        if scenario_type not in ALL_SCENARIO_TYPES:
            raise ValueError(f"Unknown scenario type '{scenario_type}'. Must be one of {ALL_SCENARIO_TYPES}")

        scen_id = custom_id or f"gen_a_{scenario_type}_{self.rng.randint(1000, 9999)}"
        target_id = f"synth_target_{self.rng.randint(100, 999)}"
        sender_id = f"synth_sender_{self.rng.randint(100, 999)}"

        events: List[StoredEvent] = []
        now_ms = self.base_time_ms + self.rng.randint(10_000_000, 50_000_000)

        expected_tier = "low"
        sev_min, sev_max = 0, 30

        if scenario_type == "normal":
            expected_tier = "low"
            sev_min, sev_max = 0, 20
            t = now_ms - 3_600_000
            for i in range(6):
                t += self.rng.randint(120_000, 600_000)
                snd = sender_id if i % 2 == 0 else target_id
                rcp = target_id if snd == sender_id else sender_id
                events.append(StoredEvent(
                    event_id=f"evt_{i:04d}",
                    sender=snd,
                    recipient=rcp,
                    timestamp_ms=t,
                    kind="message",
                    channel="dm",
                    is_contact=True,
                    toxicity=0.01,
                    sentiment=0.4,
                ))

        elif scenario_type == "heavy_benign":
            expected_tier = "low"
            sev_min, sev_max = 0, 25
            t = now_ms - 86_400_000
            for i in range(50):
                t += self.rng.randint(60_000, 1_200_000)
                snd = sender_id if self.rng.random() < 0.55 else target_id
                rcp = target_id if snd == sender_id else sender_id
                events.append(StoredEvent(
                    event_id=f"evt_{i:04d}",
                    sender=snd,
                    recipient=rcp,
                    timestamp_ms=t,
                    kind="message",
                    channel="dm",
                    is_contact=True,
                    toxicity=0.0,
                    sentiment=0.5,
                ))
            # Include long answered call
            events.append(StoredEvent(
                event_id="evt_call_01",
                sender=sender_id,
                recipient=target_id,
                timestamp_ms=t + 180_000,
                kind="call",
                channel="call",
                is_contact=True,
                answered=True,
                duration_s=450.0,
            ))

        elif scenario_type == "polite_stalker":
            expected_tier = "high"
            sev_min, sev_max = 65, 85
            t = now_ms - 72_000_000
            # 60 relentless polite unreplied messages
            for i in range(60):
                t += self.rng.randint(300_000, 900_000)
                events.append(StoredEvent(
                    event_id=f"evt_{i:04d}",
                    sender=sender_id,
                    recipient=target_id,
                    timestamp_ms=t,
                    kind="message",
                    channel="dm",
                    is_contact=False,
                    toxicity=0.05,
                    sentiment=0.1,
                ))
            # Block refusal
            t += 60_000
            events.append(StoredEvent(
                event_id="evt_refusal_block",
                sender=target_id,
                recipient=sender_id,
                timestamp_ms=t,
                kind="refusal",
                channel="block",
            ))
            # Continued contacts after block
            for j in range(5):
                t += 120_000
                events.append(StoredEvent(
                    event_id=f"evt_post_{j:04d}",
                    sender=sender_id,
                    recipient=target_id,
                    timestamp_ms=t,
                    kind="message",
                    channel="dm",
                    is_contact=False,
                    after_refusal=True,
                    toxicity=0.05,
                ))

        elif scenario_type == "escalating_stalker":
            expected_tier = "high"
            sev_min, sev_max = 70, 90
            t = now_ms - 36_000_000
            for i in range(25):
                t += self.rng.randint(300_000, 600_000)
                tox = min(0.95, 0.05 + i * 0.04)
                th = 0.9 if i > 20 else 0.0
                events.append(StoredEvent(
                    event_id=f"evt_{i:04d}",
                    sender=sender_id,
                    recipient=target_id,
                    timestamp_ms=t,
                    kind="message",
                    channel="dm",
                    is_contact=False,
                    toxicity=tox,
                    threat=th,
                    sentiment=-0.5 if i > 15 else 0.0,
                ))

        elif scenario_type == "post_block_contacter":
            expected_tier = "high"
            sev_min, sev_max = 65, 85
            t = now_ms - 10_000_000
            events.append(StoredEvent(
                event_id="evt_block_marker",
                sender=target_id,
                recipient=sender_id,
                timestamp_ms=t,
                kind="refusal",
                channel="block",
            ))
            for i in range(8):
                t += 180_000
                events.append(StoredEvent(
                    event_id=f"evt_after_{i:04d}",
                    sender=sender_id,
                    recipient=target_id,
                    timestamp_ms=t,
                    kind="message",
                    channel="dm",
                    is_contact=False,
                    after_refusal=True,
                    toxicity=0.1,
                ))

        elif scenario_type == "pile_on":
            expected_tier = "high"
            sev_min, sev_max = 65, 85
            t = now_ms - 60_000
            for i in range(12):
                t += 4_000
                distinct_sender = f"synth_spammer_{i:03d}"
                events.append(StoredEvent(
                    event_id=f"evt_pile_{i:04d}",
                    sender=distinct_sender,
                    recipient=target_id,
                    timestamp_ms=t,
                    kind="message",
                    channel="comment",
                    is_contact=False,
                    toxicity=0.6,
                ))

        elif scenario_type == "bully":
            expected_tier = "high"
            sev_min, sev_max = 65, 85
            t = now_ms - 7_200_000
            for i in range(10):
                t += 300_000
                events.append(StoredEvent(
                    event_id=f"evt_bully_{i:04d}",
                    sender=sender_id,
                    recipient=target_id,
                    timestamp_ms=t,
                    kind="message",
                    channel="dm",
                    is_contact=False,
                    toxicity=0.85,
                    sentiment=-0.8,
                ))

        elif scenario_type == "cyberflasher":
            expected_tier = "high"
            sev_min, sev_max = 65, 85
            events.append(StoredEvent(
                event_id="evt_explicit_media",
                sender=sender_id,
                recipient=target_id,
                timestamp_ms=now_ms - 60_000,
                kind="image",
                channel="dm",
                is_contact=False,
                unsolicited_media=True,
                sexual=0.92,
            ))

        elif scenario_type == "sextortion":
            expected_tier = "urgent"
            sev_min, sev_max = 86, 100
            t = now_ms - 600_000
            events.append(StoredEvent(
                event_id="evt_sextortion_01",
                sender=sender_id,
                recipient=target_id,
                timestamp_ms=t,
                kind="message",
                channel="dm",
                is_contact=False,
                toxicity=0.9,
                threat=0.9,
                location_cues=1,
            ))

        elif scenario_type == "silent_caller":
            expected_tier = "review"
            sev_min, sev_max = 35, 60
            t = now_ms - 7_200_000
            for i in range(15):
                t += 300_000
                events.append(StoredEvent(
                    event_id=f"evt_call_{i:04d}",
                    sender=sender_id,
                    recipient=target_id,
                    timestamp_ms=t,
                    kind="call",
                    channel="call",
                    is_contact=False,
                    answered=True,
                    duration_s=2.0,  # silent / hangup
                ))

        elif scenario_type == "night_caller":
            expected_tier = "review"
            sev_min, sev_max = 35, 60
            # Set time in recipient night (e.g. 02:00 UTC)
            t = (now_ms // DAY_MS) * DAY_MS + 2 * 3_600_000
            for i in range(12):
                t += 180_000
                events.append(StoredEvent(
                    event_id=f"evt_night_{i:04d}",
                    sender=sender_id,
                    recipient=target_id,
                    timestamp_ms=t,
                    kind="call",
                    channel="call",
                    is_contact=False,
                    answered=False,
                    duration_s=0.0,
                ))
            now_ms = t + 60_000

        elif scenario_type == "post_block_caller":
            expected_tier = "high"
            sev_min, sev_max = 65, 85
            t = now_ms - 3_600_000
            events.append(StoredEvent(
                event_id="evt_block_call",
                sender=target_id,
                recipient=sender_id,
                timestamp_ms=t,
                kind="refusal",
                channel="block",
            ))
            for i in range(5):
                t += 300_000
                events.append(StoredEvent(
                    event_id=f"evt_call_after_{i:04d}",
                    sender=sender_id,
                    recipient=target_id,
                    timestamp_ms=t,
                    kind="call",
                    channel="call",
                    is_contact=False,
                    after_refusal=True,
                    answered=False,
                    duration_s=0.0,
                ))

        elif scenario_type == "new_number_caller":
            expected_tier = "review"
            sev_min, sev_max = 35, 60
            t = now_ms - 1_800_000
            for i in range(8):
                t += 180_000
                events.append(StoredEvent(
                    event_id=f"evt_new_num_{i:04d}",
                    sender=sender_id,
                    recipient=target_id,
                    timestamp_ms=t,
                    kind="call",
                    channel="call",
                    sender_account_age_days=1.0,
                    is_contact=False,
                    answered=False,
                ))

        elif scenario_type == "noisy_ambiguous":
            expected_tier = "low"
            sev_min, sev_max = 10, 40
            t = now_ms - 1_800_000
            events.append(StoredEvent(
                event_id="evt_ambig_01",
                sender=sender_id,
                recipient=target_id,
                timestamp_ms=t,
                kind="message",
                channel="dm",
                is_contact=True,
                toxicity=0.35,  # borderline
                sentiment=0.1,
            ))
            events.append(StoredEvent(
                event_id="evt_ambig_02",
                sender=target_id,
                recipient=sender_id,
                timestamp_ms=t + 120_000,
                kind="message",
                channel="dm",
                is_contact=True,
                toxicity=0.0,
                sentiment=0.3,
            ))

        # Sort events chronologically
        events.sort(key=lambda e: (e.timestamp_ms, e.event_id))
        now_ms = max(now_ms, events[-1].timestamp_ms if events else now_ms)

        features_dict = extract_features(events, sender_id, target_id, now_ms)
        f_vec = feature_vector(features_dict)

        serialized_events = [asdict(e) for e in events]

        provenance = {
            "generator": "generator_a",
            "version": "1.0.0",
            "partition": "training",
            "seed": self.seed,
            "synthetic": True,
            "disclaimer": "Generated for model training only. Structurally distinct from evaluation generator B.",
        }

        return SyntheticScenario(
            scenario_id=scen_id,
            scenario_type=scenario_type,
            target_id=target_id,
            sender_id=sender_id,
            events=serialized_events,
            expected_tier=expected_tier,
            expected_severity_range=(sev_min, sev_max),
            features=f_vec,
            provenance=provenance,
        )

    def generate_all_scenarios(self) -> List[SyntheticScenario]:
        """Generates one instance of every scenario class."""
        return [self.generate_scenario(st) for st in ALL_SCENARIO_TYPES]
