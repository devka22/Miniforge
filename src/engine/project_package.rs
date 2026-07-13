use std::fs;
use std::io::{self, Read, Seek};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use walkdir::WalkDir;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::engine::asset_tools::AssetTools;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectPackageReport {
    pub archive_path: PathBuf,
    pub project_path: PathBuf,
    pub files: usize,
    pub bytes: u64,
}

#[derive(Debug, Clone, Default)]
pub struct ProjectPackageManager;

impl ProjectPackageManager {
    pub fn export_project(
        project_path: impl AsRef<Path>,
        output_path: impl AsRef<Path>,
    ) -> io::Result<ProjectPackageReport> {
        let project_path = project_path.as_ref();
        AssetTools::ensure_project_folders(project_path)?;
        let output_path = output_path.as_ref();
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let file = fs::File::create(output_path)?;
        let mut writer = ZipWriter::new(file);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        let mut report = ProjectPackageReport {
            archive_path: output_path.to_path_buf(),
            project_path: project_path.to_path_buf(),
            files: 0,
            bytes: 0,
        };

        for entry in WalkDir::new(project_path)
            .into_iter()
            .filter_map(Result::ok)
        {
            let path = entry.path();
            if path.is_dir() || should_skip_project_package_path(project_path, path) {
                continue;
            }
            let relative = path.strip_prefix(project_path).unwrap_or(path);
            let relative = relative.to_string_lossy().replace('\\', "/");
            writer.start_file(relative, options)?;
            let mut source = fs::File::open(path)?;
            let copied = io::copy(&mut source, &mut writer)?;
            report.files += 1;
            report.bytes += copied;
        }

        writer.finish()?;
        Ok(report)
    }

    pub fn import_project(
        archive_path: impl AsRef<Path>,
        destination_root: impl AsRef<Path>,
    ) -> io::Result<ProjectPackageReport> {
        let archive_path = archive_path.as_ref();
        let destination_root = destination_root.as_ref();
        fs::create_dir_all(destination_root)?;
        let project_name = archive_path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("ImportedProject");
        let project_path = AssetTools::unique_path(
            destination_root,
            &AssetTools::safe_name(project_name, "ImportedProject"),
        );
        fs::create_dir_all(&project_path)?;

        let file = fs::File::open(archive_path)?;
        let mut archive = ZipArchive::new(file)?;
        let mut report = ProjectPackageReport {
            archive_path: archive_path.to_path_buf(),
            project_path: project_path.clone(),
            files: 0,
            bytes: 0,
        };
        extract_archive(&mut archive, &project_path, &mut report)?;
        AssetTools::ensure_project_folders(&project_path)?;
        Ok(report)
    }
}

fn extract_archive<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    project_path: &Path,
    report: &mut ProjectPackageReport,
) -> io::Result<()> {
    for index in 0..archive.len() {
        let mut file = archive.by_index(index)?;
        let Some(enclosed) = file.enclosed_name() else {
            continue;
        };
        let outpath = project_path.join(enclosed);
        if file.is_dir() {
            fs::create_dir_all(&outpath)?;
            continue;
        }
        if let Some(parent) = outpath.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut output = fs::File::create(&outpath)?;
        let copied = io::copy(&mut file, &mut output)?;
        report.files += 1;
        report.bytes += copied;
    }
    Ok(())
}

fn should_skip_project_package_path(project_path: &Path, path: &Path) -> bool {
    let relative = path.strip_prefix(project_path).unwrap_or(path);
    if relative.starts_with(Path::new("saves").join("autosave")) {
        return true;
    }
    let filename = relative
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    if filename == ".DS_Store"
        || filename.ends_with(".bak")
        || filename.contains(".bak.")
        || filename.contains(".tmp.")
    {
        return true;
    }
    relative.components().any(|component| {
        let part = component.as_os_str().to_string_lossy();
        matches!(
            part.as_ref(),
            "target" | "build" | "builds" | "exports" | "packages" | "logs" | ".git" | ".miniforge"
        )
    })
}
