
use nuclia_vectors::entry::entry_point::SegmentEntry;
use nuclia_vectors::segment::Segment;
use nuclia_vectors::segment_constructor::simple_segment_constructor::build_simple_segment;
use nuclia_vectors::types::{Distance, PayloadType};
use std::path::Path;

pub fn empty_segment(path: &Path) -> Segment {
    build_simple_segment(path, 4, Distance::Dot, false).unwrap()
}

pub fn build_segment_1(path: &Path) -> Segment {
    let mut segment1 = empty_segment(path);

    let vec1 = vec![1.0, 0.0, 1.0, 1.0];
    let vec2 = vec![1.0, 0.0, 1.0, 0.0];
    let vec3 = vec![1.0, 1.0, 1.0, 1.0];
    let vec4 = vec![1.0, 1.0, 0.0, 1.0];
    let vec5 = vec![1.0, 0.0, 0.0, 0.0];

    segment1.upsert_point(1, 1, &vec1).unwrap();
    segment1.upsert_point(2, 2, &vec2).unwrap();
    segment1.upsert_point(3, 3, &vec3).unwrap();
    segment1.upsert_point(4, 4, &vec4).unwrap();
    segment1.upsert_point(5, 5, &vec5).unwrap();

    segment1
}

#[allow(dead_code)]
pub fn build_segment_2(path: &Path) -> Segment {
    let mut segment2 = empty_segment(path);

    let vec1 = vec![-1.0, 0.0, 1.0, 1.0];
    let vec2 = vec![-1.0, 0.0, 1.0, 0.0];
    let vec3 = vec![-1.0, 1.0, 1.0, 1.0];
    let vec4 = vec![-1.0, 1.0, 0.0, 1.0];
    let vec5 = vec![-1.0, 0.0, 0.0, 0.0];

    segment2.upsert_point(11, 11, &vec1).unwrap();
    segment2.upsert_point(12, 12, &vec2).unwrap();
    segment2.upsert_point(13, 13, &vec3).unwrap();
    segment2.upsert_point(14, 14, &vec4).unwrap();
    segment2.upsert_point(15, 15, &vec5).unwrap();
    
    segment2
}
