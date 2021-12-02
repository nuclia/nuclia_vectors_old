use crate::entry::entry_point::OperationResult;
use crate::types::{PayloadKeyType, PayloadKeyTypeRef, PointOffsetType, SearchParams, VectorElementType,
};
use crate::vector_storage::ScoredPointOffset;

/// Trait for vector searching
pub trait VectorIndex {
    /// Return list of Ids with fitting
    fn search(
        &self,
        vector: &[VectorElementType],
        top: usize,
        params: Option<&SearchParams>,
    ) -> Vec<ScoredPointOffset>;

    /// Force internal index rebuild.
    fn build_index(&mut self) -> OperationResult<()>;
}
