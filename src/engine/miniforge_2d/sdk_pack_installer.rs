use std::fs::{self, File, OpenOptions};
use std::io::{BufReader, Read, Write};
use std::path::Path;
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use zip::ZipArchive;

use super::sdk_packs::{InstalledSdkPack, SdkPackManifest};

const COPY_BUFFER_BYTES: usize = 1024 * 1024;
const DEFAULT_MAX_FILES: usize = 200_000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SdkPackReleaseArtifact {
    pub pack_id: String,
    pub version: String,
    pub platform: String,
    pub archive_url: String,
    pub archive_bytes: u64,
    pub archive_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SdkPackInstallReceipt {
    pub pack: InstalledSdkPack,
    pub platform: String,
    pub archive_sha256: String,
    pub installed_path: String,
    pub file_count: usize,
    pub extracted_bytes: u64,
    pub replaced_existing_version: bool,
}

#[derive(Debug, Clone)]
pub struct SdkPackArchiveInstaller {
    pub max_files: usize,
}

impl Default for SdkPackArchiveInstaller {
    fn default() -> Self {
        Self {
            max_files: DEFAULT_MAX_FILES,
        }
    }
}

impl SdkPackArchiveInstaller {
    pub fn install(
        &self,
        manifest: &SdkPackManifest,
        artifact: &SdkPackReleaseArtifact,
        archive_path: impl AsRef<Path>,
        install_root: impl AsRef<Path>,
    ) -> Result<SdkPackInstallReceipt, String> {
        validate_safe_token("pack id", &manifest.id)?;
        validate_safe_token("pack version", &manifest.version)?;
        validate_artifact(manifest, artifact)?;
        if manifest.archive_format != "zip" {
            return Err(format!(
                "SDK pack {} uses unsupported archive format {}",
                manifest.id, manifest.archive_format
            ));
        }

        let archive_path = archive_path.as_ref();
        let archive_metadata = fs::metadata(archive_path)
            .map_err(|error| format!("Cannot inspect SDK pack archive: {error}"))?;
        if !archive_metadata.is_file() {
            return Err("SDK pack archive path is not a regular file".to_string());
        }
        if archive_metadata.len() != artifact.archive_bytes {
            return Err(format!(
                "SDK pack archive size mismatch: expected {}, received {} bytes",
                artifact.archive_bytes,
                archive_metadata.len()
            ));
        }

        let actual_sha256 = sha256_file(archive_path)?;
        if !actual_sha256.eq_ignore_ascii_case(&artifact.archive_sha256) {
            return Err(format!(
                "SDK pack SHA-256 mismatch: expected {}, received {actual_sha256}",
                artifact.archive_sha256
            ));
        }

        let install_root = install_root.as_ref();
        fs::create_dir_all(install_root)
            .map_err(|error| format!("Cannot create SDK pack install root: {error}"))?;
        let nonce = unique_nonce()?;
        let staging_parent = install_root.join(".staging");
        fs::create_dir_all(&staging_parent)
            .map_err(|error| format!("Cannot create SDK pack staging root: {error}"))?;
        let staging = staging_parent.join(format!(
            "{}-{}-{}-{nonce}",
            manifest.id,
            manifest.version,
            process::id()
        ));
        let destination = install_root
            .join("packs")
            .join(&manifest.id)
            .join(&manifest.version);
        let backup = staging_parent.join(format!(
            "{}-{}-previous-{nonce}",
            manifest.id, manifest.version
        ));

        if staging.exists() || backup.exists() {
            return Err("SDK pack staging collision; retry the installation".to_string());
        }
        fs::create_dir_all(&staging)
            .map_err(|error| format!("Cannot create SDK pack staging directory: {error}"))?;

        let (file_count, extracted_bytes) =
            match self.extract_verified_archive(manifest, archive_path, &staging) {
                Ok(result) => result,
                Err(error) => {
                    let _ = fs::remove_dir_all(&staging);
                    return Err(error);
                }
            };

        let replaced_existing_version = destination.exists();
        let receipt = SdkPackInstallReceipt {
            pack: InstalledSdkPack {
                id: manifest.id.clone(),
                version: manifest.version.clone(),
                installed_bytes: extracted_bytes,
                verified: true,
            },
            platform: artifact.platform.clone(),
            archive_sha256: actual_sha256,
            installed_path: destination.to_string_lossy().into_owned(),
            file_count,
            extracted_bytes,
            replaced_existing_version,
        };
        let marker = serde_json::to_vec_pretty(&receipt)
            .map_err(|error| format!("Cannot serialize SDK pack receipt: {error}"))?;
        write_new_file(&staging.join(".miniforge-pack.json"), &marker)?;

        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("Cannot create SDK pack destination: {error}"))?;
        }
        if replaced_existing_version {
            fs::rename(&destination, &backup).map_err(|error| {
                format!("Cannot stage existing SDK pack for replacement: {error}")
            })?;
        }
        if let Err(error) = fs::rename(&staging, &destination) {
            if replaced_existing_version {
                let _ = fs::rename(&backup, &destination);
            }
            let _ = fs::remove_dir_all(&staging);
            return Err(format!("Cannot publish SDK pack atomically: {error}"));
        }
        if replaced_existing_version {
            fs::remove_dir_all(&backup).map_err(|error| {
                format!("SDK pack installed but old backup cleanup failed: {error}")
            })?;
        }
        Ok(receipt)
    }

    fn extract_verified_archive(
        &self,
        manifest: &SdkPackManifest,
        archive_path: &Path,
        staging: &Path,
    ) -> Result<(usize, u64), String> {
        let archive_file = File::open(archive_path)
            .map_err(|error| format!("Cannot open SDK pack archive: {error}"))?;
        let mut archive = ZipArchive::new(BufReader::new(archive_file))
            .map_err(|error| format!("Invalid SDK pack ZIP archive: {error}"))?;
        if archive.len() > self.max_files {
            return Err(format!(
                "SDK pack archive contains {} entries; limit is {}",
                archive.len(),
                self.max_files
            ));
        }
        let declared_bytes = archive
            .decompressed_size()
            .ok_or_else(|| "SDK pack expanded size overflowed".to_string())?;
        if declared_bytes > u128::from(manifest.installed_bytes) {
            return Err(format!(
                "SDK pack expands to {declared_bytes} bytes; manifest limit is {}",
                manifest.installed_bytes
            ));
        }

        let mut extracted_bytes = 0u64;
        let mut file_count = 0usize;
        let mut copy_buffer = vec![0u8; COPY_BUFFER_BYTES];
        for index in 0..archive.len() {
            let mut entry = archive
                .by_index(index)
                .map_err(|error| format!("Cannot read SDK pack ZIP entry {index}: {error}"))?;
            let relative_path = entry.enclosed_name().ok_or_else(|| {
                format!(
                    "SDK pack ZIP entry {} escapes the installation root",
                    entry.name()
                )
            })?;
            if relative_path.as_os_str().is_empty() {
                continue;
            }
            if entry.is_symlink() {
                return Err(format!(
                    "SDK pack ZIP entry {} is a symbolic link",
                    entry.name()
                ));
            }
            let output_path = staging.join(&relative_path);
            if entry.is_dir() {
                fs::create_dir_all(&output_path)
                    .map_err(|error| format!("Cannot create SDK pack directory: {error}"))?;
                continue;
            }
            if !entry.is_file() {
                return Err(format!(
                    "SDK pack ZIP entry {} is not a regular file",
                    entry.name()
                ));
            }
            let next_total = extracted_bytes
                .checked_add(entry.size())
                .ok_or_else(|| "SDK pack expanded size overflowed".to_string())?;
            if next_total > manifest.installed_bytes {
                return Err(format!(
                    "SDK pack extracted bytes exceed manifest limit {}",
                    manifest.installed_bytes
                ));
            }
            if let Some(parent) = output_path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|error| format!("Cannot create SDK pack directory: {error}"))?;
            }
            let mut output = OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&output_path)
                .map_err(|error| {
                    format!(
                        "Cannot create SDK pack file {}: {error}",
                        relative_path.display()
                    )
                })?;
            let expected_size = entry.size();
            let unix_mode = entry.unix_mode();
            let entry_name = entry.name().to_string();
            let copied = copy_entry(&mut entry, &mut output, &mut copy_buffer)?;
            if copied != expected_size {
                return Err(format!(
                    "SDK pack ZIP entry {entry_name} size mismatch: expected {expected_size}, extracted {copied}"
                ));
            }
            apply_safe_permissions(&output_path, unix_mode)?;
            extracted_bytes = next_total;
            file_count += 1;
        }
        Ok((file_count, extracted_bytes))
    }
}

