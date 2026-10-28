use criterion::{black_box, criterion_group, criterion_main, Criterion};
use hre_fusion::{fuse, FusionConfig, FusionInput};
fn benchmark(c: &mut Criterion) {
    let input = FusionInput::default();
    let config = FusionConfig::default();
    c.bench_function("fusion_noisy_or", |b| {
        b.iter(|| fuse(black_box(&input), black_box(&config)))
    });
}
criterion_group!(benches, benchmark);
criterion_main!(benches);
