use criterion::{black_box, criterion_group, criterion_main, Criterion};
use neuralbudget::{
    evaluate_composite_slo, CompositeDependencyEdge, CompositeServiceSlo, CompositeSloGraph,
};

fn make_chain_graph(size: usize) -> CompositeSloGraph {
    let mut services = Vec::with_capacity(size);
    let mut dependencies = Vec::with_capacity(size.saturating_sub(1));

    for index in 0..size {
        services.push(CompositeServiceSlo {
            service: format!("svc_{index:05}"),
            local_score: if index % 17 == 0 { 0.82 } else { 0.97 },
            min_pass_score: 0.9,
            impact_weight: 1.0,
        });

        if index > 0 {
            dependencies.push(CompositeDependencyEdge {
                dependency: format!("svc_{:05}", index - 1),
                dependent: format!("svc_{index:05}"),
                failure_penalty: 0.08,
            });
        }
    }

    CompositeSloGraph {
        services,
        dependencies,
        global_min_pass_score: 0.9,
    }
}

fn bench_composite_slo_dag(c: &mut Criterion) {
    let mut group = c.benchmark_group("composite_slo_dag");

    for size in [100_usize, 1_000_usize, 5_000_usize] {
        let graph = make_chain_graph(size);
        group.bench_function(format!("chain_{size}"), |b| {
            b.iter(|| {
                let evaluation = evaluate_composite_slo(black_box(&graph))
                    .expect("benchmark graph must be valid");
                black_box(evaluation.global_slo);
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_composite_slo_dag);
criterion_main!(benches);
