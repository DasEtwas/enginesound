use criterion::{criterion_group, criterion_main, Criterion};
use enginesound::gen::{Generator, LowPassFilter};

fn bench_perf(c: &mut Criterion) {
    let simd = if is_x86_feature_detected!("avx2") {
        "avx2"
    } else if is_x86_feature_detected!("sse4.1") {
        "sse4.1"
    } else if is_x86_feature_detected!("sse2") {
        "sse2"
    } else {
        "scalar"
    };
    println!("SIMD: {}", simd);

    const SAMPLE_RATE: u32 = 48000;
    const DC_OFFSET_LP_FREQ: f32 = 4.0;

    let engine = enginesound::load_engine("example6.esc", SAMPLE_RATE).unwrap();

    let mut generator = Generator::new(
        SAMPLE_RATE,
        engine,
        LowPassFilter::new(DC_OFFSET_LP_FREQ, SAMPLE_RATE),
    );

    let mut buf = [0.0; SAMPLE_RATE as usize / 100];

    c.bench_function("perf", |b| b.iter(|| generator.generate(&mut buf)));
}

criterion_group!(benches, bench_perf);
criterion_main!(benches);
