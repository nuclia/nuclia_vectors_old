use crate::entry::entry_point::{
    get_service_error, OperationError, OperationResult, SegmentEntry, SegmentFailedState,
};
use crate::id_tracker::IdTracker;
use crate::index::{VectorIndex};
use crate::spaces::tools::mertic_object;
use crate::types::{
    PayloadKeyType, PayloadKeyTypeRef, PayloadSchemaInfo, PayloadType, PointIdType,
    PointOffsetType, ScoredPoint, SearchParams, SegmentConfig, SegmentInfo, SegmentState,
    SegmentType, SeqNumberType, TheMap, VectorElementType, WithPayload,
};
use crate::vector_storage::VectorStorage;
use atomic_refcell::AtomicRefCell;
use atomicwrites::{AllowOverwrite, AtomicFile};
use std::fs::{remove_dir_all, rename};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub const SEGMENT_STATE_FILE: &str = "segment.json";

/// Simple segment implementation
pub struct Segment {
    pub version: SeqNumberType,
    pub persisted_version: Arc<Mutex<SeqNumberType>>,
    pub current_path: PathBuf,
    pub id_tracker: Arc<AtomicRefCell<dyn IdTracker>>,
    pub vector_storage: Arc<AtomicRefCell<dyn VectorStorage>>,
    pub vector_index: Arc<AtomicRefCell<dyn VectorIndex>>,
    pub appendable_flag: bool,
    pub segment_type: SegmentType,
    pub segment_config: SegmentConfig,
    pub error_status: Option<SegmentFailedState>,
}

impl Segment {
    fn update_vector(
        &mut self,
        old_internal_id: PointOffsetType,
        vector: Vec<VectorElementType>,
    ) -> OperationResult<PointOffsetType> {
        let new_internal_index = {
            let mut vector_storage = self.vector_storage.borrow_mut();
            vector_storage.update_vector(old_internal_id, vector)
        }?;

        Ok(new_internal_index)
    }

    fn handle_version_and_failure<F>(
        &mut self,
        op_num: SeqNumberType,
        op_point_id: Option<PointIdType>,
        operation: F,
    ) -> OperationResult<bool>
    where
        F: FnOnce(&mut Segment) -> OperationResult<bool>,
    {
        if let Some(SegmentFailedState {
            version: failed_version,
            point_id: _failed_point_id,
            error,
        }) = &self.error_status
        {
            // Failed operations should not be skipped,
            // fail if newer operation is attempted before proper recovery
            if *failed_version < op_num {
                return Err(OperationError::ServiceError {
                    description: format!("Not recovered from previous error: {}", error),
                });
            } // else: Re-try operation
        }

        let res = self.handle_version(op_num, op_point_id, operation);

        match get_service_error(&res) {
            None => {
                // Recover error state
                match &self.error_status {
                    None => {} // all good
                    Some(error) => {
                        if error.point_id == op_point_id {
                            // Fixed
                            self.error_status = None;
                        }
                    }
                }
            }
            Some(error) => {
                // ToDo: Recover previous segment state
                self.error_status = Some(SegmentFailedState {
                    version: op_num,
                    point_id: op_point_id,
                    error,
                })
            }
        }
        res
    }

    /// Manage segment version checking
    /// If current version if higher than operation version - do not perform the operation
    /// Update current version if operation successfully executed
    fn handle_version<F>(
        &mut self,
        op_num: SeqNumberType,
        op_point_id: Option<PointIdType>,
        operation: F,
    ) -> OperationResult<bool>
    where
        F: FnOnce(&mut Segment) -> OperationResult<bool>,
    {
        match op_point_id {
            None => {
                // Not a point operation, use global version to check if already applied
                if self.version > op_num {
                    return Ok(false); // Skip without execution
                }
            }
            Some(point_id) => {
                // Check if point not exists or have lower version
                if self
                    .id_tracker
                    .borrow()
                    .version(point_id)
                    .map(|current_version| current_version > op_num)
                    .unwrap_or(false)
                {
                    return Ok(false);
                }
            }
        }

        let res = operation(self);

        if res.is_ok() {
            self.version = op_num;
            if let Some(point_id) = op_point_id {
                self.id_tracker.borrow_mut().set_version(point_id, op_num)?;
            }
        }
        res
    }

