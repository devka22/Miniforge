//! Assembles a distributable folder from an exported runtime build (no nested self-copy).

use std::env;
use std::fs;
use std::io;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::engine::asset_tools::AssetTools;
use crate::engine::runtime_exporter::{ExportProfile, RuntimeExportReport, RuntimeExporter};

#[derive(Debug, Clone)]
pub struct PackageReport {
    pub destination: PathBuf,
    pub export: RuntimeExportReport,
    pub runtime_binary_copied: Option<PathBuf>,
    pub launcher_scripts: Vec<PathBuf>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum InstallerPlatform {
    Macos,
    Windows,
    Linux,
}

impl InstallerPlatform {
    pub fn current() -> Self {
        if cfg!(windows) {
            Self::Windows
        } else if cfg!(target_os = "macos") {
            Self::Macos
        } else {
            Self::Linux
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Macos => "macos",
            Self::Windows => "windows",
            Self::Linux => "linux",
        }
    }

    pub fn installer_extension(self) -> &'static str {
        match self {
            Self::Macos => "dmg",
            Self::Windows => "msi",
            Self::Linux => "AppImage",
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct InstallerSigningConfig {
    pub identity: Option<String>,
    pub team_id: Option<String>,
    pub certificate_path: Option<PathBuf>,
    pub timestamp_url: Option<String>,
    pub notarize: bool,
}

impl InstallerSigningConfig {
    pub fn ready_for_platform(&self, platform: InstallerPlatform) -> bool {
        match platform {
            InstallerPlatform::Macos => self.identity.is_some(),
            InstallerPlatform::Windows => {
                self.identity.is_some() || self.certificate_path.is_some()
            }
            InstallerPlatform::Linux => self.identity.is_some() || self.certificate_path.is_some(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InstallerPlan {
    pub platform: InstallerPlatform,
    pub package_dir: PathBuf,
    pub installer_name: String,
    pub installer_path: PathBuf,
    pub signing_ready: bool,
    pub signing_identity: Option<String>,
    pub commands: Vec<String>,
    pub artifacts: Vec<PathBuf>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Default)]
pub struct PackagingManager;

impl PackagingManager {
    /// Resolves the runtime executable using the same policy as distributable
    /// packaging (`MINIFORGE_RUNTIME`, sibling binary, then Cargo targets).
    pub fn runtime_binary() -> Option<PathBuf> {
        find_runtime_binary()
    }

    /// Resolves the native wgpu project preview without coupling the editor to
    /// a Cargo invocation.
    pub fn wgpu_preview_binary() -> Option<PathBuf> {
        find_wgpu_preview_binary()
    }

    pub fn package_project(
        project_path: &Path,
        destination: &Path,
        profile: ExportProfile,
    ) -> io::Result<PackageReport> {
        let work_root = project_path.join("exports").join("_pkg_work");
        if work_root.exists() {
            fs::remove_dir_all(&work_root)?;
        }
        let export = RuntimeExporter::export_with_profile(project_path, &work_root, profile)?;
        if destination.exists() {
            fs::remove_dir_all(destination)?;
        }
        fs::create_dir_all(destination)?;
        copy_export_tree(&export.output_path, destination)?;
        let mut warnings: Vec<String> = export
            .missing_assets
            .iter()
            .map(|s| format!("Asset faltante: {s}"))
            .collect();
        let runtime_binary_copied = match find_runtime_binary() {
            Some(src) => {
                let name = src
                    .file_name()
                    .map(|n| n.to_owned())
                    .unwrap_or_else(|| std::ffi::OsString::from(runtime_binary_name()));
                let dst = destination.join(name);
                fs::copy(&src, &dst)?;
                Some(dst)
            }
            None => {
                warnings.push(
                    "No se encontro miniforge_runtime junto al editor ni en target/debug|release. Construye el runtime una vez o define MINIFORGE_RUNTIME para incluir el ejecutable.".into(),
                );
                None
            }
        };
        let launcher_scripts = if let Some(binary) = runtime_binary_copied.as_ref() {
            write_launcher_scripts(destination, binary)?
        } else {
            warnings.push(
                "Paquete creado sin ejecutable: aun puede abrirse con miniforge_runtime --build <carpeta>."
                    .into(),
            );
            Vec::new()
        };
        write_standalone_manifest(
            destination,
            runtime_binary_copied.as_ref(),
            &launcher_scripts,
            &warnings,
        )?;
        let _ = fs::remove_dir_all(&work_root);
        Ok(PackageReport {
            destination: destination.to_path_buf(),
            export,
            runtime_binary_copied,
            launcher_scripts,
            warnings,
        })
    }

    pub fn package_project_with_installer_plan(
        project_path: &Path,
        destination: &Path,
        profile: ExportProfile,
        platform: InstallerPlatform,
        signing: InstallerSigningConfig,
    ) -> io::Result<(PackageReport, InstallerPlan)> {
        let report = Self::package_project(project_path, destination, profile)?;
        let project_name = project_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("MiniForgeGame");
        let plan = Self::write_installer_plan(destination, project_name, platform, signing)?;
        Ok((report, plan))
    }

    pub fn write_installer_plan(
        package_dir: &Path,
        game_name: &str,
        platform: InstallerPlatform,
        signing: InstallerSigningConfig,
    ) -> io::Result<InstallerPlan> {
        let safe_name = AssetTools::safe_name(game_name, "MiniForgeGame");
        let installer_name = format!(
            "{safe_name}_{}.{}",
            platform.label(),
            platform.installer_extension()
        );
        let installer_path = package_dir.join(&installer_name);
        let signing_ready = signing.ready_for_platform(platform);
        let mut commands = installer_commands(package_dir, &installer_path, platform, &signing);
        let mut warnings = Vec::new();
        if !signing_ready {
            warnings.push(match platform {
                InstallerPlatform::Macos => {
                    "Firma macOS pendiente: configura identity Developer ID y team_id para notarizar."
                        .to_string()
                }
                InstallerPlatform::Windows => {
                    "Firma Windows pendiente: configura certificado Authenticode o identity de signtool."
                        .to_string()
                }
                InstallerPlatform::Linux => {
                    "Firma Linux pendiente: configura GPG/cosign identity para firmar AppImage."
                        .to_string()
                }
            });
            commands.push("# signing skipped until credentials are configured".to_string());
        }
        let artifacts = vec![
            installer_path.clone(),
            package_dir.join("installer_manifest.json"),
        ];
        let plan = InstallerPlan {
            platform,
            package_dir: package_dir.to_path_buf(),
            installer_name,
            installer_path,
            signing_ready,
            signing_identity: signing.identity.clone().or_else(|| {
                signing
                    .certificate_path
                    .as_ref()
                    .map(|path| path.display().to_string())
            }),
            commands,
            artifacts,
            warnings,
        };
        AssetTools::write_json(
            package_dir.join("installer_manifest.json"),
            &serde_json::to_value(&plan).map_err(io::Error::other)?,
        )?;
        Ok(plan)
    }
}

fn installer_commands(
    package_dir: &Path,
    installer_path: &Path,
    platform: InstallerPlatform,
    signing: &InstallerSigningConfig,
) -> Vec<String> {
    let package = package_dir.display();
    let output = installer_path.display();
    match platform {
        InstallerPlatform::Macos => {
            let identity = signing
                .identity
                .as_deref()
                .unwrap_or("Developer ID Application: <TEAM>");
            let mut commands = vec![
                format!(
                    "hdiutil create -volname MiniForgeGame -srcfolder \"{package}\" -ov -format UDZO \"{output}\""
                ),
                format!("codesign --force --timestamp --sign \"{identity}\" \"{output}\""),
            ];
            if signing.notarize {
                let team = signing.team_id.as_deref().unwrap_or("<TEAM_ID>");
                commands.push(format!(
                    "xcrun notarytool submit \"{output}\" --team-id \"{team}\" --wait"
                ));
                commands.push(format!("xcrun stapler staple \"{output}\""));
            }
            commands
        }
        InstallerPlatform::Windows => {
            let cert = signing
                .certificate_path
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "<certificate.pfx>".to_string());
            let timestamp = signing
                .timestamp_url
                .as_deref()
                .unwrap_or("http://timestamp.digicert.com");
            vec![
                format!("wix build installer.wxs -d SourceDir=\"{package}\" -o \"{output}\""),
                format!(
                    "signtool sign /fd SHA256 /tr \"{timestamp}\" /td SHA256 /f \"{cert}\" \"{output}\""
                ),
            ]
        }
        InstallerPlatform::Linux => {
            let identity = signing.identity.as_deref().unwrap_or("<gpg-key-id>");
            vec![
                format!("appimagetool \"{package}\" \"{output}\""),
                format!("gpg --batch --yes --local-user \"{identity}\" --detach-sign \"{output}\""),
            ]
        }
    }
}

fn copy_export_tree(from: &Path, to: &Path) -> io::Result<()> {
    if !from.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Carpeta de export no existe",
        ));
    }
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();
        if should_skip_packaging_copy(&path) {
            continue;
        }
        let target = to.join(&name);
        if path.is_dir() {
            fs::create_dir_all(&target)?;
            copy_export_tree(&path, &target)?;
        } else {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(path, target)?;
        }
    }
    Ok(())
}

fn should_skip_packaging_copy(path: &Path) -> bool {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    matches!(
        name,
        "_pkg_work" | "exports" | "target" | "build" | "builds"
    )
}

fn runtime_binary_name() -> &'static str {
    if cfg!(windows) {
        "miniforge_runtime.exe"
    } else {
        "miniforge_runtime"
    }
}

