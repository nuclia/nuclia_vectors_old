mod fixtures;

#[cfg(test)]
mod tests {
    use crate::fixtures::segment::empty_segment;
    use nuclia_vectors::entry::entry_point::{OperationError, SegmentEntry, SegmentFailedState};
    use nuclia_vectors::types::PayloadType;
    use tempdir::TempDir;

    #[test]
    fn test_insert_fail_recovery() {
        let dir = TempDir::new("segment_dir").unwrap();

        let vec1 = vec![1.0, 0.0, 1.0, 1.0];

        let mut segment = empty_segment(dir.path());

        segment.upsert_point(1, 1, &vec1).unwrap();
        segment.upsert_point(1, 2, &vec1).unwrap();

        segment.error_status = Some(SegmentFailedState {
            version: 2,
            point_id: Some(1),
            error: OperationError::ServiceError {
                description: "test error".to_string(),
            },
        });
       
        assert!(segment.error_status.is_some());
    }
}
