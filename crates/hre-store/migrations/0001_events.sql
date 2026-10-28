CREATE TABLE IF NOT EXISTS events (
    event_id TEXT PRIMARY KEY NOT NULL,
    sender TEXT NOT NULL,
    recipient TEXT NOT NULL,
    timestamp_ms INTEGER NOT NULL,
    kind TEXT NOT NULL,
    payload TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS events_pair_time ON events(sender, recipient, timestamp_ms, event_id);
CREATE INDEX IF NOT EXISTS events_target_time ON events(recipient, timestamp_ms, event_id);
CREATE INDEX IF NOT EXISTS events_pair_kind_time ON events(sender, recipient, kind, timestamp_ms DESC, event_id DESC);
CREATE INDEX IF NOT EXISTS events_refusal_channel_time ON events(sender, recipient, kind, json_extract(payload, '$.channel'), timestamp_ms DESC, event_id DESC);
CREATE TABLE IF NOT EXISTS assessments (
    event_id TEXT PRIMARY KEY NOT NULL REFERENCES events(event_id) ON DELETE CASCADE,
    payload TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS alerts (
    event_id TEXT PRIMARY KEY NOT NULL REFERENCES events(event_id) ON DELETE CASCADE,
    payload TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS feed_protection (
    recipient TEXT PRIMARY KEY NOT NULL,
    protected INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS reported_hashes (
    p_hash INTEGER NOT NULL,
    sha256 TEXT PRIMARY KEY NOT NULL,
    label TEXT NOT NULL,
    reported_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS reported_hashes_phash ON reported_hashes(p_hash);
