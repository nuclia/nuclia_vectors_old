use std::ops::Range;
use std::path::Path;

use log::debug;
use rocksdb::{IteratorMode, Options, DB};
use serde::{Deserialize, Serialize};

use crate::entry::entry_point::OperationResult;
use crate::spaces::tools::{mertic_object, peek_top_scores_iterable};
use crate::types::{Distance, PointOffsetType, ScoreType, VectorElementType};
use crate::vector_storage::{RawScorer, ScoredPointOffset};

use super::vector_storage_base::VectorStorage;
use crate::spaces::metric::Metric;
use bit_vec::BitVec;
use ndarray::{Array, Array1};
use std::mem::size_of;

/// Since sled is used for reading only during the initialization, large read cache is not required
const DB_CACHE_SIZE: usize = 10 * 1024 * 1024; // 10 mb

pub struct DriveVectorStorage {
    dim: usize,
    metric: Box<dyn Metric>,
    store: DB,
    len: usize
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct StoredRecord {
    pub vector: Vec<VectorElementType>,
}

pub struct DriveRawScorer<'a> {
    pub query: Array1<VectorElementType>,
    pub metric: &'a dyn Metric,
    pub storage: &'a DriveVectorStorage
}

impl RawScorer for DriveRawScorer<'_> {

    fn score_points<'a>(
        &'a self,
        points: &'a mut dyn Iterator<Item = PointOffsetType>,
    ) -> Box<dyn Iterator<Item = ScoredPointOffset> + 'a> {
        let res_iter = points
            .map(move |point| {
                let other_vector = self.storage.get_vector(point);
                match  other_vector {
                    Some(vec) => Some(ScoredPointOffset {
                        idx: point,
                        score: self.metric.blas_similarity(&self.query, &Array::from(vec)),
                    }),
                    None => None,
                }
                
            })
            .filter(|x| x.is_some())
            .map(|x| x.unwrap());
        Box::new(res_iter)
    }

    fn check_point(&self, point: PointOffsetType) -> bool {
        point < self.storage.len() as PointOffsetType
    }

    fn score_point(&self, point: PointOffsetType) -> ScoreType {
        let other_vector = &self.storage.get_vector(point).unwrap();
        self.metric.blas_similarity(&self.query, &Array::from(other_vector.clone()))
    }

    fn score_internal(&self, point_a: PointOffsetType, point_b: PointOffsetType) -> ScoreType {
        let vector_a = &self.storage.get_vector(point_a).unwrap();
        let vector_b = &self.storage.get_vector(point_b).unwrap();
        
        self.metric.blas_similarity(
            &Array::from(vector_a.clone()), 
            &Array::from(vector_b.clone())
        )
    }
}

impl DriveVectorStorage {

    pub fn open(path: &Path, dim: usize, distance: Distance, read_only: bool) -> OperationResult<Self> {


        let mut options: Options = Options::default();
        options.set_write_buffer_size(DB_CACHE_SIZE);
        options.create_if_missing(true);

        let store = match read_only {
            true => DB::open_for_read_only(&options, path, false)?,
            false => DB::open(&options, path)?,
        };

        let metric = mertic_object(&distance);

        let mut n_keys = 0 as usize;
        for (key, val) in store.iterator(IteratorMode::Start) {
            n_keys += 1;
        }

        Ok(DriveVectorStorage {
            dim,
            metric,
            store,
            len: n_keys
        })
    }


    pub fn upsert(&self, point_id: PointOffsetType, vector: Vec<VectorElementType>) -> OperationResult<()> {
        
        let record = StoredRecord { vector };

        self.store.put(
            bincode::serialize(&point_id).unwrap(),
            bincode::serialize(&record).unwrap(),
        )?;

        Ok(())
    }

    pub fn len(&self) -> usize {
        self.len
    }
}

impl VectorStorage for DriveVectorStorage {
    fn vector_dim(&self) -> usize {
        self.dim
    }

