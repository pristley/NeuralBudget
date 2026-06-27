use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use neuralbudget::StreamingAggregator;

/// Benchmark: push throughput at 100k samples
fn bench_push_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("streaming_aggregator");
    group.sample_size(10); // Reduced for large benchmarks

    for sample_count in [10_000, 100_000].iter() {
        group.bench_with_input(
            BenchmarkId::new("push_throughput", sample_count),
            sample_count,
            |b, &count| {
                b.iter(|| {
                    let mut agg = StreamingAggregator::new();
                    for i in 0..count {
                        // Synthetic data: timestamp and value
                        let ts = black_box(i as i64);
                        let val = black_box((i % 100) as f64);
                        agg.push(ts, val);
                    }
                    black_box(agg)
                });
            },
        );
    }
    group.finish();
}

/// Benchmark: moving average calculation on large windows
fn bench_moving_average(c: &mut Criterion) {
    let mut group = c.benchmark_group("streaming_aggregator");
    group.sample_size(10);

    for window_size in [100, 1_000, 10_000].iter() {
        group.bench_with_input(
            BenchmarkId::new("moving_average", window_size),
            window_size,
            |b, &window| {
                let mut agg = StreamingAggregator::new();
                // Pre-populate with synthetic data
                for i in 0..100_000 {
                    agg.push(i as i64, (i % 100) as f64);
                }

                b.iter(|| {
                    let current_ts = black_box(99_999i64);
                    let ws = black_box(window as i64);
                    let avg = agg.get_moving_average(current_ts, ws);
                    black_box(avg)
                });
            },
        );
    }
    group.finish();
}

/// Benchmark: prune operation (memory management)
fn bench_prune(c: &mut Criterion) {
    c.bench_function("streaming_aggregator::prune_50pct", |b| {
        b.iter_batched(
            || {
                // Setup: create aggregator with 100k entries
                let mut agg = StreamingAggregator::new();
                for i in 0..100_000 {
                    agg.push(i as i64, (i % 100) as f64);
                }
                agg
            },
            |mut agg| {
                // Prune 50% of the buffer
                agg.prune(black_box(50_000i64));
                black_box(agg)
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

criterion_group!(
    benches,
    bench_push_throughput,
    bench_moving_average,
    bench_prune
);
criterion_main!(benches);
