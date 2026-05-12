use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

static NEXT_ID: AtomicU64 = AtomicU64::new(1);
static NAME_COUNTERS: OnceLock<Mutex<BTreeMap<String, u64>>> = OnceLock::new();

pub fn generate_entity_id() -> u64 {
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
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