    fn lookup_internal_id(&self, point_id: PointIdType) -> OperationResult<PointOffsetType> {
        let internal_id_opt = self.id_tracker.borrow().internal_id(point_id);
        match internal_id_opt {
            Some(internal_id) => Ok(internal_id),
            None => Err(OperationError::PointIdError {
                missed_point_id: point_id,
            }),
        }
    }

    fn get_state(&self) -> SegmentState {
        SegmentState {
            version: self.version(),
            config: self.segment_config.clone(),
        }
    }

    fn save_state(&self, state: &SegmentState) -> OperationResult<()> {
        let state_path = self.current_path.join(SEGMENT_STATE_FILE);
        let af = AtomicFile::new(state_path, AllowOverwrite);
        let state_bytes = serde_json::to_vec(state).unwrap();
        af.write(|f| f.write_all(&state_bytes))?;
        Ok(())
    }

    pub fn save_current_state(&self) -> OperationResult<()> {
        self.save_state(&self.get_state())
    }
}

impl SegmentEntry for Segment {
    fn version(&self) -> SeqNumberType {
        self.version
    }

    fn point_version(&self, point_id: PointIdType) -> Option<SeqNumberType> {
        self.id_tracker.borrow().version(point_id)
    }

    fn search(
        &self,
        vector: &[VectorElementType],
        with_payload: &WithPayload,
        top: usize,
        params: Option<&SearchParams>,
    ) -> OperationResult<Vec<ScoredPoint>> {
        let expected_vector_dim = self.vector_storage.borrow().vector_dim();
        if expected_vector_dim != vector.len() {
            return Err(OperationError::WrongVector {
                expected_dim: expected_vector_dim,
                received_dim: vector.len(),
            });
        }

        let internal_result = self
            .vector_index
            .borrow()
            .search(vector, top, params);

        let id_tracker = self.id_tracker.borrow();

        let res: OperationResult<Vec<ScoredPoint>> = internal_result
            .iter()
            .map(|&scored_point_offset| {
                let point_id = id_tracker.external_id(scored_point_offset.idx).ok_or(
                    OperationError::ServiceError {
                        description: format!(
                            "Corrupter id_tracker, no external value for {}",
                            scored_point_offset.idx
                        ),
                    },
                )?;
                let point_version =
                    id_tracker
                        .version(point_id)
                        .ok_or(OperationError::ServiceError {
                            description: format!(
                                "Corrupter id_tracker, no version for point {}",
                                point_id
                            ),
                        })?;
                Ok(ScoredPoint {
                    id: point_id,
                    version: point_version,
                    score: scored_point_offset.score
                })
            })
            .collect();
        res
    }

    fn upsert_point(
        &mut self,
        op_num: SeqNumberType,
        point_id: PointIdType,
        vector: &[VectorElementType],
    ) -> OperationResult<bool> {
        self.handle_version_and_failure(op_num, Some(point_id), |segment| {
            let vector_dim = segment.vector_storage.borrow().vector_dim();
            if vector_dim != vector.len() {
                return Err(OperationError::WrongVector {
                    expected_dim: vector_dim,
                    received_dim: vector.len(),
                });
            }

            let metric = mertic_object(&segment.segment_config.distance);
            let processed_vector = metric
                .preprocess(vector)
                .unwrap_or_else(|| vector.to_owned());

            let stored_internal_point = segment.id_tracker.borrow().internal_id(point_id);

            let was_replaced = match stored_internal_point {
                Some(existing_internal_id) => {
                    let new_index =
                        segment.update_vector(existing_internal_id, processed_vector)?;
                    if new_index != existing_internal_id {
                        let mut id_tracker = segment.id_tracker.borrow_mut();
                        id_tracker.drop(point_id)?;
                        id_tracker.set_link(point_id, new_index)?;
                    }
                    true
                }
                None => {
                    let new_index = segment
                        .vector_storage
                        .borrow_mut()
                        .put_vector(processed_vector)?;
                    segment
                        .id_tracker
                        .borrow_mut()
                        .set_link(point_id, new_index)?;
                    false
                }
            };

            Ok(was_replaced)
        })
    }

