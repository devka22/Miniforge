use std::time::Instant;

use crate::engine::profiler::Profiler;

pub trait ScheduledSystem {
    fn name(&self) -> &str;
    fn run_in_editor(&self) -> bool {
        true
    }
    fn run_in_play(&self) -> bool {
        true
    }
    fn update(&mut self, dt: f64);
}

pub struct ScheduledItem {
    pub priority: i32,
    pub system: Box<dyn ScheduledSystem>,
}

#[derive(Default)]
pub struct SystemScheduler {
    pub items: Vec<ScheduledItem>,
}

impl SystemScheduler {
    pub fn register(&mut self, system: Box<dyn ScheduledSystem>, priority: i32) {
        self.items.push(ScheduledItem { priority, system });
        self.items.sort_by_key(|item| item.priority);
    }

    pub fn update(&mut self, dt: f64, mode: &str, profiler: Option<&mut Profiler>) {
        let mut profiler = profiler;
        for item in &mut self.items {
            if mode == "EDITOR" && !item.system.run_in_editor() {
                continue;
            }
            if mode == "PLAY" && !item.system.run_in_play() {
                continue;
            }
            let start = Instant::now();
            item.system.update(dt);
            if let Some(profiler) = profiler.as_deref_mut() {
                profiler.record_system(item.system.name(), start.elapsed().as_secs_f64() * 1000.0);
            }
        }
    }
}
