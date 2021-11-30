mod prof;

use criterion::{criterion_group, criterion_main, Criterion};
use rand::thread_rng;
use nuclia_vectors::fixtures::index_fixtures::{FakeConditionChecker, TestRawScorerProducer};
use nuclia_vectors::index::hnsw_index::graph_layers::GraphLayers;
use nuclia_vectors::index::hnsw_index::point_scorer::FilteredScorer;
use nuclia_vectors::types::{Distance, PointOffsetType};

const NUM_VECTORS: usize = 10000;
const DIM: usize = 32;
const M: usize = 16;
const EF_CONSTRUCT: usize = 64;
const USE_HEURISTIC: bool = true;

fn hnsw_benchmark(c: &mut Criterion) {
    let vector_holder = TestRawScorerProducer::new(DIM, NUM_VECTORS, Distance::Cosine);
    let mut group = c.benchmark_group("hnsw-index-build-group");
    group.sample_size(10);
    group.bench_function("hnsw_index", |b| {
        b.iter(|| {
            let mut rng = thread_rng();
            let mut graph_layers =
                GraphLayers::new(NUM_VECTORS, M, M * 2, EF_CONSTRUCT, 10, USE_HEURISTIC);
            let fake_condition_checker = FakeConditionChecker {};
            for idx in 0..(NUM_VECTORS as PointOffsetType) {
                let added_vector = vector_holder.vectors[idx as usize].to_vec();
                let raw_scorer = vector_holder.get_raw_scorer(added_vector);
                let scorer = FilteredScorer {
                    raw_scorer: &raw_scorer,
                    condition_checker: &fake_condition_checker,
                    filter: None,
                };
                let level = graph_layers.get_random_layer(&mut rng);
                graph_layers.link_new_point(idx, level, &scorer);
            }
        })
    });
    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default().with_profiler(prof::FlamegraphProfiler::new(100));
    targets = hnsw_benchmark
}

criterion_main!(benches);
