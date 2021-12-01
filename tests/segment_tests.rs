mod fixtures;

#[cfg(test)]
mod tests {
    use crate::fixtures::segment::build_segment_1;
    use nuclia_vectors::entry::entry_point::SegmentEntry;
    use nuclia_vectors::segment_constructor::build_segment;
    use nuclia_vectors::types::{Condition, WithPayload, SegmentConfig, Indexes, Distance};
    use std::collections::HashSet;
    use std::path::Path;
    use tempdir::TempDir;


    #[test]
    fn testttt() {
        let config = SegmentConfig {
            vector_size: 3,
            index: Indexes::Plain {},
            payload_index: None,
            distance: Distance::Dot,
            storage_type: Default::default(),
        };

        //let dir = TempDir::new().unwrap();
        let mut segment_w = build_segment(Path::new("data"), &config, false).unwrap();
        let segment_r = build_segment(Path::new("data"), &config, true).unwrap();

        let vec1 = vec![1.0, 0.0, 1.0];
        let vec2 = vec![1.0, 0.0, 1.0];
        let vec3 = vec![1.0, 1.0, 1.0];
        let vec4 = vec![1.0, 1.0, 0.0];
        let vec5 = vec![1.0, 0.0, 0.0];
    
        segment_w.upsert_point(1, 1, &vec1).unwrap();
        segment_w.upsert_point(2, 2, &vec2).unwrap();
        segment_w.upsert_point(3, 3, &vec3).unwrap();
        segment_w.upsert_point(4, 4, &vec4).unwrap();
        segment_w.upsert_point(5, 5, &vec5).unwrap();

        segment_w.vector(1).unwrap();

        segment_r.vector(1).unwrap();
    }


    #[test]
    fn test_point_exclusion() {
        let dir = TempDir::new("segment_dir").unwrap();

        let segment = build_segment_1(dir.path());
        assert!(segment.has_point(3));

        let query_vector = vec![1.0, 1.0, 1.0, 1.0];

        let res = segment
            .search(&query_vector, &WithPayload::default(), 1, None)
            .unwrap();

        let res2 = segment
            .search(&query_vector, &WithPayload::default(), 3, None)
            .unwrap();
        dbg!(res2);

        let best_match = res.get(0).expect("Non-empty result");
        assert_eq!(best_match.id, 3);


        let point_ids1: Vec<_> = segment.iter_points().collect();
        let point_ids2: Vec<_> = segment.iter_points().collect();

        assert!(!point_ids1.is_empty());
        assert!(!point_ids2.is_empty());

        assert_eq!(&point_ids1, &point_ids2)
    }
}