fn find_runtime_binary() -> Option<PathBuf> {
    runtime_binary_candidates()
        .into_iter()
        .find(|candidate| candidate.is_file())
}

fn wgpu_preview_binary_name() -> &'static str {
    if cfg!(windows) {
        "miniforge_wgpu_preview.exe"
    } else {
        "miniforge_wgpu_preview"
    }
}

fn find_wgpu_preview_binary() -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(path) = env::var("MINIFORGE_WGPU_PREVIEW") {
        candidates.push(PathBuf::from(path));
    }
    if let Ok(current) = env::current_exe() {
        candidates.push(current.with_file_name(wgpu_preview_binary_name()));
        if let Some(debug_or_release) = current.parent().and_then(Path::parent) {
            candidates.push(debug_or_release.join(wgpu_preview_binary_name()));
        }
    }
    if let Ok(cwd) = env::current_dir() {
        candidates.push(
            cwd.join("target")
                .join("debug")
                .join(wgpu_preview_binary_name()),
        );
        candidates.push(
            cwd.join("target")
                .join("release")
                .join(wgpu_preview_binary_name()),
        );
    }
    candidates.into_iter().find(|candidate| candidate.is_file())
}

fn runtime_binary_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(path) = env::var("MINIFORGE_RUNTIME") {
        candidates.push(PathBuf::from(path));
    }
    if let Ok(current) = env::current_exe() {
        candidates.push(current.with_file_name(runtime_binary_name()));
        if let Some(debug_or_release) = current.parent().and_then(Path::parent) {
            candidates.push(debug_or_release.join(runtime_binary_name()));
        }
    }
    if let Ok(cwd) = env::current_dir() {
        candidates.push(cwd.join("target").join("debug").join(runtime_binary_name()));
        candidates.push(
            cwd.join("target")
                .join("release")
                .join(runtime_binary_name()),
        );
    }
    candidates.sort();
    candidates.dedup();
    candidates
}