    fn vector_count(&self) -> usize {
        self.len
    }

    fn deleted_count(&self) -> usize {
        0
    }

    fn total_vector_count(&self) -> usize {
        self.len
    }

    fn get_vector(&self, key: PointOffsetType) -> Option<Vec<VectorElementType>> {

        let val = self.store.get(bincode::serialize(&key).unwrap());
        match val {
            Ok(val) => match val {
                Some(val) => {
                    let stored_record: StoredRecord = bincode::deserialize(&val).unwrap();
                    Some(stored_record.vector)
                },
                None => {
                    eprintln!("No vector with this key found.");
                    None
                },
            }
            Err(e) => {
                eprintln!("Error retrieving key{}", e);
                None
            },
        }
    }

    fn put_vector(&mut self, vector: Vec<VectorElementType>) -> OperationResult<PointOffsetType> {
        assert_eq!(self.dim, vector.len());

        let new_id = self.len as PointOffsetType;
        self.len += 1;

        self.upsert(new_id, vector)?;

        Ok(new_id)
    }

    fn update_vector(
        &mut self,
        key: PointOffsetType,
        vector: Vec<VectorElementType>,
    ) -> OperationResult<PointOffsetType> {

        self.upsert(key, vector)?;
        Ok(key)
    }

    fn update_from(
        &mut self,
        other: &dyn VectorStorage,
    ) -> OperationResult<Range<PointOffsetType>> {
        let start_index = self.len as u32;

        for id in other.iter_ids() {
            let other_vector = other.get_vector(id).unwrap();
            self.put_vector(other_vector).unwrap();
        }
        let end_index = self.len as u32;
        Ok(start_index..end_index)
    }

    fn delete(&mut self, key: PointOffsetType) -> OperationResult<()> {
        Ok(self.store.delete(bincode::serialize(&key).unwrap())?)
    }

    fn is_deleted(&self, _: PointOffsetType) -> bool {
        false
    }

    fn iter_ids(&self) -> Box<dyn Iterator<Item = PointOffsetType> + '_> {
        let iter = self.store.iterator(IteratorMode::Start)
            .map(|(point_id, _)| {
                let point: PointOffsetType = bincode::deserialize(&point_id).unwrap();
                point
            });

        Box::new(iter)
    }

    fn flush(&self) -> OperationResult<()> {
        Ok(self.store.flush()?)
    }

    fn raw_scorer(&self, vector: Vec<VectorElementType>) -> Box<dyn RawScorer + '_> {
        Box::new(DriveRawScorer {
            query: Array::from(self.metric.preprocess(&vector).unwrap_or(vector)),
            metric: self.metric.as_ref(),
            storage: self
        })
    }

    fn raw_scorer_internal(&self, point_id: PointOffsetType) -> Box<dyn RawScorer + '_> {
        let vector = self.get_vector(point_id).unwrap();
        Box::new(DriveRawScorer {
            query: Array::from(self.metric.preprocess(&vector).unwrap_or(vector)),
            metric: self.metric.as_ref(),
            storage: self
        })
    }

    fn score_points(
        &self,
        vector: &[VectorElementType],
        points: &mut dyn Iterator<Item = PointOffsetType>,
        top: usize,
    ) -> Vec<ScoredPointOffset> {
        let preprocessed_vector = Array::from(
            self.metric
                .preprocess(vector)
                .unwrap_or_else(|| vector.to_owned()),
        );
        let scores: Vec<_> = points
            .map(|point| {
                let other_vector = self.get_vector(point);

                match other_vector {
                    Some(vec) => Some(ScoredPointOffset {
                        idx: point,
                        score: self
                                .metric
                                .blas_similarity(&preprocessed_vector, &Array::from(vec)),
                    }),
                    None => None,
                }
                
            })
            .filter(|x| x.is_some())
            .map(|x| x.unwrap())
            .collect();

        peek_top_scores_iterable(scores, top)
    }

    fn score_all(&self, vector: &[VectorElementType], top: usize) -> Vec<ScoredPointOffset> {
        let preprocessed_vector = Array::from(
            self.metric
                .preprocess(vector)
                .unwrap_or_else(|| vector.to_owned()),
        );

        let scores = self
            .store
            .iterator(IteratorMode::Start)
            .map(|(point, other_vector)| {
                
                let point: PointOffsetType = bincode::deserialize(&point).unwrap();
                let other_vector: Vec<f32> = bincode::deserialize(&other_vector).unwrap();

                ScoredPointOffset {
                    idx: point,
                    score: self
                        .metric
                        .blas_similarity(&preprocessed_vector, &Array::from(other_vector)),
                }
            });
            

        peek_top_scores_iterable(scores, top)
    }

    fn score_internal(
        &self,
        point: PointOffsetType,
        points: &mut dyn Iterator<Item = PointOffsetType>,
        top: usize,
    ) -> Vec<ScoredPointOffset> {
        let vector = self.get_vector(point).unwrap();
        self.score_points(&vector, points, top)
    }
}

