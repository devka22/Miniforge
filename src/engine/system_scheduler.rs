use std::collections::{BTreeMap, BTreeSet};
use std::time::Instant;

use crate::engine::profiler::Profiler;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum SystemPhase {
    Fixed,
    #[default]
    Update,
    Late,
}

impl SystemPhase {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Fixed => "Fixed",
            Self::Update => "Update",
            Self::Late => "Late",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SchedulerBudgetPolicy {
    /// Preserve gameplay determinism and only report budget overruns.
    #[default]
    WarnOnly,
    /// Once the frame budget is consumed, defer non-critical systems.
    SkipOptional,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SystemSchedule {
    pub phase: SystemPhase,
    pub priority: i32,
    pub enabled: bool,
    pub critical: bool,
    pub budget_ms: Option<f64>,
    pub after: Vec<String>,
}

impl SystemSchedule {
    pub fn new(priority: i32) -> Self {
        Self {
            priority,
            ..Self::default()
        }
    }

    pub fn in_phase(mut self, phase: SystemPhase) -> Self {
        self.phase = phase;
        self
    }

    pub fn critical(mut self, critical: bool) -> Self {
        self.critical = critical;
        self
    }

    pub fn with_budget_ms(mut self, budget_ms: f64) -> Self {
        self.budget_ms = (budget_ms.is_finite() && budget_ms > 0.0).then_some(budget_ms);
        self
    }

    pub fn after(mut self, dependency: impl Into<String>) -> Self {
        let dependency = dependency.into();
        if !dependency.trim().is_empty() && !self.after.contains(&dependency) {
            self.after.push(dependency);
        }
        self
    }
}

impl Default for SystemSchedule {
    fn default() -> Self {
        Self {
            phase: SystemPhase::Update,
            priority: 0,
            enabled: true,
            critical: false,
            budget_ms: None,
            after: Vec::new(),
        }
    }
}

pub trait ScheduledSystem {
    fn name(&self) -> &str;

    fn phase(&self) -> SystemPhase {
        SystemPhase::Update
    }

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
    pub phase: SystemPhase,
    pub enabled: bool,
    pub critical: bool,
    pub budget_ms: Option<f64>,
    pub after: Vec<String>,
    pub system: Box<dyn ScheduledSystem>,
    registration_order: u64,
}

impl ScheduledItem {
    fn sort_key(&self) -> (SystemPhase, i32, u64) {
        (self.phase, self.priority, self.registration_order)
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SystemRunSample {
    pub name: String,
    pub phase: SystemPhase,
    pub milliseconds: f64,
    pub skipped: bool,
    pub skip_reason: Option<String>,
    pub budget_ms: Option<f64>,
    pub budget_exceeded: bool,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SchedulerFrameReport {
    pub mode: String,
    pub phase: Option<SystemPhase>,
    pub total_ms: f64,
    pub budget_ms: f64,
    pub ran: usize,
    pub skipped: usize,
    pub over_budget: usize,
    pub samples: Vec<SystemRunSample>,
    pub warnings: Vec<String>,
}

#[derive(Default)]
pub struct SystemScheduler {
    pub items: Vec<ScheduledItem>,
    pub budget_ms: f64,
    pub budget_policy: SchedulerBudgetPolicy,
}

impl SystemScheduler {
    pub fn register(&mut self, system: Box<dyn ScheduledSystem>, priority: i32) {
        let phase = system.phase();
        self.register_configured(system, SystemSchedule::new(priority).in_phase(phase));
    }

    pub fn register_configured(
        &mut self,
        system: Box<dyn ScheduledSystem>,
        schedule: SystemSchedule,
    ) {
        let registration_order = self
            .items
            .iter()
            .map(|item| item.registration_order)
            .max()
            .map_or(0, |order| order.saturating_add(1));
        self.items.push(ScheduledItem {
            priority: schedule.priority,
            phase: schedule.phase,
            enabled: schedule.enabled,
            critical: schedule.critical,
            budget_ms: schedule
                .budget_ms
                .filter(|budget| budget.is_finite() && *budget > 0.0),
            after: normalized_dependencies(schedule.after),
            system,
            registration_order,
        });
        self.items.sort_by_key(ScheduledItem::sort_key);
    }

    pub fn try_register_configured(
        &mut self,
        system: Box<dyn ScheduledSystem>,
        schedule: SystemSchedule,
    ) -> Result<(), String> {
        let name = system.name().trim();
        if name.is_empty() {
            return Err("Scheduled system name cannot be empty".to_string());
        }
        if self.items.iter().any(|item| item.system.name() == name) {
            return Err(format!("Scheduled system already registered: {name}"));
        }
        self.register_configured(system, schedule);
        Ok(())
    }

    pub fn set_enabled(&mut self, name: &str, enabled: bool) -> bool {
        let Some(item) = self
            .items
            .iter_mut()
            .find(|item| item.system.name() == name)
        else {
            return false;
        };
        item.enabled = enabled;
        true
    }

    pub fn set_system_budget_ms(&mut self, name: &str, budget_ms: Option<f64>) -> bool {
        let Some(item) = self
            .items
            .iter_mut()
            .find(|item| item.system.name() == name)
        else {
            return false;
        };
        item.budget_ms = budget_ms.filter(|budget| budget.is_finite() && *budget > 0.0);
        true
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
        self.run(dt, mode, None, profiler)
    }

    pub fn update_phase(
        &mut self,
        phase: SystemPhase,
        dt: f64,
        mode: &str,
        profiler: Option<&mut Profiler>,
    ) {
        let _ = self.update_phase_with_report(phase, dt, mode, profiler);
    }

    pub fn update_phase_with_report(
        &mut self,
        phase: SystemPhase,
        dt: f64,
        mode: &str,
        profiler: Option<&mut Profiler>,
    ) -> SchedulerFrameReport {
        self.run(dt, mode, Some(phase), profiler)
    }

    fn run(
        &mut self,
        dt: f64,
        mode: &str,
        phase: Option<SystemPhase>,
        profiler: Option<&mut Profiler>,
    ) -> SchedulerFrameReport {
        let budget_ms = self.effective_budget_ms();
        let (order, invalid, mut warnings) = self.execution_plan(phase);
        let mut profiler = profiler;
        let mut outcomes = BTreeMap::<String, bool>::new();
        let mut report = SchedulerFrameReport {
            mode: mode.to_string(),
            phase,
            budget_ms,
            ..Default::default()
        };

        for index in order {
            let item = &mut self.items[index];
            let name = item.system.name().to_string();
            let mut skip_reason = invalid.get(&index).cloned();
            if skip_reason.is_none() && !item.enabled {
                skip_reason = Some("disabled".to_string());
            }
            if skip_reason.is_none() && mode == "EDITOR" && !item.system.run_in_editor() {
                skip_reason = Some("disabled in editor mode".to_string());
            }
            if skip_reason.is_none() && mode == "PLAY" && !item.system.run_in_play() {
                skip_reason = Some("disabled in play mode".to_string());
            }
            if skip_reason.is_none() {
                skip_reason = item.after.iter().find_map(|dependency| {
                    outcomes
                        .get(dependency)
                        .is_some_and(|ran| !ran)
                        .then(|| format!("dependency did not run: {dependency}"))
                });
            }
            if skip_reason.is_none()
                && self.budget_policy == SchedulerBudgetPolicy::SkipOptional
                && !item.critical
                && report.total_ms >= budget_ms
            {
                skip_reason = Some(format!("frame budget exhausted: {budget_ms:.2}ms"));
            }

            if let Some(reason) = skip_reason {
                outcomes.insert(name.clone(), false);
                report.skipped += 1;
                report.samples.push(SystemRunSample {
                    name,
                    phase: item.phase,
                    milliseconds: 0.0,
                    skipped: true,
                    skip_reason: Some(reason),
                    budget_ms: item.budget_ms,
                    budget_exceeded: false,
                });
                continue;
            }

            let start = Instant::now();
            item.system.update(dt);
            let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
            let budget_exceeded = item
                .budget_ms
                .is_some_and(|system_budget| elapsed_ms > system_budget);
            report.total_ms += elapsed_ms;
            report.ran += 1;
            if budget_exceeded {
                report.over_budget += 1;
                warnings.push(format!(
                    "System {name} exceeded its budget: {elapsed_ms:.2}ms > {:.2}ms",
                    item.budget_ms.unwrap_or_default()
                ));
            }
            report.samples.push(SystemRunSample {
                name: name.clone(),
                phase: item.phase,
                milliseconds: elapsed_ms,
                skipped: false,
                skip_reason: None,
                budget_ms: item.budget_ms,
                budget_exceeded,
            });
            outcomes.insert(name, true);
            if let Some(profiler) = profiler.as_deref_mut() {
                profiler.record_system(item.system.name(), elapsed_ms);
            }
        }

        if report.total_ms > budget_ms {
            warnings.push(format!(
                "Scheduler budget exceeded: {:.2}ms > {:.2}ms",
                report.total_ms, budget_ms
            ));
        }
        report.warnings = warnings;
        report
    }

    fn effective_budget_ms(&self) -> f64 {
        if self.budget_ms.is_finite() && self.budget_ms > 0.0 {
            self.budget_ms
        } else {
            16.67
        }
    }

    fn execution_plan(
        &self,
        phase_filter: Option<SystemPhase>,
    ) -> (Vec<usize>, BTreeMap<usize, String>, Vec<String>) {
        let active = self
            .items
            .iter()
            .enumerate()
            .filter_map(|(index, item)| {
                phase_filter
                    .is_none_or(|phase| phase == item.phase)
                    .then_some(index)
            })
            .collect::<Vec<_>>();
        let active_set = active.iter().copied().collect::<BTreeSet<_>>();
        let mut names = BTreeMap::<String, Vec<usize>>::new();
        for (index, item) in self.items.iter().enumerate() {
            names
                .entry(item.system.name().to_string())
                .or_default()
                .push(index);
        }

        let mut invalid = BTreeMap::<usize, String>::new();
        let mut warnings = Vec::new();
        for (name, indices) in &names {
            if name.trim().is_empty() {
                for index in indices {
                    invalid.insert(*index, "system name is empty".to_string());
                }
                warnings.push("Scheduler contains a system with an empty name".to_string());
            } else if indices.len() > 1 {
                for index in indices {
                    invalid.insert(*index, format!("duplicate system name: {name}"));
                }
                warnings.push(format!("Duplicate scheduled system name: {name}"));
            }
        }

        let mut indegree = active
            .iter()
            .map(|index| (*index, 0usize))
            .collect::<BTreeMap<_, _>>();
        let mut outgoing = BTreeMap::<usize, Vec<usize>>::new();
        for index in &active {
            let item = &self.items[*index];
            for dependency in &item.after {
                let Some(dependency_indices) = names.get(dependency) else {
                    invalid.insert(*index, format!("missing dependency: {dependency}"));
                    warnings.push(format!(
                        "System {} depends on missing system {dependency}",
                        item.system.name()
                    ));
                    continue;
                };
                if dependency_indices.len() != 1 {
                    invalid.insert(*index, format!("ambiguous dependency: {dependency}"));
                    continue;
                }
                let dependency_index = dependency_indices[0];
                let dependency_item = &self.items[dependency_index];
                if dependency_item.phase > item.phase {
                    invalid.insert(
                        *index,
                        format!(
                            "dependency {dependency} runs in later phase {}",
                            dependency_item.phase.label()
                        ),
                    );
                    warnings.push(format!(
                        "System {} ({}) cannot depend on {dependency} ({})",
                        item.system.name(),
                        item.phase.label(),
                        dependency_item.phase.label()
                    ));
                    continue;
                }
                if !active_set.contains(&dependency_index) {
                    // A phase-specific pass treats dependencies from earlier phases as satisfied.
                    continue;
                }
                outgoing.entry(dependency_index).or_default().push(*index);
                *indegree.entry(*index).or_default() += 1;
            }
        }

        let mut ready = active
            .iter()
            .copied()
            .filter(|index| indegree.get(index).copied().unwrap_or_default() == 0)
            .collect::<Vec<_>>();
        ready.sort_by_key(|index| self.items[*index].sort_key());
        let mut order = Vec::with_capacity(active.len());
        while !ready.is_empty() {
            let index = ready.remove(0);
            order.push(index);
            if let Some(dependents) = outgoing.get(&index) {
                for dependent in dependents {
                    let entry = indegree.entry(*dependent).or_default();
                    *entry = entry.saturating_sub(1);
                    if *entry == 0 {
                        ready.push(*dependent);
                    }
                }
                ready.sort_by_key(|candidate| self.items[*candidate].sort_key());
            }
        }

        if order.len() != active.len() {
            let ordered = order.iter().copied().collect::<BTreeSet<_>>();
            let mut cyclic = active
                .iter()
                .copied()
                .filter(|index| !ordered.contains(index))
                .collect::<Vec<_>>();
            cyclic.sort_by_key(|index| self.items[*index].sort_key());
            for index in &cyclic {
                invalid.insert(*index, "dependency cycle".to_string());
            }
            let names = cyclic
                .iter()
                .map(|index| self.items[*index].system.name())
                .collect::<Vec<_>>()
                .join(", ");
            warnings.push(format!("Scheduler dependency cycle: {names}"));
            order.extend(cyclic);
        }

        (order, invalid, warnings)
    }
}

fn normalized_dependencies(dependencies: Vec<String>) -> Vec<String> {
    let mut dependencies = dependencies
        .into_iter()
        .filter(|dependency| !dependency.trim().is_empty())
        .collect::<Vec<_>>();
    dependencies.sort();
    dependencies.dedup();
    dependencies
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use super::*;

    struct RecordingSystem {
        name: String,
        phase: SystemPhase,
        editor: bool,
        play: bool,
        work: Duration,
        log: Arc<Mutex<Vec<String>>>,
    }

    impl RecordingSystem {
        fn new(name: &str, phase: SystemPhase, log: &Arc<Mutex<Vec<String>>>) -> Self {
            Self {
                name: name.to_string(),
                phase,
                editor: true,
                play: true,
                work: Duration::ZERO,
                log: Arc::clone(log),
            }
        }

        fn editor_only(mut self) -> Self {
            self.play = false;
            self
        }

        fn with_work(mut self, work: Duration) -> Self {
            self.work = work;
            self
        }
    }

    impl ScheduledSystem for RecordingSystem {
        fn name(&self) -> &str {
            &self.name
        }

        fn phase(&self) -> SystemPhase {
            self.phase
        }

        fn run_in_editor(&self) -> bool {
            self.editor
        }

        fn run_in_play(&self) -> bool {
            self.play
        }

        fn update(&mut self, _dt: f64) {
            let start = Instant::now();
            while start.elapsed() < self.work {
                std::hint::spin_loop();
            }
            self.log.lock().unwrap().push(self.name.clone());
        }
    }

    #[test]
    fn phases_and_dependencies_produce_deterministic_order() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let mut scheduler = SystemScheduler::default();
        scheduler.register_configured(
            Box::new(RecordingSystem::new("Late", SystemPhase::Late, &log)),
            SystemSchedule::new(-50).in_phase(SystemPhase::Late),
        );
        scheduler.register_configured(
            Box::new(RecordingSystem::new("Gameplay", SystemPhase::Update, &log)),
            SystemSchedule::new(-100).after("Input"),
        );
        scheduler.register_configured(
            Box::new(RecordingSystem::new("Input", SystemPhase::Update, &log)),
            SystemSchedule::new(100),
        );

        let report = scheduler.update_with_report(1.0 / 60.0, "PLAY", None);

        assert_eq!(
            log.lock().unwrap().as_slice(),
            &[
                "Input".to_string(),
                "Gameplay".to_string(),
                "Late".to_string()
            ]
        );
        assert_eq!(report.ran, 3);
        assert_eq!(report.skipped, 0);
        assert_eq!(report.samples[2].phase, SystemPhase::Late);
    }

    #[test]
    fn missing_dependencies_and_mode_skips_propagate() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let mut scheduler = SystemScheduler::default();
        scheduler.register_configured(
            Box::new(RecordingSystem::new("EditorInput", SystemPhase::Update, &log).editor_only()),
            SystemSchedule::new(0),
        );
        scheduler.register_configured(
            Box::new(RecordingSystem::new("Tools", SystemPhase::Update, &log)),
            SystemSchedule::new(1).after("EditorInput"),
        );
        scheduler.register_configured(
            Box::new(RecordingSystem::new("Broken", SystemPhase::Late, &log)),
            SystemSchedule::new(0)
                .in_phase(SystemPhase::Late)
                .after("Missing"),
        );

        let report = scheduler.update_with_report(0.016, "PLAY", None);

        assert!(log.lock().unwrap().is_empty());
        assert_eq!(report.skipped, 3);
        assert!(
            report.samples[1]
                .skip_reason
                .as_deref()
                .is_some_and(|reason| reason.contains("dependency did not run"))
        );
        assert!(
            report
                .warnings
                .iter()
                .any(|warning| warning.contains("missing system Missing"))
        );
    }

    #[test]
    fn optional_systems_can_be_deferred_after_budget_is_consumed() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let mut scheduler = SystemScheduler {
            budget_ms: 0.1,
            budget_policy: SchedulerBudgetPolicy::SkipOptional,
            ..Default::default()
        };
        scheduler.register_configured(
            Box::new(
                RecordingSystem::new("Critical", SystemPhase::Update, &log)
                    .with_work(Duration::from_millis(1)),
            ),
            SystemSchedule::new(0).critical(true),
        );
        scheduler.register_configured(
            Box::new(RecordingSystem::new("Cosmetics", SystemPhase::Update, &log)),
            SystemSchedule::new(1),
        );

        let report = scheduler.update_with_report(0.016, "PLAY", None);

        assert_eq!(log.lock().unwrap().as_slice(), &["Critical".to_string()]);
        assert_eq!(report.ran, 1);
        assert_eq!(report.skipped, 1);
        assert!(
            report.samples[1]
                .skip_reason
                .as_deref()
                .is_some_and(|reason| reason.contains("frame budget exhausted"))
        );
    }

    #[test]
    fn phase_pass_runs_only_requested_phase() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let mut scheduler = SystemScheduler::default();
        scheduler.register(
            Box::new(RecordingSystem::new("Fixed", SystemPhase::Fixed, &log)),
            0,
        );
        scheduler.register(
            Box::new(RecordingSystem::new("Update", SystemPhase::Update, &log)),
            0,
        );

        let report = scheduler.update_phase_with_report(SystemPhase::Fixed, 0.02, "PLAY", None);

        assert_eq!(log.lock().unwrap().as_slice(), &["Fixed".to_string()]);
        assert_eq!(report.phase, Some(SystemPhase::Fixed));
        assert_eq!(report.samples.len(), 1);
    }

    #[test]
    fn checked_registration_rejects_duplicate_names() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let mut scheduler = SystemScheduler::default();
        scheduler
            .try_register_configured(
                Box::new(RecordingSystem::new("Physics", SystemPhase::Fixed, &log)),
                SystemSchedule::default(),
            )
            .unwrap();

        let error = scheduler
            .try_register_configured(
                Box::new(RecordingSystem::new("Physics", SystemPhase::Fixed, &log)),
                SystemSchedule::default(),
            )
            .unwrap_err();

        assert!(error.contains("already registered"));
    }
}
