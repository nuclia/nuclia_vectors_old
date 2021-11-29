use crate::types::{
    Condition, FieldCondition, Match, PayloadType, Range as RangeCondition,
    VectorElementType,
};
use itertools::Itertools;
use rand::prelude::ThreadRng;
use rand::seq::SliceRandom;
use rand::Rng;
use std::ops::Range;

const ADJECTIVE: &[&str] = &[
    "jobless",
    "rightful",
    "breakable",
    "impartial",
    "shocking",
    "faded",
    "phobic",
    "overt",
    "like",
    "wide-eyed",
    "broad",
];

const NOUN: &[&str] = &[
    "territory",
    "jam",
    "neck",
    "chicken",
    "cap",
    "kiss",
    "veil",
    "trail",
    "size",
    "digestion",
    "rod",
    "seed",
];

const INT_RANGE: Range<i64> = 0..500;

pub fn random_keyword(rnd_gen: &mut ThreadRng) -> String {
    let random_adj = ADJECTIVE.choose(rnd_gen).unwrap();
    let random_noun = NOUN.choose(rnd_gen).unwrap();
    format!("{} {}", random_adj, random_noun)
}

pub fn random_keyword_payload(rnd_gen: &mut ThreadRng) -> PayloadType {
    PayloadType::Keyword(vec![random_keyword(rnd_gen)])
}

pub fn random_int_payload(rnd_gen: &mut ThreadRng, num_values: usize) -> PayloadType {
    PayloadType::Integer(
        (0..num_values)
            .map(|_| rnd_gen.gen_range(INT_RANGE))
            .collect_vec(),
    )
}

pub fn random_vector(rnd_gen: &mut ThreadRng, size: usize) -> Vec<VectorElementType> {
    (0..size).map(|_| rnd_gen.gen()).collect()
}

pub fn random_field_condition(rnd_gen: &mut ThreadRng) -> Condition {
    let kv_or_int: bool = rnd_gen.gen();
    match kv_or_int {
        true => Condition::Field(FieldCondition {
            key: "kvd".to_string(),
            r#match: Some(Match {
                keyword: Some(random_keyword(rnd_gen)),
                integer: None,
            }),
            range: None,
            geo_bounding_box: None,
            geo_radius: None,
        }),
        false => Condition::Field(FieldCondition {
            key: "int".to_string(),
            r#match: None,
            range: Some(RangeCondition {
                lt: None,
                gt: None,
                gte: Some(rnd_gen.gen_range(INT_RANGE) as f64),
                lte: Some(rnd_gen.gen_range(INT_RANGE) as f64),
            }),
            geo_bounding_box: None,
            geo_radius: None,
        }),
    }
}

