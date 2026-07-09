use std::env;
use std::path::{Path, PathBuf};

use miniforge::engine::development_workflow::{
    BenchmarkOptions, WorkflowKind, inspect_development_environment, inspect_project,
    run_microbenchmarks, run_workflow,
};
use miniforge::engine::{
    asset_database::AssetDatabase,
    asset_tools::AssetTools,
    automation_bridge::AutomationBridge,
    runtime_exporter::{ExportProfile, RuntimeExporter},
    runtime_manifest_loader::RuntimeManifestLoader,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("MiniForge dev error: {error}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let command = args.next().unwrap_or_else(|| "help".to_string());
    let trailing = args.collect::<Vec<_>>();
    let json = trailing.iter().any(|arg| arg == "--json");
    let keep_going = trailing.iter().any(|arg| arg == "--keep-going");
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"));

    match command.as_str() {
        "doctor" => {
            let report = inspect_development_environment(workspace);
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!("MiniForge development doctor");
                println!("  workspace: {}", report.workspace.display());
                println!(
                    "  manifest: {}",
                    if report.manifest_found {
                        "ok"
                    } else {
                        "missing"
                    }
                );
                for tool in &report.tools {
                    let status = if tool.available {
                        tool.version.as_deref().unwrap_or("available")
                    } else if tool.required {
                        "MISSING (required)"
                    } else {
                        "not installed (optional)"
                    };
                    println!("  {:<15} {:<28} {}", tool.name, status, tool.purpose);
                }
            }
            if !report.healthy() {
                std::process::exit(1);
            }
        }
        "project" => {
            let path = first_value(&trailing)
                .unwrap_or_else(|| PathBuf::from("projects").join("DefaultProject"));
            let project = absolute_from(workspace, &path);
            let report = inspect_project(project)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!("Project: {}", report.project.display());
                println!("Valid: {}", report.valid);
                println!("Readiness: {}%", report.readiness_score);
                for error in &report.errors {
                    println!("  error: {error}");
                }
                for warning in &report.warnings {
                    println!("  warning: {warning}");
                }
                for action in &report.next_actions {
                    println!("  next: {action}");
                }
            }
            if !report.valid {
                std::process::exit(1);
            }
        }
        "assets" => {
            let path = first_value(&trailing)
                .unwrap_or_else(|| PathBuf::from("projects").join("DefaultProject"));
            let project = absolute_from(workspace, &path);
            let paths = AssetTools::ensure_project_folders(&project)?;
            let mut database = AssetDatabase::new(&paths.assets, &project)?;
            let before = database.assets.len();
            database.scan()?;
            if json {
                println!(
                    "{}",
                    serde_json::json!({
                        "project": project,
                        "before": before,
                        "after": database.assets.len(),
                        "metadata": database.metadata_file,
                    })
                );
            } else {
                println!(
                    "Asset metadata refreshed: {before} -> {} records ({})",
                    database.assets.len(),
                    database.metadata_file.display()
                );
            }
        }
        "automation" => {
            let path = first_value(&trailing)
                .unwrap_or_else(|| PathBuf::from("projects").join("DefaultProject"));
            let project = absolute_from(workspace, &path);
            let install_python = trailing
                .iter()
                .any(|arg| arg == "--install-python" || arg == "--install-tools");
            let installed = if install_python {
                AutomationBridge::install_python_tools(&project)?
            } else {
                Vec::new()
            };
            let report = AutomationBridge::inspect_project(&project)?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "installed": installed,
                        "report": report,
                    }))?
                );
            } else {
                println!("Automation bridge: {}", report.project_path.display());
                println!("  languages: {}", report.languages.len());
                println!("  python tools: {}", report.python_tools.len());
                println!("  plugins: {}", report.plugins.load_order.len());
                println!(
                    "  render: {:?} ({})",
                    report.render.selection.selected, report.render.selection.reason
                );
                if install_python {
                    println!("  installed python files: {}", installed.len());
                }
                for recommendation in &report.recommendations {
                    println!("  next: {recommendation}");
                }
            }
        }
        "scaffold-csharp-plugin" | "csharp-plugin" => {
            let values = positional_values(&trailing);
            let default_project = PathBuf::from("projects").join("DefaultProject");
            let (project_path, plugin_name) = match values.as_slice() {
                [] => (default_project, "CSharpEditorPlugin".to_string()),
                [name] => (default_project, name.to_string_lossy().to_string()),
                [project, name, ..] => (project.clone(), name.to_string_lossy().to_string()),
            };
            let project = absolute_from(workspace, &project_path);
            let scaffold = AutomationBridge::scaffold_csharp_plugin(&project, &plugin_name)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&scaffold)?);
            } else {
                println!("C# plugin scaffold: {}", scaffold.root.display());
                println!("  manifest: {}", scaffold.manifest_path.display());
                println!("  project: {}", scaffold.project_file.display());
                println!("  written files: {}", scaffold.files_written.len());
            }
        }
        "export" => {
            let values = positional_values(&trailing);
            let path = values
                .first()
                .cloned()
                .unwrap_or_else(|| PathBuf::from("projects").join("DefaultProject"));
            let project = absolute_from(workspace, &path);
            let output = values
                .get(1)
                .map(|path| absolute_from(workspace, path))
                .unwrap_or_else(|| project.join("builds"));
            let profile = values
                .get(2)
                .and_then(|value| value.to_str())
                .map(parse_export_profile)
                .transpose()?
                .unwrap_or(ExportProfile::Debug);
            let report = RuntimeExporter::export_with_profile(&project, &output, profile)?;
            let validated_missing = RuntimeManifestLoader::validate_tree(&report.output_path)?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "report": report,
                        "validated_missing": validated_missing,
                    }))?
                );
            } else {
                println!("Runtime export: {}", report.output_path.display());
                println!("  profile: {}", report.profile.label());
                println!("  copied files: {}", report.copied_files);
                println!("  readiness: {}%", report.readiness_score);
                println!("  missing assets: {}", validated_missing.len());
            }
            if !validated_missing.is_empty() {
                return Err(format!(
                    "runtime export contains missing assets: {}",
                    validated_missing.join(", ")
                )
                .into());
            }
        }
        "bench" | "benchmark" | "benchmarks" => {
            let options = benchmark_options_from_args(&trailing)?;
            let report = run_microbenchmarks(options)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!("MiniForge runtime microbenchmarks");
                println!(
                    "  workload: {} entities, {} spatial queries, {} raycasts, {} script entities x {} frames",
                    report.workload.entity_count,
                    report.workload.spatial_queries,
                    report.workload.raycasts,
                    report.workload.script_entities,
                    report.workload.script_frames
                );
                println!("  total: {:.3} ms", report.total_elapsed_ms);
                for case in &report.cases {
                    println!(
                        "  {:<30} {:>10} ops {:>10.3} ms {:>12.0} ops/s",
                        case.name, case.operations, case.elapsed_ms, case.operations_per_second
                    );
                    for note in &case.notes {
                        println!("      {note}");
                    }
                }
            }
        }
        "quick" | "verify" | "test" | "docs" | "ship" => {
            let kind = match command.as_str() {
                "quick" => WorkflowKind::Quick,
                "verify" => WorkflowKind::Verify,
                "test" => WorkflowKind::Test,
                "docs" => WorkflowKind::Docs,
                "ship" => WorkflowKind::Ship,
                _ => unreachable!(),
            };
            let report = run_workflow(workspace, kind, keep_going, json)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!(
                    "Workflow {}: {} ({} ms)",
                    report.workflow,
                    if report.success { "ok" } else { "failed" },
                    report.elapsed_ms
                );
            }
            if !report.success {
                std::process::exit(1);
            }
        }
        "help" | "--help" | "-h" => print_help(),
        other => {
            print_help();
            return Err(format!("unknown command: {other}").into());
        }
    }
    Ok(())
}

