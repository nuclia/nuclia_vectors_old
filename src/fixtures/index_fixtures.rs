use crate::payload_storage::ConditionChecker;
use crate::spaces::metric::Metric;
use crate::spaces::tools::mertic_object;
use crate::types::{Distance, Filter, PointOffsetType, VectorElementType};
use crate::vector_storage::simple_vector_storage::SimpleRawScorer;
use bit_vec::BitVec;
use itertools::Itertools;
use ndarray::{Array, Array1};
use rand::Rng;

pub fn random_vector<R: Rng + ?Sized>(rnd_gen: &mut R, size: usize) -> Vec<VectorElementType> {
    (0..size).map(|_| rnd_gen.gen()).collect()
}

pub struct FakeConditionChecker {}

impl ConditionChecker for FakeConditionChecker {
    fn check(&self, _point_id: PointOffsetType, _query: &Filter) -> bool {
        true
    }
}

pub struct TestRawScorerProducer {
    pub vectors: Vec<Array1<VectorElementType>>,
    pub deleted: BitVec,
    pub metric: Box<dyn Metric>,
}

impl TestRawScorerProducer {
    pub fn new<R>(dim: usize, num_vectors: usize, distance: Distance, rng: &mut R) -> Self
    where
        R: Rng + ?Sized,
    {
        let metric = mertic_object(&distance);

        let vectors = (0..num_vectors)
            .map(|_x| {
                let rnd_vec = random_vector(rng, dim);
                metric.preprocess(&rnd_vec).unwrap_or(rnd_vec)
            })
            .collect_vec();

        TestRawScorerProducer {
            vectors: vectors.into_iter().map(Array::from).collect(),
            deleted: BitVec::from_elem(num_vectors, false),
            metric,
        }
    }

    pub fn get_raw_scorer(&self, query: Vec<VectorElementType>) -> SimpleRawScorer {
        SimpleRawScorer {
            query: Array::from(self.metric.preprocess(&query).unwrap_or(query)),
            metric: self.metric.as_ref(),
            vectors: &self.vectors,
            deleted: &self.deleted,
        }
    }
}
