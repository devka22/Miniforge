use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use walkdir::WalkDir;

use crate::engine::render_2d::{DynamicTextureAtlas2D, TextureAtlasError2D};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PythonToolManifest {
    pub id: String,
    pub label: String,
    pub entry: String,
    pub menu_path: String,
    pub description: String,
    #[serde(default)]
    pub allow_generated_writes: bool,
    #[serde(default)]
    pub trusted: bool,
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct PythonEditorContext {
    pub project_root: String,
    pub active_scene: Option<String>,
    #[serde(default)]
    pub selected_entity_ids: Vec<u64>,
    #[serde(default)]
    pub assets: Vec<String>,
    #[serde(default)]
    pub parameters: Value,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct PythonEditorOperation {
    pub operation: String,
    #[serde(default)]
    pub target: String,
    #[serde(default)]
    pub value: Value,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct PythonToolResult {
    pub success: bool,
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub operations: Vec<PythonEditorOperation>,
    #[serde(default)]
    pub generated_files: Vec<String>,
    #[serde(default)]
    pub stdout: String,
    #[serde(default)]
    pub stderr: String,
    #[serde(default)]
    pub elapsed_ms: u128,
}

#[derive(Debug, Clone)]
pub struct PythonAutomationHost {
    pub project_root: PathBuf,
    pub python_command: String,
}

impl PythonAutomationHost {
    pub fn new(project_root: impl AsRef<Path>) -> Self {
        Self {
            project_root: project_root.as_ref().to_path_buf(),
            python_command: "python3".to_string(),
        }
    }

    pub fn interpreter_version(&self) -> io::Result<String> {
        let output = Command::new(&self.python_command)
            .arg("--version")
            .output()?;
        let text = if output.stdout.is_empty() {
            &output.stderr
        } else {
            &output.stdout
        };
        Ok(String::from_utf8_lossy(text).trim().to_string())
    }

    pub fn install_builtin_tools(&self) -> io::Result<Vec<PathBuf>> {
        let tools = self.project_root.join("tools");
        fs::create_dir_all(&tools)?;
        let builtins = [
            (
                "miniforge_editor_api.py",
                include_str!("../../tools/miniforge_editor_api.py"),
            ),
            (
                "scene_report.py",
                include_str!("../../tools/scene_report.py"),
            ),
            (
                "scene_report.mftool.json",
                include_str!("../../tools/scene_report.mftool.json"),
            ),
            (
                "production_suite.py",
                include_str!("../../tools/production_suite.py"),
            ),
            (
                "batch_asset_import.mftool.json",
                include_str!("../../tools/batch_asset_import.mftool.json"),
            ),
            (
                "sprite_converter.mftool.json",
                include_str!("../../tools/sprite_converter.mftool.json"),
            ),
            (
                "atlas_generator.mftool.json",
                include_str!("../../tools/atlas_generator.mftool.json"),
            ),
            (
                "bulk_properties.mftool.json",
                include_str!("../../tools/bulk_properties.mftool.json"),
            ),
            (
                "procedural_level.mftool.json",
                include_str!("../../tools/procedural_level.mftool.json"),
            ),
            (
                "project_data_export.mftool.json",
                include_str!("../../tools/project_data_export.mftool.json"),
            ),
            (
                "automated_build.mftool.json",
                include_str!("../../tools/automated_build.mftool.json"),
            ),
            (
                "animation_processor.mftool.json",
                include_str!("../../tools/animation_processor.mftool.json"),
            ),
            (
                "documentation_generator.mftool.json",
                include_str!("../../tools/documentation_generator.mftool.json"),
            ),
            (
                "project_health_matrix.py",
                include_str!("../../tools/project_health_matrix.py"),
            ),
            (
                "project_health_matrix.mftool.json",
                include_str!("../../tools/project_health_matrix.mftool.json"),
            ),
        ];
        let mut installed = Vec::new();
        for (name, contents) in builtins {
            let path = tools.join(name);
            if !path.exists() {
                fs::write(&path, contents)?;
            }
            installed.push(path);
        }
        Ok(installed)
    }

    pub fn discover(&self) -> io::Result<Vec<PythonToolManifest>> {
        let root = self.project_root.join("tools");
        if !root.exists() {
            return Ok(Vec::new());
        }
        let mut manifests: Vec<PythonToolManifest> = Vec::new();
        for entry in fs::read_dir(root)? {
            let path = entry?.path();
            if !path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with(".mftool.json"))
            {
                continue;
            }
            let value = serde_json::from_slice::<Value>(&fs::read(path)?)?;
            if let Ok(manifest) = serde_json::from_value(value) {
                manifests.push(manifest);
            }
        }
        manifests.sort_by(|left, right| left.menu_path.cmp(&right.menu_path));
        Ok(manifests)
    }

    pub fn run(
        &self,
        manifest: &PythonToolManifest,
        mut context: PythonEditorContext,
    ) -> io::Result<PythonToolResult> {
        self.validate_manifest(manifest)?;
        let entry = self.resolve_entry(&manifest.entry)?;
        context.project_root = self.project_root.to_string_lossy().to_string();
        let input = serde_json::to_vec(&json!({
            "protocol": "miniforge-editor-tool-v1",
            "tool": {
                "id": manifest.id,
                "label": manifest.label,
                "menu_path": manifest.menu_path,
            },
            "context": context,
            "permissions": {
                "editor_only": true,
                "allow_generated_writes": manifest.allow_generated_writes,
                "network": false,
            }
        }))?;
        let started = Instant::now();
        let mut child = Command::new(&self.python_command)
            .arg("-I")
            .arg(&entry)
            .current_dir(&self.project_root)
            .env("MINIFORGE_EDITOR_TOOL", "1")
            .env("MINIFORGE_NETWORK_ALLOWED", "0")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(&input)?;
        }
        let timeout = Duration::from_millis(manifest.timeout_ms.clamp(100, 120_000));
        loop {
            if child.try_wait()?.is_some() {
                break;
            }
            if started.elapsed() >= timeout {
                child.kill()?;
                let output = child.wait_with_output()?;
                return Ok(PythonToolResult {
                    success: false,
                    message: format!("Python tool timed out after {} ms", timeout.as_millis()),
                    stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                    stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                    elapsed_ms: started.elapsed().as_millis(),
                    ..PythonToolResult::default()
                });
            }
            thread::sleep(Duration::from_millis(10));
        }
        let output = child.wait_with_output()?;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let payload = stdout
            .lines()
            .rev()
            .find(|line| !line.trim().is_empty())
            .and_then(|line| serde_json::from_str::<Value>(line).ok());
        let mut result = payload
            .and_then(|value| serde_json::from_value::<PythonToolResult>(value).ok())
            .unwrap_or_else(|| PythonToolResult {
                success: output.status.success(),
                message: if output.status.success() {
                    "Python tool completed without a protocol result".to_string()
                } else {
                    format!("Python exited with {}", output.status)
                },
                ..PythonToolResult::default()
            });
        result.success &= output.status.success();
        result.stdout = stdout;
        result.stderr = stderr;
        result.elapsed_ms = started.elapsed().as_millis();
        self.validate_result(manifest, &mut result);
        Ok(result)
    }

    fn validate_manifest(&self, manifest: &PythonToolManifest) -> io::Result<()> {
        if !manifest.trusted {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Python editor tool must be explicitly trusted",
            ));
        }
        if manifest.id.trim().is_empty() || manifest.entry.trim().is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Python tool id and entry are required",
            ));
        }
        Ok(())
    }

    fn resolve_entry(&self, entry: &str) -> io::Result<PathBuf> {
        let tools_root = self.project_root.join("tools");
        let path = self.project_root.join(entry);
        if path.extension().and_then(|extension| extension.to_str()) != Some("py") {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "editor automation entry must be a .py file",
            ));
        }
        let canonical_tools = tools_root.canonicalize()?;
        let canonical_entry = path.canonicalize()?;
        if !canonical_entry.starts_with(canonical_tools) {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Python tools must live under project/tools",
            ));
        }
        Ok(canonical_entry)
    }

    fn validate_result(&self, manifest: &PythonToolManifest, result: &mut PythonToolResult) {
        let generated_root = self.project_root.join(".miniforge").join("generated");
        result.generated_files.retain(|path| {
            manifest.allow_generated_writes
                && self
                    .project_root
                    .join(path)
                    .canonicalize()
                    .ok()
                    .is_some_and(|path| path.starts_with(&generated_root))
        });
        let allowed_operations = [
            "log",
            "select_entities",
            "set_editor_property",
            "create_asset_descriptor",
            "request_reimport",
            "open_document",
            "refresh_assets",
            "batch_import_assets",
            "convert_sprites",
            "generate_atlas",
            "create_procedural_level",
            "export_project_data",
            "automate_build",
            "process_animations",
            "generate_documentation",
        ];
        result
            .operations
            .retain(|operation| allowed_operations.contains(&operation.operation.as_str()));
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct PythonBatchReport {
    pub processed: usize,
    pub skipped: usize,
    #[serde(default)]
    pub output_files: Vec<String>,
    #[serde(default)]
    pub warnings: Vec<String>,
}

pub fn batch_import_assets(
    project_root: impl AsRef<Path>,
    source: &str,
    destination: &str,
) -> io::Result<PythonBatchReport> {
    let project_root = project_root.as_ref();
    let source_root = safe_project_path(project_root, source)?;
    let destination_root = safe_project_path(project_root, destination)?;
    let mut report = PythonBatchReport::default();
    if !source_root.exists() {
        report.warnings.push(format!(
            "Import drop does not exist yet: {}",
            source_root.display()
        ));
        return Ok(report);
    }
    let files = WalkDir::new(&source_root)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| entry.into_path())
        .collect::<Vec<_>>();
    for file in files {
        let relative = file.strip_prefix(&source_root).map_err(io::Error::other)?;
        let output = destination_root.join(relative);
        if output.exists() {
            let source_metadata = fs::metadata(&file)?;
            let output_metadata = fs::metadata(&output)?;
            let destination_is_current = source_metadata.len() == output_metadata.len()
                && match (source_metadata.modified(), output_metadata.modified()) {
                    (Ok(source_time), Ok(output_time)) => output_time >= source_time,
                    _ => false,
                };
            if destination_is_current {
                report.skipped += 1;
                continue;
            }
        }
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(&file, &output)?;
        report.processed += 1;
        report
            .output_files
            .push(display_project_path(project_root, &output));
    }
    Ok(report)
}