pub fn sha256_file(path: impl AsRef<Path>) -> Result<String, String> {
    let file = File::open(path.as_ref())
        .map_err(|error| format!("Cannot open file for SHA-256 verification: {error}"))?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; COPY_BUFFER_BYTES];
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|error| format!("Cannot read file for SHA-256 verification: {error}"))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    let digest = hasher.finalize();
    let mut encoded = String::with_capacity(64);
    for byte in digest {
        std::fmt::Write::write_fmt(&mut encoded, format_args!("{byte:02x}"))
            .expect("writing SHA-256 into a String cannot fail");
    }
    Ok(encoded)
}

fn validate_artifact(
    manifest: &SdkPackManifest,
    artifact: &SdkPackReleaseArtifact,
) -> Result<(), String> {
    if artifact.pack_id != manifest.id || artifact.version != manifest.version {
        return Err("SDK pack artifact identity does not match its catalog manifest".to_string());
    }
    if artifact.platform.trim().is_empty() || !manifest.platforms.contains(&artifact.platform) {
        return Err(format!(
            "SDK pack artifact platform {} is not supported",
            artifact.platform
        ));
    }
    if artifact.archive_bytes == 0 {
        return Err("SDK pack artifact has an invalid archive size".to_string());
    }
    let checksum = artifact.archive_sha256.trim();
    if checksum.len() != 64 || !checksum.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("SDK pack artifact SHA-256 must contain 64 hexadecimal characters".to_string());
    }
    if !artifact.archive_url.starts_with("https://") && !artifact.archive_url.starts_with("file://")
    {
        return Err(
            "SDK pack artifact URL must use HTTPS or an explicit local file URL".to_string(),
        );
    }
    Ok(())
}

