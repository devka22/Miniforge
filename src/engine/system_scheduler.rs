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

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SystemRunSample {
    pub name: String,
    pub milliseconds: f64,
    pub skipped: bool,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SchedulerFrameReport {
    pub mode: String,
    pub total_ms: f64,
    pub ran: usize,
    pub skipped: usize,
    pub samples: Vec<SystemRunSample>,
    pub warnings: Vec<String>,
}

#[derive(Default)]
pub struct SystemScheduler {
    pub items: Vec<ScheduledItem>,
    pub budget_ms: f64,
}

impl SystemScheduler {
    pub fn register(&mut self, system: Box<dyn ScheduledSystem>, priority: i32) {
        self.items.push(ScheduledItem { priority, system });
        self.items.sort_by_key(|item| item.priority);
    }

    pub fn update(&mut self, dt: f64, mode: &str, profiler: Option<&mut Profiler>) {
        let _ = self.update_with_report(dt, mode, profiler);
    }

    pub fn update_with_report(
        &mut self,
        dt: f64,
        mode: &str,
        profiler: Option<&mut Profiler>,
    ) -> SchedulerFrameReport {
        let mut profiler = profiler;
        let mut report = SchedulerFrameReport {
            mode: mode.to_string(),
            ..Default::default()
        };
        for item in &mut self.items {
            if mode == "EDITOR" && !item.system.run_in_editor() {
                report.skipped += 1;
                report.samples.push(SystemRunSample {
                    name: item.system.name().to_string(),
                    milliseconds: 0.0,
                    skipped: true,
                });
                continue;
            }
            if mode == "PLAY" && !item.system.run_in_play() {
                report.skipped += 1;
                report.samples.push(SystemRunSample {
                    name: item.system.name().to_string(),
                    milliseconds: 0.0,
                    skipped: true,
                });
                continue;
            }
            let start = Instant::now();
            item.system.update(dt);
            let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
            report.total_ms += elapsed_ms;
            report.ran += 1;
            report.samples.push(SystemRunSample {
                name: item.system.name().to_string(),
                milliseconds: elapsed_ms,
                skipped: false,
            });
            if let Some(profiler) = profiler.as_deref_mut() {
                profiler.record_system(item.system.name(), elapsed_ms);
            }
        }
        let budget = if self.budget_ms > 0.0 {
            self.budget_ms
        } else {
            16.67
        };
        if report.total_ms > budget {
            report.warnings.push(format!(
                "Scheduler budget exceeded: {:.2}ms > {:.2}ms",
                report.total_ms, budget
            ));
        }
        report
    }
}
