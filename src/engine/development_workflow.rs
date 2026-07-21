//! Reproducible developer workflows shared by the local CLI and CI.

use std::fs;
use std::hint::black_box;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use serde::Serialize;
use serde_json::json;

use crate::engine::component::default_component;
use crate::engine::luau_scripting::LuauScriptRuntime;
use crate::engine::project_validator::ProjectValidator;
use crate::engine::system_audit::SystemReadinessReport;
use crate::engine::world::RuntimeWorld;
use crate::entities::game_object::GameObject;
use crate::systems::physics_system::PhysicsSystem;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkflowKind {
    Quick,
    Verify,
    Test,
    Docs,
    Ship,
}

impl WorkflowKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Quick => "quick",
            Self::Verify => "verify",
            Self::Test => "test",
            Self::Docs => "docs",
            Self::Ship => "ship",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowStep {
    pub label: &'static str,
    pub program: &'static str,
    pub args: &'static [&'static str],
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct WorkflowStepReport {
    pub label: String,
    pub success: bool,
    pub elapsed_ms: u128,
    pub exit_code: Option<i32>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct WorkflowReport {
    pub workflow: String,
    pub success: bool,
    pub elapsed_ms: u128,
    pub steps: Vec<WorkflowStepReport>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ToolStatus {
    pub name: String,
    pub required: bool,
    pub available: bool,
    pub version: Option<String>,
    pub purpose: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DevelopmentDoctorReport {
    pub workspace: PathBuf,
    pub manifest_found: bool,
    pub tools: Vec<ToolStatus>,
}

impl DevelopmentDoctorReport {
    pub fn healthy(&self) -> bool {
        self.manifest_found
            && self
                .tools
                .iter()
                .filter(|tool| tool.required)
                .all(|tool| tool.available)
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ProjectHealthReport {
    pub project: PathBuf,
    pub valid: bool,
    pub readiness_score: u8,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub next_actions: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub struct BenchmarkOptions {
    pub entity_count: usize,
    pub spatial_queries: usize,
    pub raycasts: usize,
    pub script_entities: usize,
    pub script_frames: usize,
}

impl Default for BenchmarkOptions {
    fn default() -> Self {
        Self {
            entity_count: 10_000,
            spatial_queries: 2_000,
            raycasts: 1_000,
            script_entities: 100,
            script_frames: 60,
        }
    }
}

impl BenchmarkOptions {
    pub fn normalized(self) -> Self {
        Self {
            entity_count: self.entity_count.max(1),
            spatial_queries: self.spatial_queries.max(1),
            raycasts: self.raycasts.max(1),
            script_entities: self.script_entities.max(1),
            script_frames: self.script_frames.max(1),
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct BenchmarkCaseReport {
    pub name: String,
    pub operations: usize,
    pub elapsed_ms: f64,
    pub operations_per_second: f64,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct BenchmarkReport {
    pub workload: BenchmarkOptions,
    pub total_elapsed_ms: f64,
    pub cases: Vec<BenchmarkCaseReport>,
}

pub fn workflow_plan(kind: WorkflowKind) -> Vec<WorkflowStep> {
    let format = WorkflowStep {
        label: "format",
        program: "cargo",
        args: &["fmt", "--all", "--", "--check"],
    };
    let check = WorkflowStep {
        label: "check",
        program: "cargo",
        args: &["check", "--locked", "--all-targets"],
    };
    let clippy = WorkflowStep {
        label: "clippy",
        program: "cargo",
        args: &[
            "clippy",
            "--locked",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ],
    };
    let all_tests = WorkflowStep {
        label: "tests",
        program: "cargo",
        args: &["test", "--locked", "--all-targets"],
    };
    let lib_tests = WorkflowStep {
        label: "library tests",
        program: "cargo",
        args: &["test", "--locked", "--lib"],
    };
    let docs = WorkflowStep {
        label: "documentation",
        program: "cargo",
        args: &["doc", "--locked", "--no-deps"],
    };
    let ship = WorkflowStep {
        label: "ship runtime",
        program: "cargo",
        args: &[
            "build",
            "--locked",
            "--profile",
            "ship",
            "--bin",
            "miniforge_runtime",
        ],
    };

    match kind {
        WorkflowKind::Quick => vec![format, check, lib_tests],
        WorkflowKind::Verify => vec![format, check, clippy, all_tests, docs],
        WorkflowKind::Test => vec![all_tests],
        WorkflowKind::Docs => vec![docs],
        WorkflowKind::Ship => vec![format, check, clippy, all_tests, ship],
    }
}

pub fn run_workflow(
    workspace: impl AsRef<Path>,
    kind: WorkflowKind,
    keep_going: bool,
    quiet: bool,
) -> io::Result<WorkflowReport> {
    let workspace = workspace.as_ref();
    let workflow_started = Instant::now();
    let mut steps = Vec::new();

    for step in workflow_plan(kind) {
        if !quiet {
            eprintln!("[MiniForge dev] {}...", step.label);
        }
        let started = Instant::now();
        let mut command = Command::new(step.program);
        command.args(step.args).current_dir(workspace);
        if step.label == "documentation" {
            command.env("RUSTDOCFLAGS", "-D warnings");
        }
        if quiet {
            command.stdout(Stdio::null()).stderr(Stdio::null());
        }
        let status = command.status()?;
        let success = status.success();
        steps.push(WorkflowStepReport {
            label: step.label.to_string(),
            success,
            elapsed_ms: started.elapsed().as_millis(),
            exit_code: status.code(),
        });
        if !success && !keep_going {
            break;
        }
    }

    Ok(WorkflowReport {
        workflow: kind.label().to_string(),
        success: steps.len() == workflow_plan(kind).len() && steps.iter().all(|step| step.success),
        elapsed_ms: workflow_started.elapsed().as_millis(),
        steps,
    })
}

pub fn inspect_development_environment(workspace: impl AsRef<Path>) -> DevelopmentDoctorReport {
    let workspace = workspace.as_ref().to_path_buf();
    DevelopmentDoctorReport {
        manifest_found: workspace.join("Cargo.toml").is_file(),
        tools: vec![
            inspect_tool("rustc", true, "rustc", &["--version"], "Rust compiler"),
            inspect_tool(
                "cargo",
                true,
                "cargo",
                &["--version"],
                "Build orchestration",
            ),
            inspect_tool(
                "rustfmt",
                true,
                "cargo",
                &["fmt", "--version"],
                "Deterministic formatting",
            ),
            inspect_tool(
                "clippy",
                true,
                "cargo",
                &["clippy", "--version"],
                "Static analysis",
            ),
            inspect_tool(
                "cargo-nextest",
                false,
                "cargo",
                &["nextest", "--version"],
                "Faster parallel test runner",
            ),
            inspect_tool(
                "cargo-deny",
                false,
                "cargo",
                &["deny", "--version"],
                "Dependency policy and advisory checks",
            ),
        ],
        workspace,
    }
}

pub fn inspect_project(project: impl AsRef<Path>) -> io::Result<ProjectHealthReport> {
    let project = project.as_ref().to_path_buf();
    let mut validator = ProjectValidator::default();
    let valid = validator.validate(&project);
    let audit = SystemReadinessReport::audit_project(&project)?;
    Ok(ProjectHealthReport {
        project,
        valid,
        readiness_score: audit.total_score,
        errors: validator.errors,
        warnings: validator.warnings,
        next_actions: audit.top_actions(8),
    })
}

pub fn run_microbenchmarks(options: BenchmarkOptions) -> io::Result<BenchmarkReport> {
    let options = options.normalized();
    let started = Instant::now();
    let mut cases = Vec::new();

    let (mut world, ids, create_case) = benchmark_entity_creation(options.entity_count)?;
    cases.push(create_case);
    cases.push(benchmark_spatial_queries(&world, options.spatial_queries));
    cases.push(benchmark_entity_removal(&mut world, &ids));
    cases.push(benchmark_physics_raycasts(
        options.entity_count,
        options.raycasts,
    ));
    cases.push(benchmark_luau_updates(
        options.script_entities,
        options.script_frames,
    )?);

    Ok(BenchmarkReport {
        workload: options,
        total_elapsed_ms: elapsed_ms(started),
        cases,
    })
}

fn inspect_tool(
    name: &str,
    required: bool,
    program: &str,
    args: &[&str],
    purpose: &str,
) -> ToolStatus {
    let output = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output();
    let (available, version) = match output {
        Ok(output) if output.status.success() => {
            let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
            (true, (!text.is_empty()).then_some(text))
        }
        _ => (false, None),
    };
    ToolStatus {
        name: name.to_string(),
        required,
        available,
        version,
        purpose: purpose.to_string(),
    }
}

fn benchmark_entity_creation(
    count: usize,
) -> io::Result<(RuntimeWorld, Vec<u64>, BenchmarkCaseReport)> {
    let started = Instant::now();
    let mut world = RuntimeWorld::default();
    let mut ids = Vec::with_capacity(count);
    for index in 0..count {
        let entity = benchmark_entity(index);
        let id = entity.id;
        world.push(entity).map_err(io::Error::other)?;
        ids.push(id);
    }
    let elapsed_ms = elapsed_ms(started);
    Ok((
        world,
        ids,
        case_report(
            "runtime_world_entity_create",
            count,
            elapsed_ms,
            vec![
                "GameObject::new plus RuntimeWorld::push".to_string(),
                "includes indexed duplicate-id lookup and spatial-index insert".to_string(),
            ],
        ),
    ))
}

fn benchmark_spatial_queries(world: &RuntimeWorld, queries: usize) -> BenchmarkCaseReport {
    let started = Instant::now();
    let mut total_hits = 0usize;
    for index in 0..queries {
        let x = ((index * 37) % 128) as f64 * 1.25;
        let y = ((index * 53) % 128) as f64 * 1.25;
        total_hits += world.query_radius(x, y, 6.0, None, None).len();
    }
    black_box(total_hits);
    case_report(
        "runtime_world_spatial_query",
        queries,
        elapsed_ms(started),
        vec![
            "RuntimeWorld::query_radius through the canonical spatial index".to_string(),
            format!("total hits observed: {total_hits}"),
        ],
    )
}

fn benchmark_entity_removal(world: &mut RuntimeWorld, ids: &[u64]) -> BenchmarkCaseReport {
    let started = Instant::now();
    let mut removed = 0usize;
    for id in ids {
        if world.remove_unordered(*id).is_some() {
            removed += 1;
        }
    }
    black_box(world.units.len());
    case_report(
        "runtime_world_entity_remove",
        removed,
        elapsed_ms(started),
        vec![
            "RuntimeWorld::remove_unordered by indexed id".to_string(),
            "use RuntimeWorld::remove when stable render/editor order is required".to_string(),
        ],
    )
}

fn benchmark_physics_raycasts(entity_count: usize, raycasts: usize) -> BenchmarkCaseReport {
    let entities = (0..entity_count).map(benchmark_entity).collect::<Vec<_>>();
    let physics = PhysicsSystem::new();
    let started = Instant::now();
    let mut hits = 0usize;
    for index in 0..raycasts {
        let y = ((index * 17) % 128) as f64 * 1.25;
        if physics
            .raycast(&entities, (-20.0, y), (1.0, 0.0), 260.0)
            .is_some()
        {
            hits += 1;
        }
    }
    black_box(hits);
    case_report(
        "physics2d_raycast",
        raycasts,
        elapsed_ms(started),
        vec![
            "PhysicsSystem::raycast against Collider2D components".to_string(),
            format!("entities scanned per raycast: {entity_count}"),
            format!("hits observed: {hits}"),
        ],
    )
}

fn benchmark_luau_updates(entity_count: usize, frames: usize) -> io::Result<BenchmarkCaseReport> {
    let root = benchmark_temp_dir("luau")?;
    let scripts = root.join("scripts");
    fs::create_dir_all(&scripts)?;
    fs::write(
        scripts.join("Bench.luau"),
        r#"
local Bench = {}

function Bench:on_update(dt)
    self.entity.transform.position.x += dt
    self.ticks = (self.ticks or 0) + 1
end

return Bench
"#,
    )?;

    let mut entities = (0..entity_count)
        .map(|index| {
            let mut entity = benchmark_entity(index);
            let mut script = default_component("ScriptComponent").expect("ScriptComponent");
            script.set("path", json!("Bench.luau"));
            entity.add_component(script);
            entity
        })
        .collect::<Vec<_>>();
    let mut runtime = LuauScriptRuntime::new(&root);

    let warmup = runtime.update_entities(&mut entities, 1.0 / 60.0, "PLAY");
    if !warmup.errors.is_empty() {
        let _ = fs::remove_dir_all(&root);
        return Err(io::Error::other(warmup.errors.join("; ")));
    }

    let started = Instant::now();
    let mut scripts_run = 0usize;
    for _ in 0..frames {
        let report = runtime.update_entities(&mut entities, 1.0 / 60.0, "PLAY");
        if !report.errors.is_empty() {
            let _ = fs::remove_dir_all(&root);
            return Err(io::Error::other(report.errors.join("; ")));
        }
        scripts_run += report.scripts_run;
    }
    let elapsed = elapsed_ms(started);
    black_box(entities.first().map(|entity| entity.x).unwrap_or_default());
    let _ = fs::remove_dir_all(&root);

    Ok(case_report(
        "luau_cached_update",
        scripts_run,
        elapsed,
        vec![
            format!("{entity_count} entities over {frames} frames"),
            "one warm-up frame excluded so this measures cached script dispatch".to_string(),
        ],
    ))
}

fn benchmark_entity(index: usize) -> GameObject {
    let mut entity = GameObject::new(
        (index % 128) as f64 * 1.25,
        (index / 128) as f64 * 1.25,
        Some(format!("BenchEntity{index}")),
    );
    entity.radius = 0.45;
    entity.width = 1.0;
    entity.height = 1.0;
    entity.sync_to_components();
    entity
}

fn benchmark_temp_dir(label: &str) -> io::Result<PathBuf> {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let path = std::env::temp_dir().join(format!("miniforge-bench-{label}-{stamp}"));
    fs::create_dir_all(&path)?;
    Ok(path)
}

fn case_report(
    name: &str,
    operations: usize,
    elapsed_ms: f64,
    notes: Vec<String>,
) -> BenchmarkCaseReport {
    let seconds = (elapsed_ms / 1000.0).max(f64::EPSILON);
    BenchmarkCaseReport {
        name: name.to_string(),
        operations,
        elapsed_ms,
        operations_per_second: operations as f64 / seconds,
        notes,
    }
}

fn elapsed_ms(started: Instant) -> f64 {
    started.elapsed().as_secs_f64() * 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quick_workflow_keeps_feedback_loop_small() {
        let labels = workflow_plan(WorkflowKind::Quick)
            .into_iter()
            .map(|step| step.label)
            .collect::<Vec<_>>();
        assert_eq!(labels, vec!["format", "check", "library tests"]);
    }

    #[test]
    fn verify_workflow_matches_ci_quality_gates() {
        let labels = workflow_plan(WorkflowKind::Verify)
            .into_iter()
            .map(|step| step.label)
            .collect::<Vec<_>>();
        assert_eq!(
            labels,
            vec!["format", "check", "clippy", "tests", "documentation"]
        );
    }

    #[test]
    fn microbenchmarks_cover_runtime_physics_and_luau() {
        let report = run_microbenchmarks(BenchmarkOptions {
            entity_count: 8,
            spatial_queries: 4,
            raycasts: 3,
            script_entities: 2,
            script_frames: 2,
        })
        .expect("benchmark report");
        let names = report
            .cases
            .iter()
            .map(|case| case.name.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            names,
            vec![
                "runtime_world_entity_create",
                "runtime_world_spatial_query",
                "runtime_world_entity_remove",
                "physics2d_raycast",
                "luau_cached_update",
            ]
        );
        assert!(report.cases.iter().all(|case| case.operations > 0));
    }
}