fn validate_safe_token(label: &str, value: &str) -> Result<(), String> {
    if value.is_empty()
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
        || value == "."
        || value == ".."
    {
        return Err(format!("SDK {label} contains unsafe path characters"));
    }
    Ok(())
}

fn copy_entry(
    input: &mut impl Read,
    output: &mut impl Write,
    buffer: &mut [u8],
) -> Result<u64, String> {
    let mut copied = 0u64;
    loop {
        let read = input
            .read(buffer)
            .map_err(|error| format!("Cannot extract SDK pack file: {error}"))?;
        if read == 0 {
            break;
        }
        output
            .write_all(&buffer[..read])
            .map_err(|error| format!("Cannot write SDK pack file: {error}"))?;
        copied = copied
            .checked_add(read as u64)
            .ok_or_else(|| "SDK pack extracted size overflowed".to_string())?;
    }
    output
        .flush()
        .map_err(|error| format!("Cannot flush SDK pack file: {error}"))?;
    Ok(copied)
}

fn write_new_file(path: &Path, contents: &[u8]) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .map_err(|error| format!("Cannot create SDK pack receipt: {error}"))?;
    file.write_all(contents)
        .map_err(|error| format!("Cannot write SDK pack receipt: {error}"))?;
    file.sync_all()
        .map_err(|error| format!("Cannot sync SDK pack receipt: {error}"))
}

fn unique_nonce() -> Result<u128, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .map_err(|error| format!("System clock cannot create SDK pack staging nonce: {error}"))
}

#[cfg(unix)]
fn apply_safe_permissions(path: &Path, unix_mode: Option<u32>) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;

    let mode = unix_mode.unwrap_or(0o644);
    let safe_mode = if mode & 0o111 != 0 { 0o755 } else { 0o644 };
    fs::set_permissions(path, fs::Permissions::from_mode(safe_mode))
        .map_err(|error| format!("Cannot set SDK pack file permissions: {error}"))
}