fn first_value(args: &[String]) -> Option<PathBuf> {
    args.iter()
        .find(|arg| !arg.starts_with('-'))
        .map(PathBuf::from)
}

fn positional_values(args: &[String]) -> Vec<PathBuf> {
    args.iter()
        .filter(|arg| !arg.starts_with('-'))
        .map(PathBuf::from)
        .collect()
}

fn benchmark_options_from_args(
    args: &[String],
) -> Result<BenchmarkOptions, Box<dyn std::error::Error>> {
    let defaults = BenchmarkOptions::default();
    Ok(BenchmarkOptions {
        entity_count: flag_usize(args, "--entities", defaults.entity_count)?,
        spatial_queries: flag_usize(args, "--queries", defaults.spatial_queries)?,
        raycasts: flag_usize(args, "--raycasts", defaults.raycasts)?,
        script_entities: flag_usize(args, "--script-entities", defaults.script_entities)?,
        script_frames: flag_usize(args, "--script-frames", defaults.script_frames)?,
    }
    .normalized())
}

fn flag_usize(
    args: &[String],
    flag: &str,
    default: usize,
) -> Result<usize, Box<dyn std::error::Error>> {
    let inline_prefix = format!("{flag}=");
    for (index, arg) in args.iter().enumerate() {
        if arg == flag {
            let value = args
                .get(index + 1)
                .ok_or_else(|| format!("missing value for {flag}"))?;
            return parse_positive_usize(flag, value);
        }
        if let Some(value) = arg.strip_prefix(&inline_prefix) {
            return parse_positive_usize(flag, value);
        }
    }
    Ok(default)
}

fn parse_positive_usize(flag: &str, value: &str) -> Result<usize, Box<dyn std::error::Error>> {
    let parsed = value.parse::<usize>()?;
    if parsed == 0 {
        return Err(format!("{flag} must be greater than zero").into());
    }
    Ok(parsed)
}

fn parse_export_profile(value: &str) -> Result<ExportProfile, Box<dyn std::error::Error>> {
    match value.to_ascii_lowercase().as_str() {
        "debug" => Ok(ExportProfile::Debug),
        "release" => Ok(ExportProfile::Release),
        "shipping" => Ok(ExportProfile::Shipping),
        "web_future" | "web-future" => Ok(ExportProfile::WebFuture),
        "macos_app_future" | "macos-app-future" => Ok(ExportProfile::MacosAppFuture),
        other => Err(format!("unknown export profile: {other}").into()),
    }
}

fn absolute_from(workspace: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        workspace.join(path)
    }
}

fn print_help() {
    println!("MiniForge developer workflow");
    println!("  cargo dev -- doctor [--json]");
    println!("  cargo dev -- quick [--keep-going] [--json]");
    println!("  cargo dev -- verify [--keep-going] [--json]");
    println!("  cargo dev -- test|docs|ship [--json]");
    println!("  cargo dev -- project [path] [--json]");
    println!("  cargo dev -- assets [path] [--json]");
    println!("  cargo dev -- automation [path] [--install-python] [--json]");
    println!("  cargo dev -- scaffold-csharp-plugin [project] [PluginName] [--json]");
    println!("  cargo dev -- export [project] [output] [debug|release|shipping] [--json]");
    println!(
        "  cargo dev -- bench [--entities N] [--queries N] [--raycasts N] [--script-entities N] [--script-frames N] [--json]"
    );
}
