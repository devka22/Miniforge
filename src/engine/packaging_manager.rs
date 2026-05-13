//! Assembles a distributable folder from an exported runtime build (no nested self-copy).

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::engine::runtime_exporter::{ExportProfile, RuntimeExportReport, RuntimeExporter};

#[derive(Debug, Clone)]
pub struct PackageReport {
    pub destination: PathBuf,
    pub export: RuntimeExportReport,
    pub runtime_binary_copied: Option<PathBuf>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Default)]
pub struct PackagingManager;

impl PackagingManager {
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
        let runtime_binary_copied = if let Ok(exe) = std::env::var("MINIFORGE_RUNTIME") {
            let src = PathBuf::from(exe);
            if src.is_file() {
                let name = src
                    .file_name()
                    .map(|n| n.to_owned())
                    .unwrap_or_else(|| std::ffi::OsString::from("miniforge_runtime"));
                let dst = destination.join(name);
                fs::copy(&src, &dst)?;
                Some(dst)
            } else {
                warnings.push(format!(
                    "MINIFORGE_RUNTIME no encontrado: {}",
                    src.display()
                ));
                None
            }
        } else {
            warnings.push(
                "MINIFORGE_RUNTIME no definido: exporta el binario runtime manualmente o define la variable de entorno.".into(),
            );
            None
        };
        let _ = fs::remove_dir_all(&work_root);
        Ok(PackageReport {
            destination: destination.to_path_buf(),
            export,
            runtime_binary_copied,
            warnings,
        })
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
