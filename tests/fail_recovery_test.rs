mod fixtures;

#[cfg(test)]
mod tests {
    use crate::fixtures::segment::empty_segment;
    use segment::entry::entry_point::{OperationError, SegmentEntry, SegmentFailedState};
    use segment::types::PayloadType;
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

        // op_num is greater than errored. Skip because not recovered yet
        let fail_res = segment.set_payload(
            3,
            1,
            &"color".to_string(),
            PayloadType::Keyword(vec!["red".to_string()]),
        );
        assert!(fail_res.is_err());

        // Also skip even with another point operation
        let fail_res = segment.set_payload(
            3,
            2,
            &"color".to_string(),
            PayloadType::Keyword(vec!["red".to_string()]),
        );
        assert!(fail_res.is_err());

        // Perform operation, but keep error status: operation is not fully recovered yet
        let ok_res = segment.set_payload(
            2,
            2,
            &"color".to_string(),
            PayloadType::Keyword(vec!["red".to_string()]),
        );
        assert!(ok_res.is_ok());
        assert!(segment.error_status.is_some());

        // Perform operation anf recover the error - operation is fixed now
        let recover_res = segment.set_payload(
            2,
            1,
            &"color".to_string(),
            PayloadType::Keyword(vec!["red".to_string()]),
        );

        assert!(recover_res.is_ok());
        assert!(segment.error_status.is_none());
    }
}
