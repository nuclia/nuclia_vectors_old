use std::sync::Arc;

use atomic_refcell::AtomicRefCell;

use crate::{entry::entry_point::OperationResult, types::{SearchParams, VectorElementType}, vector_storage::{ScoredPointOffset, VectorStorage}};

use super::VectorIndex;



pub struct PlainIndex {
    vector_storage: Arc<AtomicRefCell<dyn VectorStorage>>,
}

impl PlainIndex {
    pub fn new(
        vector_storage: Arc<AtomicRefCell<dyn VectorStorage>>,
    ) -> PlainIndex {
        PlainIndex {
            vector_storage,
        }
    }
}

impl VectorIndex for PlainIndex {
    fn search(
        &self,
        vector: &[VectorElementType],
        top: usize,
        _params: Option<&SearchParams>,
    ) -> Vec<ScoredPointOffset> {
        self.vector_storage.borrow().score_all(vector, top)
    }

    fn build_index(&mut self) -> OperationResult<()> {
        Ok(())
    }
}
