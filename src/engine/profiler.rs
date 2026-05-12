use std::collections::BTreeMap;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct Profiler {
    pub frame_start: Option<Instant>,
    pub frame_time: Duration,
    pub systems: BTreeMap<String, f64>,
    pub counters: BTreeMap<String, usize>,
}

impl Default for Profiler {
    fn default() -> Self {
        Self::new()
    }
}

impl Profiler {
    pub fn new() -> Self {
        Self {
            frame_start: None,
            frame_time: Duration::ZERO,
            systems: BTreeMap::new(),
            counters: BTreeMap::new(),
        }
    }

    pub fn begin_frame(&mut self) {
        self.frame_start = Some(Instant::now());
    }

    pub fn end_frame(&mut self) {
        if let Some(start) = self.frame_start.take() {
            self.frame_time = start.elapsed();
        }
    }

    pub fn record_system(&mut self, name: &str, milliseconds: f64) {
        self.systems.insert(name.to_string(), milliseconds);
    }

    pub fn set_counter(&mut self, name: &str, value: usize) {
        self.counters.insert(name.to_string(), value);
    }

    pub fn rows(&self) -> Vec<(String, String)> {
        self.systems
            .iter()
            .map(|(name, value)| (name.clone(), format!("{value:.1} ms")))
            .collect()
    }
}
