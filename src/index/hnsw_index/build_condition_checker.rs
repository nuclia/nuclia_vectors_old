use crate::index::visited_pool::VisitedList;
use crate::types::{PointOffsetType};

pub struct BuildConditionChecker {
    pub filter_list: VisitedList,
    pub current_point: PointOffsetType,
}

impl BuildConditionChecker {
    pub fn new(list_size: usize) -> Self {
        BuildConditionChecker {
            filter_list: VisitedList::new(list_size),
            current_point: PointOffsetType::default(),
        }
    }
}