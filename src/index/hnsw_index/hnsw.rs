use crate::entry::entry_point::OperationResult;
use crate::index::hnsw_index::build_condition_checker::BuildConditionChecker;
use crate::index::hnsw_index::config::HnswGraphConfig;
use crate::index::hnsw_index::graph_layers::GraphLayers;
use crate::index::hnsw_index::point_scorer::FilteredScorer;
use crate::index::sample_estimation::sample_check_cardinality;
use crate::index::{PayloadIndex, VectorIndex};
use crate::types::Condition::Field;
use crate::types::{
    FieldCondition, HnswConfig, PointOffsetType, SearchParams, VectorElementType,
};
use crate::vector_storage::{ScoredPointOffset, VectorStorage};
use atomic_refcell::AtomicRefCell;
use log::debug;
use rand::prelude::ThreadRng;
use rand::thread_rng;
use std::cmp::max;
use std::fs::create_dir_all;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const HNSW_USE_HEURISTIC: bool = true;

pub struct HNSWIndex {
    vector_storage: Arc<AtomicRefCell<dyn VectorStorage>>,
    config: HnswGraphConfig,
    path: PathBuf,
    thread_rng: ThreadRng,
    graph: GraphLayers,
}

impl HNSWIndex {
    pub fn open(
        path: &Path,
        vector_storage: Arc<AtomicRefCell<dyn VectorStorage>>,
        hnsw_config: HnswConfig,
    ) -> OperationResult<Self> {
        create_dir_all(path)?;
        let rng = thread_rng();

        let config_path = HnswGraphConfig::get_config_path(path);
        let config = if config_path.exists() {
            HnswGraphConfig::load(&config_path)?
        } else {
            HnswGraphConfig::new(
                hnsw_config.m,
                hnsw_config.ef_construct,
                hnsw_config.full_scan_threshold,
            )
        };

        let graph_path = GraphLayers::get_path(path);
        let graph = if graph_path.exists() {
            GraphLayers::load(&graph_path)?
        } else {
            let total_points = vector_storage.borrow().total_vector_count();
            GraphLayers::new(
                vector_storage.borrow().total_vector_count(),
                config.m,
                config.m0,
                config.ef_construct,
                max(1, total_points / hnsw_config.full_scan_threshold * 10),
                HNSW_USE_HEURISTIC,
            )
        };

        Ok(HNSWIndex {
            vector_storage,
            config,
            path: path.to_owned(),
            thread_rng: rng,
            graph,
        })
    }

    fn save_config(&self) -> OperationResult<()> {
        let config_path = HnswGraphConfig::get_config_path(&self.path);
        self.config.save(&config_path)
    }

    fn save_graph(&self) -> OperationResult<()> {
        let graph_path = GraphLayers::get_path(&self.path);
        self.graph.save(&graph_path)
    }

    pub fn save(&self) -> OperationResult<()> {
        self.save_config()?;
        self.save_graph()?;
        Ok(())
    }

    pub fn link_point(&mut self, point_id: PointOffsetType, points_scorer: &FilteredScorer) {
        let point_level = self.graph.get_random_layer(&mut self.thread_rng);
        self.graph
            .link_new_point(point_id, point_level, points_scorer);
    }

    pub fn build_filtered_graph(
        &self,
        graph: &mut GraphLayers,
        condition: FieldCondition,
        block_condition_checker: &mut BuildConditionChecker,
    ) {
        block_condition_checker.filter_list.next_iteration();

        let vector_storage = self.vector_storage.borrow();
    }

    pub fn search_with_graph(
        &self,
        vector: &[VectorElementType],
        top: usize,
        params: Option<&SearchParams>,
    ) -> Vec<ScoredPointOffset> {
        let req_ef = params
            .and_then(|params| params.hnsw_ef)
            .unwrap_or(self.config.ef);

        // ef should always be bigger that required top
        let ef = max(req_ef, top);

        let vector_storage = self.vector_storage.borrow();
        let raw_scorer = vector_storage.raw_scorer(vector.to_owned());

        let points_scorer = FilteredScorer {
            raw_scorer: raw_scorer.as_ref(),
        };

        self.graph.search(top, ef, &points_scorer)
    }
}

impl VectorIndex for HNSWIndex {
    fn search(
        &self,
        vector: &[VectorElementType],
        top: usize,
        params: Option<&SearchParams>,
    ) -> Vec<ScoredPointOffset> {
        self.search_with_graph(vector, top, params)
    }

    fn build_index(&mut self) -> OperationResult<()> {
        // Build main index graph
        let vector_storage = self.vector_storage.borrow();
        let mut rng = thread_rng();

        let total_points = vector_storage.total_vector_count();

        debug!("building hnsw for {}", total_points);
        self.graph = GraphLayers::new(
            total_points,
            self.config.m,
            self.config.m0,
            self.config.ef_construct,
            max(1, total_points / self.config.indexing_threshold * 10),
            HNSW_USE_HEURISTIC,
        );

        for vector_id in vector_storage.iter_ids() {
            let vector = vector_storage.get_vector(vector_id).unwrap();
            let raw_scorer = vector_storage.raw_scorer(vector);
            let points_scorer = FilteredScorer {
                raw_scorer: raw_scorer.as_ref(),
            };

            let level = self.graph.get_random_layer(&mut rng);
            self.graph.link_new_point(vector_id, level, &points_scorer);
        }

        debug!("finish main graph");

        let total_vectors_count = vector_storage.total_vector_count();
        let mut block_condition_checker = BuildConditionChecker::new(total_vectors_count);


        
        self.save()
    }
}
