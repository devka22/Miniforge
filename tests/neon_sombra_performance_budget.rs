use std::path::PathBuf;

use miniforge::runtime::EngineRuntime;

fn project_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("projects/NewProject_5")
}

#[test]
fn neon_sombra_uses_script_scheduler_budget_after_warmup() {
    let mut runtime = EngineRuntime::new(project_path()).expect("Neon Sombra runtime");
    for _ in 0..8 {
        runtime.run_headless_once(1.0 / 60.0);
    }

    let mut samples = Vec::new();
    for _ in 0..30 {
        runtime.run_headless_once(1.0 / 60.0);
        samples.push(runtime.luau_script_runtime.last_frame_scripts);
    }

    let max_scripts = samples.iter().copied().max().unwrap_or_default();
    let average_scripts = samples.iter().sum::<usize>() as f64 / samples.len() as f64;
    assert!(
        max_scripts <= 64,
        "expected scheduler to cap update scripts, samples={samples:?}"
    );
    assert!(
        average_scripts <= 38.0,
        "expected staggered AI updates, average={average_scripts:.2}, samples={samples:?}"
    );
    assert!(
        runtime.luau_script_runtime.last_errors.is_empty(),
        "{:?}",
        runtime.luau_script_runtime.last_errors
    );
}