#[cfg(not(unix))]
fn apply_safe_permissions(_path: &Path, _unix_mode: Option<u32>) -> Result<(), String> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::miniforge_2d::sdk_packs::SdkPackKind;
    use std::io::{Cursor, Write};
    use std::path::PathBuf;
    use zip::write::SimpleFileOptions;
    use zip::{CompressionMethod, ZipWriter};

    #[test]
    fn verified_zip_installs_atomically_and_can_replace_the_same_version() {
        let root = test_root("verified");
        let archive_path = root.join("content.zip");
        fs::create_dir_all(&root).unwrap();
        write_zip(
            &archive_path,
            &[("materials/ground.txt", b"detailed ground texture")],
        );
        let archive_bytes = fs::metadata(&archive_path).unwrap().len();
        let checksum = sha256_file(&archive_path).unwrap();
        let manifest = manifest(archive_bytes, 1_024);
        let artifact = artifact(&manifest, archive_bytes, &checksum);
        let installer = SdkPackArchiveInstaller::default();

        let first = installer
            .install(&manifest, &artifact, &archive_path, &root)
            .unwrap();
        assert!(first.pack.verified);
        assert!(!first.replaced_existing_version);
        assert_eq!(first.file_count, 1);
        assert_eq!(
            fs::read_to_string(root.join("packs/test-content/1.0.0/materials/ground.txt")).unwrap(),
            "detailed ground texture"
        );

        let second = installer
            .install(&manifest, &artifact, &archive_path, &root)
            .unwrap();
        assert!(second.replaced_existing_version);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn checksum_mismatch_never_publishes_a_pack() {
        let root = test_root("checksum");
        let archive_path = root.join("content.zip");
        fs::create_dir_all(&root).unwrap();
        write_zip(&archive_path, &[("safe.txt", b"safe")]);
        let archive_bytes = fs::metadata(&archive_path).unwrap().len();
        let manifest = manifest(archive_bytes, 1_024);
        let artifact = artifact(&manifest, archive_bytes, &"0".repeat(64));

        let error = SdkPackArchiveInstaller::default()
            .install(&manifest, &artifact, &archive_path, &root)
            .unwrap_err();
        assert!(error.contains("SHA-256 mismatch"));
        assert!(!root.join("packs/test-content/1.0.0").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn traversal_entry_is_rejected_and_cleaned_up() {
        let root = test_root("traversal");
        let archive_path = root.join("content.zip");
        fs::create_dir_all(&root).unwrap();
        write_zip(&archive_path, &[("../escape.txt", b"blocked")]);
        let archive_bytes = fs::metadata(&archive_path).unwrap().len();
        let checksum = sha256_file(&archive_path).unwrap();
        let manifest = manifest(archive_bytes, 1_024);
        let artifact = artifact(&manifest, archive_bytes, &checksum);

        let error = SdkPackArchiveInstaller::default()
            .install(&manifest, &artifact, &archive_path, &root)
            .unwrap_err();
        assert!(error.contains("escapes the installation root"));
        assert!(!root.join("escape.txt").exists());
        assert!(!root.join("packs/test-content/1.0.0").exists());
        fs::remove_dir_all(root).unwrap();
    }

    fn manifest(download_bytes: u64, installed_bytes: u64) -> SdkPackManifest {
        SdkPackManifest {
            id: "test-content".to_string(),
            label: "Test Content".to_string(),
            version: "1.0.0".to_string(),
            summary: "Installer test".to_string(),
            kind: SdkPackKind::Content,
            download_bytes,
            installed_bytes,
            dependencies: Vec::new(),
            platforms: vec!["test-platform".to_string()],
            capabilities: vec!["test".to_string()],
            archive_format: "zip".to_string(),
            checksum_policy: "sha256-release-manifest-required".to_string(),
            source_path: "packs/test-content.zip".to_string(),
        }
    }

    fn artifact(
        manifest: &SdkPackManifest,
        archive_bytes: u64,
        checksum: &str,
    ) -> SdkPackReleaseArtifact {
        SdkPackReleaseArtifact {
            pack_id: manifest.id.clone(),
            version: manifest.version.clone(),
            platform: "test-platform".to_string(),
            archive_url: "file:///tmp/content.zip".to_string(),
            archive_bytes,
            archive_sha256: checksum.to_string(),
        }
    }

    fn write_zip(path: &Path, entries: &[(&str, &[u8])]) {
        let cursor = Cursor::new(Vec::new());
        let mut writer = ZipWriter::new(cursor);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        for (name, contents) in entries {
            writer.start_file(*name, options).unwrap();
            writer.write_all(contents).unwrap();
        }
        let bytes = writer.finish().unwrap().into_inner();
        fs::write(path, bytes).unwrap();
    }

    fn test_root(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "miniforge-sdk-installer-{label}-{}-{}",
            process::id(),
            unique_nonce().unwrap()
        ))
    }
}
