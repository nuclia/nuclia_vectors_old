use crate::entry::entry_point::{OperationError, OperationResult};
use crate::id_tracker::simple_id_tracker::SimpleIdTracker;
use crate::index::hnsw_index::hnsw::HNSWIndex;
use crate::index::{PayloadIndex, VectorIndex};
use crate::segment::{Segment, SEGMENT_STATE_FILE};
use crate::types::{
    Indexes, PayloadIndexType, SegmentConfig, SegmentState, SegmentType, SeqNumberType, StorageType,
};
use crate::vector_storage::memmap_vector_storage::MemmapVectorStorage;
use crate::vector_storage::simple_vector_storage::SimpleVectorStorage;
use crate::vector_storage::VectorStorage;
use atomic_refcell::AtomicRefCell;
use std::fs::{create_dir_all, File};
use std::io::Read;
use std::path::Path;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

fn sp<T>(t: T) -> Arc<AtomicRefCell<T>> {
    Arc::new(AtomicRefCell::new(t))
}

fn create_segment(
    version: SeqNumberType,
    segment_path: &Path,
    config: &SegmentConfig,
) -> OperationResult<Segment> {
    let tracker_path = segment_path.join("id_tracker");
    let payload_storage_path = segment_path.join("payload_storage");
    let payload_index_path = segment_path.join("payload_index");
    let vector_storage_path = segment_path.join("vector_storage");
    let vector_index_path = segment_path.join("vector_index");

    let id_tracker = sp(SimpleIdTracker::open(&tracker_path)?);

    let vector_storage: Arc<AtomicRefCell<dyn VectorStorage>> = match config.storage_type {
        StorageType::InMemory => sp(SimpleVectorStorage::open(
            &vector_storage_path,
            config.vector_size,
            config.distance,
        )?),
        StorageType::Mmap => sp(MemmapVectorStorage::open(
            &vector_storage_path,
            config.vector_size,
            config.distance,
        )?),
    };

    let vector_index: Arc<AtomicRefCell<dyn VectorIndex>> = match config.index {
        Indexes::Hnsw(hnsw_config) => sp(HNSWIndex::open(
            &vector_index_path,
            vector_storage.clone(),
            hnsw_config,
        )?),
    };

    let segment_type = match config.index {
        Indexes::Hnsw { .. } => SegmentType::Indexed,
    };

    let appendable_flag =
        segment_type == SegmentType::Plain {} && config.storage_type == StorageType::InMemory;

    Ok(Segment {
        version,
        persisted_version: Arc::new(Mutex::new(version)),
        current_path: segment_path.to_owned(),
        id_tracker,
        vector_storage,
        vector_index,
        appendable_flag,
        segment_type,
        segment_config: config.clone(),
        error_status: None,
    })
}

pub fn load_segment(path: &Path) -> OperationResult<Segment> {
    let segment_config_path = path.join(SEGMENT_STATE_FILE);
    let mut contents = String::new();

    let mut file = File::open(segment_config_path)?;
    file.read_to_string(&mut contents)?;

    let segment_state: SegmentState =
        serde_json::from_str(&contents).map_err(|err| OperationError::ServiceError {
            description: format!(
                "Failed to read segment {}. Error: {}",
                path.to_str().unwrap(),
                err
            ),
        })?;

    create_segment(segment_state.version, path, &segment_state.config)
}

/// Build segment instance using given configuration.
/// Builder will generate folder for the segment and store all segment information inside it.
///
/// # Arguments
///
/// * `path` - A path to collection. Segment folder will be created in this directory
/// * `config` - Segment configuration
///
///
pub fn build_segment(path: &Path, config: &SegmentConfig) -> OperationResult<Segment> {
    create_dir_all(&path)?;

    let segment = create_segment(0, &path, config)?;
    segment.save_current_state()?;

    Ok(segment)
}
