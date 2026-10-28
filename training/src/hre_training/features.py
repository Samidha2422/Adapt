"""Feature extraction pipeline matching hre-behavior Rust implementation exactly."""

from dataclasses import dataclass, field
from typing import Any, Dict, List, Optional
import math

DAY_MS = 86_400_000

WINDOW_FEATURES = [
    "msg_count",
    "reply_ratio",
    "night_ratio",
    "tox_mean",
    "tox_max",
    "threat_max",
    "sexual_max",
    "sent_mean",
    "pii_mentions",
    "location_cue_count",
    "unsolicited_media_count",
]

GLOBAL_FEATURES = [
    "unreplied_streak",
    "initiator_ratio",
    "median_gap_s",
    "burst_max_10m",
    "after_refusal_count",
    "since_refusal_hours",
    "active_days_streak",
    "tox_slope_last20",
    "sent_slope_last20",
    "repetition_score",
    "sender_account_age_days",
    "is_contact",
    "distinct_channels",
    "evasion_similarity",
]


def feature_order() -> List[str]:
    """Returns the canonical 47-feature name order."""
    order = []
    for w in ["1h", "24h", "7d"]:
        for f in WINDOW_FEATURES:
            order.append(f"{f}_{w}")
    order.extend(GLOBAL_FEATURES)
    return order


@dataclass
class StoredEvent:
    event_id: str
    sender: str
    recipient: str
    timestamp_ms: int
    kind: str = "message"  # "message", "call", "refusal", "image", "audio"
    channel: str = "dm"
    content_hash: Optional[str] = None
    sender_account_age_days: Optional[float] = None
    is_contact: bool = False
    after_refusal: bool = False
    answered: bool = False
    duration_s: float = 0.0
    toxicity: float = 0.0
    threat: float = 0.0
    sexual: float = 0.0
    sentiment: float = 0.0
    pii_mentions: int = 0
    location_cues: int = 0
    unsolicited_media: bool = False


@dataclass
class BehaviorConfig:
    recipient_utc_offset_minutes: int = 0
    night_start_hour: int = 22
    night_end_hour: int = 6


def is_contact(e: StoredEvent) -> bool:
    return e.kind != "refusal"


def is_message(e: StoredEvent) -> bool:
    return e.kind == "message"


def is_recent(e: StoredEvent, now_ms: int, window_ms: int) -> bool:
    return (e.timestamp_ms >= (now_ms - window_ms)) and (e.timestamp_ms <= now_ms)


