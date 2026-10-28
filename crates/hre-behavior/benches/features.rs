use criterion::{black_box, criterion_group, criterion_main, Criterion};
use hre_behavior::{extract_features, BehaviorConfig};
use hre_store::StoredEvent;
fn benchmark(c: &mut Criterion) {
    let events: Vec<_> = (0..1000)
        .map(|i| StoredEvent {
            event_id: i.to_string(),
            sender: "a".repeat(64),
            recipient: "b".repeat(64),
            timestamp_ms: i * 1000,
            ..Default::default()
        })
        .collect();
    c.bench_function("features_1000_events", |b| {
        b.iter(|| {
            extract_features(
                black_box(&events),
                &"a".repeat(64),
                &"b".repeat(64),
                1_000_000,
                &BehaviorConfig::default(),
            )
        })
    });
}
criterion_group!(benches, benchmark);
criterion_main!(benches);