    fn delete_point(
        &mut self,
        op_num: SeqNumberType,
        point_id: PointIdType,
    ) -> OperationResult<bool> {
        self.handle_version_and_failure(op_num, Some(point_id), |segment| {
            let mut id_tracker = segment.id_tracker.borrow_mut();
            let internal_id = id_tracker.internal_id(point_id);
            match internal_id {
                Some(internal_id) => {
                    segment.vector_storage.borrow_mut().delete(internal_id)?;
                    id_tracker.drop(point_id)?;
                    Ok(true)
                }
                None => Ok(false),
            }
        })
    }

    fn vector(&self, point_id: PointIdType) -> OperationResult<Vec<VectorElementType>> {
        let internal_id = self.lookup_internal_id(point_id)?;
        Ok(self
            .vector_storage
            .borrow()
            .get_vector(internal_id)
            .unwrap())
    }

    fn iter_points(&self) -> Box<dyn Iterator<Item = PointIdType> + '_> {
        // Sorry for that, but I didn't find any way easier.
        // If you try simply return iterator - it won't work because AtomicRef should exist
        // If you try to make callback instead - you won't be able to create <dyn SegmentEntry>
        // Attempt to create return borrowed value along with iterator failed because of insane lifetimes
        unsafe { self.id_tracker.as_ptr().as_ref().unwrap().iter_external() }
    }

    fn read_filtered<'a>(
        &'a self,
        offset: PointIdType,
        limit: usize,
    ) -> Vec<PointIdType> {
        let storage = self.vector_storage.borrow();
        self
            .id_tracker
            .borrow()
            .iter_from(offset)
            .map(|x| x.0)
            .take(limit)
            .collect()
    }

    fn has_point(&self, point_id: PointIdType) -> bool {
        self.id_tracker.borrow().internal_id(point_id).is_some()
    }

    fn vectors_count(&self) -> usize {
        self.vector_storage.borrow().vector_count()
    }

    fn deleted_count(&self) -> usize {
        self.vector_storage.borrow().deleted_count()
    }

    fn segment_type(&self) -> SegmentType {
        self.segment_type
    }

    fn info(&self) -> SegmentInfo {

        SegmentInfo {
            segment_type: self.segment_type,
            num_vectors: self.vectors_count(),
            num_deleted_vectors: self.vector_storage.borrow().deleted_count(),
            ram_usage_bytes: 0,  // ToDo: Implement
            disk_usage_bytes: 0, // ToDo: Implement
            is_appendable: self.appendable_flag,
        }
    }

    fn config(&self) -> SegmentConfig {
        self.segment_config.clone()
    }

    fn is_appendable(&self) -> bool {
        self.appendable_flag
    }

    fn flush(&self) -> OperationResult<SeqNumberType> {
        let mut persisted_version = self.persisted_version.lock().unwrap();
        if *persisted_version == self.version() {
            return Ok(*persisted_version);
        }

        let state = self.get_state();

        self.id_tracker.borrow().flush()?;
        self.vector_storage.borrow().flush()?;
        self.save_state(&state)?;

        *persisted_version = state.version;

        Ok(state.version)
    }

    fn drop_data(&mut self) -> OperationResult<()> {
        let mut deleted_path = self.current_path.clone();
        deleted_path.set_extension("deleted");
        rename(&self.current_path, &deleted_path)?;
        Ok(remove_dir_all(&deleted_path)?)
    }

    fn check_error(&self) -> Option<SegmentFailedState> {
        self.error_status.clone()
    }
}