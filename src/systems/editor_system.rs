use std::collections::{BTreeMap, BTreeSet};

use crate::entities::game_object::GameObject;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EditorFrameReport {
    pub entities: usize,
    pub selected: usize,
    pub locked: usize,
    pub duplicate_ids: Vec<u64>,
    pub invalid_transforms: Vec<u64>,
}

#[derive(Debug, Clone, Default)]
pub struct EditorSystem {
    pub frame: u64,
    pub stats: BTreeMap<String, usize>,
    pub last_report: EditorFrameReport,
}

impl EditorSystem {
    pub fn update(&mut self, entities: &[GameObject]) -> &EditorFrameReport {
        self.frame = self.frame.saturating_add(1);
        let mut seen = BTreeSet::new();
        let mut duplicate_ids = BTreeSet::new();
        let mut invalid_transforms = Vec::new();
        for entity in entities {
            if !seen.insert(entity.id) {
                duplicate_ids.insert(entity.id);
            }
            if !entity.x.is_finite()
                || !entity.y.is_finite()
                || !entity.rotation.is_finite()
                || !entity.scale_x.is_finite()
                || !entity.scale_y.is_finite()
            {
                invalid_transforms.push(entity.id);
            }
        }
        self.last_report = EditorFrameReport {
            entities: entities.len(),
            selected: entities.iter().filter(|entity| entity.selected).count(),
            locked: entities.iter().filter(|entity| entity.locked).count(),
            duplicate_ids: duplicate_ids.into_iter().collect(),
            invalid_transforms,
        };
        self.stats = BTreeMap::from([
            ("entities".to_string(), self.last_report.entities),
            ("selected".to_string(), self.last_report.selected),
            (
                "duplicate_ids".to_string(),
                self.last_report.duplicate_ids.len(),
            ),
            (
                "invalid_transforms".to_string(),
                self.last_report.invalid_transforms.len(),
            ),
        ]);
        &self.last_report
    }

    pub fn select_only(entities: &mut [GameObject], entity_id: Option<u64>) -> bool {
        let mut found = entity_id.is_none();
        for entity in entities {
            let selected = entity_id == Some(entity.id) && !entity.locked;
            entity.set_selected(selected);
            found |= selected;
        }
        found
    }
}