pub fn batch_convert_sprites(
    project_root: impl AsRef<Path>,
    source: &str,
    destination: &str,
) -> io::Result<PythonBatchReport> {
    let project_root = project_root.as_ref();
    let source_root = safe_project_path(project_root, source)?;
    let destination_root = safe_project_path(project_root, destination)?;
    let mut report = PythonBatchReport::default();
    if !source_root.exists() {
        report.warnings.push(format!(
            "Sprite source does not exist: {}",
            source_root.display()
        ));
        return Ok(report);
    }
    let files = sprite_source_files(&source_root);
    for file in files {
        let extension = file
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        if extension == "png" {
            report.skipped += 1;
            continue;
        }
        let relative = file.strip_prefix(&source_root).map_err(io::Error::other)?;
        let mut output = destination_root.join(relative);
        output.set_extension("png");
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent)?;
        }
        match image::open(&file) {
            Ok(image) => {
                image.save(&output).map_err(io::Error::other)?;
                report.processed += 1;
                report
                    .output_files
                    .push(display_project_path(project_root, &output));
            }
            Err(error) => {
                report.skipped += 1;
                report.warnings.push(format!("{}: {error}", file.display()));
            }
        }
    }
    Ok(report)
}

pub fn generate_paged_sprite_atlases(
    project_root: impl AsRef<Path>,
    source: &str,
    destination: &str,
    size: u32,
    extrude: u32,
) -> io::Result<PythonBatchReport> {
    let project_root = project_root.as_ref();
    let source_root = safe_project_path(project_root, source)?;
    let output_root = safe_project_path(project_root, destination)?;
    fs::create_dir_all(&output_root)?;
    let mut report = PythonBatchReport::default();
    let files = sprite_source_files(&source_root);
    if files.is_empty() {
        report
            .warnings
            .push("No sprite images found for atlas generation".to_string());
        return Ok(report);
    }

    let size = size.clamp(64, 8192);
    let mut page = 0usize;
    let mut atlas = DynamicTextureAtlas2D::new(format!("AutoAtlas_{page:03}"), size, size)
        .map_err(io::Error::other)?;
    for file in files {
        let relative = file.strip_prefix(&source_root).unwrap_or(&file);
        let region_name = relative
            .with_extension("")
            .to_string_lossy()
            .replace(['/', '\\'], "__");
        match atlas.insert_file(region_name.clone(), &file, extrude) {
            Ok(_) => report.processed += 1,
            Err(TextureAtlasError2D::OutOfSpace { .. }) if atlas.stats().region_count > 0 => {
                export_atlas_page(project_root, &output_root, page, &atlas, &mut report)?;
                page += 1;
                atlas = DynamicTextureAtlas2D::new(format!("AutoAtlas_{page:03}"), size, size)
                    .map_err(io::Error::other)?;
                match atlas.insert_file(region_name, &file, extrude) {
                    Ok(_) => report.processed += 1,
                    Err(error) => {
                        report.skipped += 1;
                        report.warnings.push(format!("{}: {error}", file.display()));
                    }
                }
            }
            Err(error) => {
                report.skipped += 1;
                report.warnings.push(format!("{}: {error}", file.display()));
            }
        }
    }
    if atlas.stats().region_count > 0 {
        export_atlas_page(project_root, &output_root, page, &atlas, &mut report)?;
    }
    Ok(report)
}

