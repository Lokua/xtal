use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_fps_reads(c: &mut Criterion) {
    c.bench_function("fps_reads", |b| {
        b.iter(|| {
            for _ in 0..100 {
                black_box(xtal::framework::frame_controller::fps());
            }
        })
    });
}

fn bench_frame_count(c: &mut Criterion) {
    c.bench_function("frame_count", |b| {
        b.iter(|| {
            for _ in 0..100 {
                black_box(xtal::framework::frame_controller::frame_count());
            }
        })
    });
}

criterion_group!(benches, bench_fps_reads, bench_frame_count);
criterion_main!(benches);
