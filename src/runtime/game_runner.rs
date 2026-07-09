use std::io;
use std::path::Path;

use crate::engine::crash_reporter::{CrashReporter, CrashReporterConfig};
use crate::runtime::engine_runtime::EngineRuntime;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RuntimeRunOptions {
    pub fixed_dt: f64,
    pub steps: usize,
    pub runtime_mode: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeRunReport {
    pub steps: usize,
    pub simulated_seconds: f64,
    pub entity_count: usize,
    pub project_path: String,
}

impl Default for RuntimeRunOptions {
    fn default() -> Self {
        Self {
            fixed_dt: 1.0 / 60.0,
            steps: 1,
            runtime_mode: true,
        }
    }
}

pub fn run(project_path: impl AsRef<Path>) -> io::Result<EngineRuntime> {
    let project_path = project_path.as_ref();
    CrashReporter::install(CrashReporterConfig::for_project(
        project_path,
        "MiniForge Runtime",
    ));
    let mut runtime = EngineRuntime::new(project_path)?;
    runtime.run_headless_once(RuntimeRunOptions::default().fixed_dt);
    Ok(runtime)
}

pub fn run_with_options(
    project_path: impl AsRef<Path>,
    options: RuntimeRunOptions,
) -> io::Result<(EngineRuntime, RuntimeRunReport)> {
    let project_path = project_path.as_ref();
    CrashReporter::install(CrashReporterConfig::for_project(
        project_path,
        "MiniForge Runtime",
    ));
    let mut runtime = EngineRuntime::new(project_path)?;
    let dt = options.fixed_dt.max(0.0001);
    for _ in 0..options.steps.max(1) {
        runtime.run_headless_once(dt);
    }
    let report = RuntimeRunReport {
        steps: options.steps.max(1),
        simulated_seconds: dt * options.steps.max(1) as f64,
        entity_count: runtime.runtime_world.units.len(),
        project_path: project_path.display().to_string(),
    };
    Ok((runtime, report))
}
