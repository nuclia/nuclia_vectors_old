#[cfg(test)]
mod tests {
    use atomic_refcell::AtomicRefCell;
    use itertools::Itertools;
    use rand::{thread_rng, Rng};
    use segment::entry::entry_point::SegmentEntry;
    use segment::fixtures::payload_fixtures::{random_int_payload, random_vector};
    use segment::index::hnsw_index::hnsw::HNSWIndex;
    use segment::index::{PayloadIndex, VectorIndex};
    use segment::segment_constructor::build_segment;
    use segment::types::{
        Condition, Distance, FieldCondition, HnswConfig, Indexes, PayloadIndexType,
        PayloadKeyType, PayloadType, PointIdType, Range, SearchParams, SegmentConfig,
        SeqNumberType, StorageType, TheMap,
    };
    use std::sync::Arc;
    use tempdir::TempDir;

    #[test]
    fn test_filterable_hnsw() {
        let dim = 8;
        let m = 8;
        let num_vectors: PointIdType = 5_000;
        let ef = 32;
        let ef_construct = 16;
        let distance = Distance::Cosine;
        let indexing_threshold = 500;
        let num_payload_values = 2;

        let mut rnd = thread_rng();

        let dir = TempDir::new("segment_dir").unwrap();
        let payload_index_dir = TempDir::new("payload_index_dir").unwrap();
        let hnsw_dir = TempDir::new("hnsw_dir").unwrap();

        let config = SegmentConfig {
            vector_size: dim,
            index: Indexes::Plain {},
            payload_index: Some(PayloadIndexType::Plain),
            storage_type: StorageType::InMemory,
            distance,
        };

        let int_key = "int".to_string();

        let mut segment = build_segment(dir.path(), &config).unwrap();
        for idx in 0..num_vectors {
            let vector = random_vector(&mut rnd, dim);
            let mut payload: TheMap<PayloadKeyType, PayloadType> = Default::default();
            payload.insert(
                int_key.clone(),
                random_int_payload(&mut rnd, num_payload_values),
            );

            segment
                .upsert_point(idx as SeqNumberType, idx, &vector)
                .unwrap();
        }
        // let opnum = num_vectors + 1;


        let hnsw_config = HnswConfig {
            m,
            ef_construct,
            full_scan_threshold: indexing_threshold,
        };

        let mut hnsw_index = HNSWIndex::open(
            hnsw_dir.path(),
            segment.vector_storage.clone(),
            hnsw_config,
        )
        .unwrap();

        hnsw_index.build_index().unwrap();

       
        hnsw_index.build_index().unwrap();

        let top = 3;
        let mut hits = 0;
        let attempts = 100;
        for _i in 0..attempts {
            let query = random_vector(&mut rnd, dim);

            let index_result = hnsw_index.search_with_graph(
                &query,
                top,
                Some(&SearchParams { hnsw_ef: Some(ef) }),
            );

            let plain_result =
                segment
                    .vector_index
                    .borrow()
                    .search(&query, top, None);

            if plain_result == index_result {
                hits += 1;
            }
        }
        assert!(attempts - hits < 5, "hits: {} of {}", hits, attempts); // Not more than 5% failures
        eprintln!("hits = {:#?} out of {}", hits, attempts);
    }
}