def is_night(e: StoredEvent, config: BehaviorConfig) -> bool:
    # Recipient local hour calculation
    hour = ((e.timestamp_ms // 60_000) + config.recipient_utc_offset_minutes) % 1440 // 60
    start = config.night_start_hour
    end = config.night_end_hour
    if start <= end:
        return start <= hour < end
    else:
        return hour >= start or hour < end


def calc_ratio(n: int, d: int) -> float:
    return 0.0 if d == 0 else float(n) / float(d)


def calc_slope(values: List[float]) -> float:
    if len(values) < 2:
        return 0.0
    n = len(values)
    mx = (n - 1) / 2.0
    my = sum(values) / float(n)
    numerator = sum((i - mx) * (y - my) for i, y in enumerate(values))
    denominator = sum((i - mx) ** 2 for i in range(n))
    return 0.0 if denominator == 0 else numerator / denominator


def extract_features(
    events: List[StoredEvent],
    sender: str,
    recipient: str,
    now_ms: int,
    config: Optional[BehaviorConfig] = None,
) -> Dict[str, float]:
    """Extracts all 47 features exactly reproducing hre-behavior."""
    if config is None:
        config = BehaviorConfig()

    seen = set()
    pair = []
    for e in events:
        if (
            (is_recent(e, now_ms, 7 * DAY_MS) or (e.kind == "refusal" and e.timestamp_ms <= now_ms))
            and (
                (e.sender == sender and e.recipient == recipient)
                or (e.sender == recipient and e.recipient == sender)
            )
            and e.event_id not in seen
        ):
            seen.add(e.event_id)
            pair.append(e)

    pair.sort(key=lambda x: (x.timestamp_ms, x.event_id))

    out = [e for e in pair if e.sender == sender and is_message(e)]
    contacts = [e for e in pair if is_contact(e)]

    f: Dict[str, float] = {}

    for w, ms in [("1h", 3_600_000), ("24h", DAY_MS), ("7d", 7 * DAY_MS)]:
        current = [e for e in out if is_recent(e, now_ms, ms)]
        replies = sum(1 for e in pair if e.sender == recipient and is_message(e) and is_recent(e, now_ms, ms))
        n = len(current)

        mean_tox = 0.0 if n == 0 else sum(e.toxicity for e in current) / float(n)
        max_tox = 0.0 if n == 0 else max(e.toxicity for e in current)
        max_threat = 0.0 if n == 0 else max(e.threat for e in current)
        max_sexual = 0.0 if n == 0 else max(e.sexual for e in current)
        mean_sent = 0.0 if n == 0 else sum(e.sentiment for e in current) / float(n)
        pii = float(sum(e.pii_mentions for e in current))
        loc = float(sum(e.location_cues for e in current))
        night_count = sum(1 for e in current if is_night(e, config))

        f[f"msg_count_{w}"] = float(n)
        f[f"reply_ratio_{w}"] = calc_ratio(replies, n + replies)
        f[f"night_ratio_{w}"] = calc_ratio(night_count, n)
        f[f"tox_mean_{w}"] = mean_tox
        f[f"tox_max_{w}"] = max_tox
        f[f"threat_max_{w}"] = max_threat
        f[f"sexual_max_{w}"] = max_sexual
        f[f"sent_mean_{w}"] = mean_sent
        f[f"pii_mentions_{w}"] = pii
        f[f"location_cue_count_{w}"] = loc

        media = sum(1 for e in pair if e.sender == sender and is_contact(e) and e.unsolicited_media and is_recent(e, now_ms, ms))
        f[f"unsolicited_media_count_{w}"] = float(media)

    gaps = []
    for i in range(len(out) - 1):
        gap = max(0, out[i + 1].timestamp_ms - out[i].timestamp_ms) / 1000.0
        gaps.append(gap)
    gaps.sort()

    if not gaps:
        median_gap = 0.0
    else:
        l = len(gaps)
        median_gap = (gaps[(l - 1) // 2] + gaps[l // 2]) / 2.0

    burst = 0
    left = 0
    for right in range(len(out)):
        while out[right].timestamp_ms - out[left].timestamp_ms > 600_000:
            left += 1
        burst = max(burst, right - left + 1)

    markers = [e for e in pair if e.sender == sender and e.kind == "refusal"]
    block_markers = [e for e in markers if e.channel in ("block", "unblock")]
    block = block_markers[-1] if block_markers else None
    unblock = block if (block and block.channel == "unblock") else None

    active_timestamps = []
    for e in markers:
        if e.channel not in ("block", "unblock"):
            active_timestamps.append(e.timestamp_ms)
        elif block and block.channel == "block" and block.event_id == e.event_id:
            active_timestamps.append(e.timestamp_ms)

    active = min(active_timestamps) if active_timestamps else None

    after = 0
    for e in contacts:
        if e.sender == sender:
            if active is not None:
                if e.timestamp_ms > active or (e.timestamp_ms == active and e.after_refusal):
                    after += 1
            else:
                if e.after_refusal and (unblock is None or e.timestamp_ms > unblock.timestamp_ms):
                    after += 1

    days = sorted(set(e.timestamp_ms // DAY_MS for e in out), reverse=True)
    active_days = 1 if days else 0
    for i in range(len(days) - 1):
        if days[i] - days[i + 1] != 1:
            break
        active_days += 1

    last20 = out[-20:]
    hashes: Dict[str, int] = {}
    for e in out:
        if e.content_hash:
            hashes[e.content_hash] = hashes.get(e.content_hash, 0) + 1

    known = sum(hashes.values())
    repeat = 0.0 if known < 2 else sum(n * (n - 1) for n in hashes.values()) / float(known * (known - 1))

    initiator = bool(contacts and contacts[0].sender == sender)

    unreplied = 0
    for e in reversed(contacts):
        if e.sender == sender:
            unreplied += 1
        else:
            break

    f["unreplied_streak"] = float(unreplied)
    f["initiator_ratio"] = float(initiator)
    f["median_gap_s"] = float(median_gap)
    f["burst_max_10m"] = float(burst)
    f["after_refusal_count"] = float(after)
    f["since_refusal_hours"] = -1.0 if active is None else max(0, now_ms - active) / 3_600_000.0
    f["active_days_streak"] = float(active_days)
    f["tox_slope_last20"] = calc_slope([e.toxicity for e in last20])
    f["sent_slope_last20"] = calc_slope([e.sentiment for e in last20])
    f["repetition_score"] = float(repeat)
    f["sender_account_age_days"] = (
        float(out[-1].sender_account_age_days)
        if (out and out[-1].sender_account_age_days is not None)
        else -1.0
    )
    f["is_contact"] = float(out[-1].is_contact) if out else 0.0
    f["distinct_channels"] = float(len(set(e.channel for e in contacts)))
    f["evasion_similarity"] = 0.0

    return f


def feature_vector(f: Dict[str, float]) -> List[float]:
    """Serializes feature dictionary into ordered vector of length 47."""
    return [f.get(k, 0.0) for k in feature_order()]