#[cfg(test)]
mod tests {
    use tempdir::TempDir;

    use super::*;
    use itertools::Itertools;

    #[test]
    fn test_score_points() {
        let dir = TempDir::new("storage_dir").unwrap();
        let distance = Distance::Dot;
        let dim = 4;
        let mut storage = DriveVectorStorage::open(dir.path(), dim, distance, false).unwrap();
        let vec0 = vec![1.0, 0.0, 1.0, 1.0];
        let vec1 = vec![1.0, 0.0, 1.0, 0.0];
        let vec2 = vec![1.0, 1.0, 1.0, 1.0];
        let vec3 = vec![1.0, 1.0, 0.0, 1.0];
        let vec4 = vec![1.0, 0.0, 0.0, 0.0];

        let _id1 = storage.put_vector(vec0.clone()).unwrap();
        let id2 = storage.put_vector(vec1.clone()).unwrap();
        let _id3 = storage.put_vector(vec2.clone()).unwrap();
        let _id4 = storage.put_vector(vec3.clone()).unwrap();
        let id5 = storage.put_vector(vec4.clone()).unwrap();

        assert_eq!(id2, 1);
        assert_eq!(id5, 4);

        let query = vec![0.0, 1.0, 1.1, 1.0];

        let closest = storage.score_points(&query, &mut [0, 1, 2, 3, 4].iter().cloned(), 2);

        let top_idx = match closest.get(0) {
            Some(scored_point) => {
                assert_eq!(scored_point.idx, 2);
                scored_point.idx
            }
            None => {
                panic!("No close vector found!")
            }
        };

        storage.delete(top_idx).unwrap();

        let closest = storage.score_points(&query, &mut [0, 1, 2, 3, 4].iter().cloned(), 2);

        let raw_scorer = storage.raw_scorer(query.clone());

        let query_points = vec![0, 1, 2, 3, 4];
        let mut query_points1 = query_points.iter().cloned();
        let mut query_points2 = query_points.iter().cloned();

        let raw_res1 = raw_scorer.score_points(&mut query_points1).collect_vec();
        let raw_res2 = raw_scorer.score_points(&mut query_points2).collect_vec();

        assert_eq!(raw_res1, raw_res2);

        let _top_idx = match closest.get(0) {
            Some(scored_point) => {
                assert_ne!(scored_point.idx, 2);
                assert_eq!(&raw_res1[scored_point.idx as usize], scored_point);
            }
            None => {
                panic!("No close vector found!")
            }
        };

        let all_ids1: Vec<_> = storage.iter_ids().collect();
        let all_ids2: Vec<_> = storage.iter_ids().collect();

        assert_eq!(all_ids1, all_ids2);

        assert!(!all_ids1.contains(&top_idx))
    }
}
