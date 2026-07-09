use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

static NEXT_ID: AtomicU64 = AtomicU64::new(1);
static NAME_COUNTERS: OnceLock<Mutex<BTreeMap<String, u64>>> = OnceLock::new();

pub fn generate_entity_id() -> u64 {
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

/// Reserves every id up to and including `existing_id`.
///
/// Scene and prefab deserialization may restore ids that are much larger than
/// the number of entities loaded in the current process. Advancing the
/// allocator prevents a subsequently created entity from reusing one of those
/// persistent ids.
pub fn register_existing_entity_id(existing_id: u64) {
    if let Some(next_available) = existing_id.checked_add(1) {
        NEXT_ID.fetch_max(next_available, Ordering::Relaxed);
    }
}

pub fn generate_entity_name(prefix: &str) -> String {
    let counters = NAME_COUNTERS.get_or_init(|| Mutex::new(BTreeMap::new()));
    let mut counters = counters.lock().expect("entity name counter poisoned");
    let next = counters.entry(prefix.to_string()).or_insert(0);
    *next += 1;
    format!("{prefix}_{next}")
}

pub fn register_existing_name(name: &str) {
    let Some((prefix, suffix)) = name.rsplit_once('_') else {
        return;
    };
    let Ok(number) = suffix.parse::<u64>() else {
        return;
    };
    let counters = NAME_COUNTERS.get_or_init(|| Mutex::new(BTreeMap::new()));
    let mut counters = counters.lock().expect("entity name counter poisoned");
    let current = counters.entry(prefix.to_string()).or_insert(0);
    *current = (*current).max(number);
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::generate_entity_id;
    use crate::entities::game_object::GameObject;

    #[test]
    fn deserialized_ids_advance_the_runtime_allocator() {
        let restored_id = generate_entity_id().saturating_add(10_000);
        let restored = GameObject::from_data(
            &json!({
                "type": "GameObject",
                "id": restored_id,
                "name": "Restored_1",
                "components": [],
            }),
            true,
        );

        assert_eq!(restored.id, restored_id);
        assert!(generate_entity_id() > restored_id);

        GameObject::from_data(
            &json!({
                "type": "GameObject",
                "id": u64::MAX,
                "name": "Restored_Max",
                "components": [],
            }),
            true,
        );
        assert_ne!(generate_entity_id(), u64::MAX);
    }
}