fn export_atlas_page(
    project_root: &Path,
    output_root: &Path,
    page: usize,
    atlas: &DynamicTextureAtlas2D,
    report: &mut PythonBatchReport,
) -> io::Result<()> {
    let png = output_root.join(format!("AutoAtlas_{page:03}.png"));
    let metadata = output_root.join(format!("AutoAtlas_{page:03}.atlas.json"));
    atlas.export(&png, &metadata).map_err(io::Error::other)?;
    report
        .output_files
        .push(display_project_path(project_root, &png));
    report
        .output_files
        .push(display_project_path(project_root, &metadata));
    Ok(())
}

fn sprite_source_files(root: &Path) -> Vec<PathBuf> {
    if !root.exists() {
        return Vec::new();
    }
    let mut files = WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| entry.into_path())
        .filter(|path| {
            matches!(
                path.extension()
                    .and_then(|value| value.to_str())
                    .unwrap_or_default()
                    .to_ascii_lowercase()
                    .as_str(),
                "png" | "jpg" | "jpeg" | "webp" | "bmp"
            )
        })
        .collect::<Vec<_>>();
    files.sort();
    files
}

fn safe_project_path(project_root: &Path, relative: &str) -> io::Result<PathBuf> {
    let path = Path::new(relative);
    if path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                std::path::Component::ParentDir
                    | std::path::Component::RootDir
                    | std::path::Component::Prefix(_)
            )
        })
    {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "automation paths must stay inside the project",
        ));
    }
    Ok(project_root.join(path))
}

fn display_project_path(project_root: &Path, path: &Path) -> String {
    path.strip_prefix(project_root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

const fn default_timeout_ms() -> u64 {
    15_000
}
