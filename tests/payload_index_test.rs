#[cfg(test)]
mod tests {
    use itertools::Itertools;
    use segment::entry::entry_point::SegmentEntry;
    use segment::fixtures::payload_fixtures::{
        random_int_payload, random_keyword_payload, random_vector,
    };
    use segment::segment_constructor::build_segment;
    use segment::types::{
        Condition, Distance, FieldCondition, Indexes, PayloadIndexType, PayloadKeyType,
        PayloadType, Range, SegmentConfig, StorageType, TheMap, WithPayload,
    };
    use tempdir::TempDir;

    #[test]
    fn test_cardinality_estimation() {
        let mut rnd = rand::thread_rng();

        let dir1 = TempDir::new("segment1_dir").unwrap();
        let dim = 5;

        let config = SegmentConfig {
            vector_size: dim,
            index: Indexes::Plain {},
            payload_index: Some(PayloadIndexType::Struct),
            storage_type: StorageType::InMemory,
            distance: Distance::Dot,
        };

        let str_key = "kvd".to_string();
        let int_key = "int".to_string();

        let num_points = 10000;
        let mut struct_segment = build_segment(dir1.path(), &config).unwrap();

        let mut opnum = 0;
        for idx in 0..num_points {
            let vector = random_vector(&mut rnd, dim);
            let mut payload: TheMap<PayloadKeyType, PayloadType> = Default::default();
            payload.insert(str_key.clone(), random_keyword_payload(&mut rnd));
            payload.insert(int_key.clone(), random_int_payload(&mut rnd, 2));

            struct_segment.upsert_point(opnum, idx, &vector).unwrap();

            opnum += 1;
        }


        let exact = struct_segment
            .vector_storage
            .borrow()
            .iter_ids()
            .collect_vec()
            .len();

        eprintln!("exact = {:#?}", exact);

    }

    #[test]
    fn test_struct_payload_index() {
        // Compare search with plain and struct indexes
        let mut rnd = rand::thread_rng();

        let dir1 = TempDir::new("segment1_dir").unwrap();
        let dir2 = TempDir::new("segment2_dir").unwrap();

        let dim = 5;

        let mut config = SegmentConfig {
            vector_size: dim,
            index: Indexes::Plain {},
            payload_index: Some(PayloadIndexType::Plain),
            storage_type: StorageType::InMemory,
            distance: Distance::Dot,
        };

        let mut plain_segment = build_segment(dir1.path(), &config).unwrap();
        config.payload_index = Some(PayloadIndexType::Struct);
        let mut struct_segment = build_segment(dir2.path(), &config).unwrap();

        let str_key = "kvd".to_string();
        let int_key = "int".to_string();

        let num_points = 1000;
        let num_int_values = 2;

        let mut opnum = 0;
        for idx in 0..num_points {
            let vector = random_vector(&mut rnd, dim);
            let mut payload: TheMap<PayloadKeyType, PayloadType> = Default::default();
            payload.insert(str_key.clone(), random_keyword_payload(&mut rnd));
            payload.insert(
                int_key.clone(),
                random_int_payload(&mut rnd, num_int_values),
            );

            plain_segment.upsert_point(idx, idx, &vector).unwrap();
            struct_segment.upsert_point(idx, idx, &vector).unwrap();

            opnum += 1;
        }

        let attempts = 100;
        for _i in 0..attempts {
            let query_vector = random_vector(&mut rnd, dim);

            let plain_result = plain_segment
                .search(
                    &query_vector,
                    &WithPayload::default(),
                    5,
                    None,
                )
                .unwrap();
            let struct_result = struct_segment
                .search(
                    &query_vector,
                    &WithPayload::default(),
                    5,
                    None,
                )
                .unwrap();

            plain_result
                .iter()
                .zip(struct_result.iter())
                .for_each(|(r1, r2)| {
                    assert_eq!(r1.id, r2.id);
                    assert!((r1.score - r2.score) < 0.0001)
                });
        }
    }
}
