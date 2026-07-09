use std::path::PathBuf;

use miniforge::runtime::game_runner::{RuntimeRunOptions, run_with_options};

fn main() {
    if let Err(error) = run() {
        eprintln!("MiniForge headless runtime error: {error}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let project = args
        .next()
        .map(PathBuf::from)
        .ok_or("usage: miniforge_headless <project> [steps]")?;
    let steps = args
        .next()
        .as_deref()
        .unwrap_or("1")
        .parse::<usize>()?
        .max(1);
    let (runtime, report) = run_with_options(
        &project,
        RuntimeRunOptions {
            steps,
            ..RuntimeRunOptions::default()
        },
    )?;
    let world = runtime.runtime_world.validate();
    println!(
        "{}",
        serde_json::json!({
            "project": report.project_path,
            "steps": report.steps,
            "simulated_seconds": report.simulated_seconds,
            "entities": report.entity_count,
            "world_valid": world.is_valid(),
            "scripts": runtime.luau_script_runtime.last_frame_scripts,
            "script_errors": runtime.luau_script_runtime.last_errors,
            "visual_graphs": runtime.visual_script_runtime.last_frame_graphs,
            "sprite_animations": runtime.sprite_animation_system.last_report.animated_entities,
            "audio_voices": runtime.audio_system.voices.len(),
        })
    );
    if !world.is_valid() || !runtime.luau_script_runtime.last_errors.is_empty() {
        std::process::exit(1);
    }
    Ok(())
}
