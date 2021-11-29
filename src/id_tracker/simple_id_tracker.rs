use crate::entry::entry_point::OperationResult;
use crate::id_tracker::IdTracker;
use crate::types::{PointIdType, PointOffsetType, SeqNumberType};
use bincode;
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

/// Since sled is used for reading only during the initialization, large read cache is not required
const DB_CACHE_SIZE: usize = 10 * 1024 * 1024; // 10 mb

const MAPPING_CF: &str = "mapping";
const VERSIONS_CF: &str = "versions";

pub struct SimpleIdTracker {
    internal_to_external: HashMap<PointOffsetType, PointIdType>,
    external_to_internal: BTreeMap<PointIdType, PointOffsetType>,
    external_to_version: HashMap<PointIdType, SeqNumberType>,
}

impl SimpleIdTracker {
    pub fn open(path: &Path) -> OperationResult<Self> {
        
        let mut internal_to_external: HashMap<PointOffsetType, PointIdType> = Default::default();
        let mut external_to_internal: BTreeMap<PointIdType, PointOffsetType> = Default::default();
        let mut external_to_version: HashMap<PointIdType, SeqNumberType> = Default::default();

        Ok(SimpleIdTracker {
            internal_to_external,
            external_to_internal,
            external_to_version,
        })
    }
}

impl IdTracker for SimpleIdTracker {
    fn version(&self, external_id: PointIdType) -> Option<SeqNumberType> {
        self.external_to_version.get(&external_id).cloned()
    }

    fn set_version(
        &mut self,
        external_id: PointIdType,
        version: SeqNumberType,
    ) -> OperationResult<()> {
        self.external_to_version.insert(external_id, version);
        Ok(())
    }

    fn internal_id(&self, external_id: PointIdType) -> Option<PointOffsetType> {
        self.external_to_internal.get(&external_id).cloned()
    }

    fn external_id(&self, internal_id: PointOffsetType) -> Option<PointIdType> {
        self.internal_to_external.get(&internal_id).cloned()
    }

    fn set_link(
        &mut self,
        external_id: PointIdType,
        internal_id: PointOffsetType,
    ) -> OperationResult<()> {
        self.external_to_internal.insert(external_id, internal_id);
        self.internal_to_external.insert(internal_id, external_id);

        Ok(())
    }

    fn drop(&mut self, external_id: PointIdType) -> OperationResult<()> {
        self.external_to_version.remove(&external_id);

        let internal_id = self.external_to_internal.remove(&external_id);
        match internal_id {
            Some(x) => self.internal_to_external.remove(&x),
            None => None,
        };
        
        Ok(())
    }

    fn iter_external(&self) -> Box<dyn Iterator<Item = PointIdType> + '_> {
        Box::new(self.external_to_internal.keys().cloned())
    }

    fn iter_internal(&self) -> Box<dyn Iterator<Item = PointOffsetType> + '_> {
        Box::new(self.internal_to_external.keys().cloned())
    }

    fn iter_from(
        &self,
        external_id: PointIdType,
    ) -> Box<dyn Iterator<Item = (PointIdType, PointOffsetType)> + '_> {
        Box::new(
            self.external_to_internal
                .range(external_id..PointIdType::MAX)
                .map(|(key, value)| (*key, *value)),
        )
    }

    fn flush(&self) -> OperationResult<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use itertools::Itertools;
    use tempdir::TempDir;

    #[test]
    fn test_iterator() {
        let dir = TempDir::new("storage_dir").unwrap();

        let mut id_tracker = SimpleIdTracker::open(dir.path()).unwrap();

        id_tracker.set_link(200, 0).unwrap();
        id_tracker.set_link(100, 1).unwrap();
        id_tracker.set_link(150, 2).unwrap();
        id_tracker.set_link(120, 3).unwrap();
        id_tracker.set_link(180, 4).unwrap();
        id_tracker.set_link(110, 5).unwrap();
        id_tracker.set_link(115, 6).unwrap();
        id_tracker.set_link(190, 7).unwrap();
        id_tracker.set_link(177, 8).unwrap();
        id_tracker.set_link(118, 9).unwrap();

        let first_four = id_tracker.iter_from(0).take(4).collect_vec();

        assert_eq!(first_four.len(), 4);
        assert_eq!(first_four[0].0, 100);

        let last = id_tracker.iter_from(first_four[3].0 + 1).collect_vec();
        assert_eq!(last.len(), 6);
    }
}