fn write_launcher_scripts(destination: &Path, runtime_binary: &Path) -> io::Result<Vec<PathBuf>> {
    let binary_name = runtime_binary
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(runtime_binary_name());
    let mut scripts = Vec::new();

    let sh_path = destination.join("run_game.sh");
    fs::write(
        &sh_path,
        format!(
            "#!/bin/sh\nDIR=\"$(cd \"$(dirname \"$0\")\" && pwd)\"\n\"$DIR/{binary_name}\" --build \"$DIR\"\n"
        ),
    )?;
    #[cfg(unix)]
    {
        let mut permissions = fs::metadata(&sh_path)?.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&sh_path, permissions)?;
    }
    scripts.push(sh_path);

    let command_path = destination.join("run_game.command");
    fs::write(
        &command_path,
        format!(
            "#!/bin/sh\nDIR=\"$(cd \"$(dirname \"$0\")\" && pwd)\"\n\"$DIR/{binary_name}\" --build \"$DIR\"\n"
        ),
    )?;
    #[cfg(unix)]
    {
        let mut permissions = fs::metadata(&command_path)?.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&command_path, permissions)?;
    }
    scripts.push(command_path);

    let bat_path = destination.join("run_game.bat");
    fs::write(
        &bat_path,
        format!("@echo off\r\n\"%~dp0{binary_name}\" --build \"%~dp0\"\r\n"),
    )?;
    scripts.push(bat_path);

    Ok(scripts)
}

fn write_standalone_manifest(
    destination: &Path,
    runtime_binary: Option<&PathBuf>,
    launcher_scripts: &[PathBuf],
    warnings: &[String],
) -> io::Result<()> {
    let runtime_name = runtime_binary
        .and_then(|path| path.file_name())
        .and_then(|name| name.to_str())
        .map(ToString::to_string);
    let launchers = launcher_scripts
        .iter()
        .filter_map(|path| path.file_name().and_then(|name| name.to_str()))
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    crate::engine::asset_tools::AssetTools::write_json(
        destination.join("standalone_manifest.json"),
        &serde_json::json!({
            "engine_version": crate::engine::version::ENGINE_VERSION,
            "engine_stream_version": crate::engine::version::ENGINE_STREAM_VERSION,
            "update_093": crate::engine::update_093::Engine093UpgradePlan::current().to_value(),
            "standalone": runtime_name.is_some(),
            "runtime_binary": runtime_name,
            "launchers": launchers,
            "build_root": ".",
            "warnings": warnings,
        }),
    )
}
