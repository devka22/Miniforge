//! Durable project-file persistence shared by editor and runtime tooling.
//!
//! Writes are serialized per destination inside the process, use a unique
//! temporary file in the destination directory, flush file data before the
//! replacement and keep the previous document available until the temp file is
//! complete. Callers that need recovery can opt into rotating backups.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock, Weak};
use std::time::Duration;

use serde::Serialize;

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(1);
static WRITE_LOCKS: OnceLock<Mutex<BTreeMap<PathBuf, Weak<Mutex<()>>>>> = OnceLock::new();

pub const DEFAULT_BACKUP_GENERATIONS: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageOperation {
    Validate,
    CreateDirectory,
    CreateTemporary,
    ReadSource,
    WriteTemporary,
    PreservePermissions,
    SyncTemporary,
    RotateBackup,
    ReplaceDestination,
    RestoreDestination,
    SyncDirectory,
    SerializeJson,
    CleanupTemporary,
}

impl fmt::Display for StorageOperation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Validate => "validate storage request",
            Self::CreateDirectory => "create destination directory",
            Self::CreateTemporary => "create temporary file",
            Self::ReadSource => "read source file",
            Self::WriteTemporary => "write temporary file",
            Self::PreservePermissions => "preserve file permissions",
            Self::SyncTemporary => "sync temporary file",
            Self::RotateBackup => "rotate backup",
            Self::ReplaceDestination => "replace destination",
            Self::RestoreDestination => "restore destination",
            Self::SyncDirectory => "sync destination directory",
            Self::SerializeJson => "serialize JSON document",
            Self::CleanupTemporary => "clean stale temporary file",
        };
        formatter.write_str(name)
    }
}

#[derive(Debug)]
pub struct StorageError {
    pub operation: StorageOperation,
    pub path: PathBuf,
    message: String,
    io_kind: io::ErrorKind,
    source: Option<Box<dyn Error + Send + Sync>>,
}

impl StorageError {
    fn io(operation: StorageOperation, path: impl Into<PathBuf>, source: io::Error) -> Self {
        Self {
            operation,
            path: path.into(),
            message: source.to_string(),
            io_kind: source.kind(),
            source: Some(Box::new(source)),
        }
    }

    fn json(path: impl Into<PathBuf>, source: serde_json::Error) -> Self {
        Self {
            operation: StorageOperation::SerializeJson,
            path: path.into(),
            message: source.to_string(),
            io_kind: io::ErrorKind::InvalidData,
            source: Some(Box::new(source)),
        }
    }

    fn invalid(path: impl Into<PathBuf>, message: impl Into<String>) -> Self {
        Self {
            operation: StorageOperation::Validate,
            path: path.into(),
            message: message.into(),
            io_kind: io::ErrorKind::InvalidInput,
            source: None,
        }
    }

    pub fn io_kind(&self) -> io::ErrorKind {
        self.io_kind
    }
}

impl fmt::Display for StorageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} failed for {}: {}",
            self.operation,
            self.path.display(),
            self.message
        )
    }
}

impl Error for StorageError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.source
            .as_deref()
            .map(|source| source as &(dyn Error + 'static))
    }
}

impl From<StorageError> for io::Error {
    fn from(error: StorageError) -> Self {
        io::Error::new(error.io_kind(), error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupPolicy {
    pub path: PathBuf,
    /// Number of recoverable versions, including `path` itself.
    pub generations: usize,
}

impl BackupPolicy {
    pub fn new(path: impl Into<PathBuf>, generations: usize) -> Self {
        Self {
            path: path.into(),
            generations,
        }
    }

    pub fn single(path: impl Into<PathBuf>) -> Self {
        Self::new(path, 1)
    }

    pub fn generation_path(&self, generation: usize) -> PathBuf {
        if generation == 0 {
            return self.path.clone();
        }
        let filename = self
            .path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("backup");
        self.path.with_file_name(format!("{filename}.{generation}"))
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AtomicWriteOptions {
    pub backup: Option<BackupPolicy>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AtomicWriteReport {
    pub bytes_written: u64,
    pub replaced_existing: bool,
    pub backup_paths: Vec<PathBuf>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ProjectStorage;

impl ProjectStorage {
    pub fn write_atomic(
        path: impl AsRef<Path>,
        bytes: &[u8],
    ) -> Result<AtomicWriteReport, StorageError> {
        Self::write_atomic_with_options(path, bytes, &AtomicWriteOptions::default())
    }

    pub fn write_atomic_with_backup(
        path: impl AsRef<Path>,
        bytes: &[u8],
        backup: BackupPolicy,
    ) -> Result<AtomicWriteReport, StorageError> {
        Self::write_atomic_with_options(
            path,
            bytes,
            &AtomicWriteOptions {
                backup: Some(backup),
            },
        )
    }

    pub fn write_atomic_with_options(
        path: impl AsRef<Path>,
        bytes: &[u8],
        options: &AtomicWriteOptions,
    ) -> Result<AtomicWriteReport, StorageError> {
        let path = path.as_ref();
        validate_destination(path, options)?;
        create_parent(path)?;

        let path_lock = lock_for(path);
        let _write_guard = path_lock
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        let replaced_existing = path.is_file();
        let backup_paths = if replaced_existing {
            options
                .backup
                .as_ref()
                .map(|policy| rotate_backups(path, policy))
                .transpose()?
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        let mut temporary = TemporaryFile::create_for(path)?;
        temporary.write_all(bytes)?;
        if replaced_existing {
            temporary.copy_permissions_from(path)?;
        }
        temporary.sync_all()?;
        temporary.persist(path)?;
        sync_parent(path)?;

        Ok(AtomicWriteReport {
            bytes_written: bytes.len() as u64,
            replaced_existing,
            backup_paths,
        })
    }

    pub fn write_json_atomic<T: Serialize + ?Sized>(
        path: impl AsRef<Path>,
        value: &T,
    ) -> Result<AtomicWriteReport, StorageError> {
        let path = path.as_ref();
        let bytes =
            serde_json::to_vec_pretty(value).map_err(|error| StorageError::json(path, error))?;
        Self::write_atomic(path, &bytes)
    }

    pub fn write_json_atomic_with_backup<T: Serialize + ?Sized>(
        path: impl AsRef<Path>,
        value: &T,
        backup: BackupPolicy,
    ) -> Result<AtomicWriteReport, StorageError> {
        let path = path.as_ref();
        let bytes =
            serde_json::to_vec_pretty(value).map_err(|error| StorageError::json(path, error))?;
        Self::write_atomic_with_backup(path, &bytes, backup)
    }

    /// Removes temp/rollback artifacts created by MiniForge that are older
    /// than `minimum_age`. This is intended for project-open recovery, not for
    /// use while another MiniForge process is actively writing the directory.
    pub fn cleanup_stale_temporary_files(
        directory: impl AsRef<Path>,
        minimum_age: Duration,
    ) -> Result<usize, StorageError> {
        let directory = directory.as_ref();
        let entries = fs::read_dir(directory).map_err(|error| {
            StorageError::io(StorageOperation::CleanupTemporary, directory, error)
        })?;
        let mut removed = 0;
        for entry in entries {
            let entry = entry.map_err(|error| {
                StorageError::io(StorageOperation::CleanupTemporary, directory, error)
            })?;
            let path = entry.path();
            if !is_miniforge_temporary(&path) || !is_older_than(&path, minimum_age) {
                continue;
            }
            fs::remove_file(&path).map_err(|error| {
                StorageError::io(StorageOperation::CleanupTemporary, &path, error)
            })?;
            removed += 1;
        }
        Ok(removed)
    }
}

fn is_miniforge_temporary(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    name.starts_with(".miniforge-") && (name.ends_with(".tmp") || name.ends_with(".rollback"))
}

fn is_older_than(path: &Path, minimum_age: Duration) -> bool {
    fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(|modified| modified.elapsed().ok())
        .is_some_and(|age| age >= minimum_age)
}

fn validate_destination(path: &Path, options: &AtomicWriteOptions) -> Result<(), StorageError> {
    if path.as_os_str().is_empty() {
        return Err(StorageError::invalid(path, "destination path is empty"));
    }
    if let Some(policy) = &options.backup {
        if policy.generations == 0 {
            return Err(StorageError::invalid(
                &policy.path,
                "backup generations must be greater than zero",
            ));
        }
        if policy.path == path {
            return Err(StorageError::invalid(
                &policy.path,
                "backup path must differ from destination",
            ));
        }
    }
    Ok(())
}

fn create_parent(path: &Path) -> Result<(), StorageError> {
    let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    else {
        return Ok(());
    };
    fs::create_dir_all(parent)
        .map_err(|error| StorageError::io(StorageOperation::CreateDirectory, parent, error))
}

fn rotate_backups(source: &Path, policy: &BackupPolicy) -> Result<Vec<PathBuf>, StorageError> {
    create_parent(&policy.path)?;
    let mut written = Vec::new();
    for generation in (1..policy.generations).rev() {
        let previous = policy.generation_path(generation - 1);
        if !previous.is_file() {
            continue;
        }
        let next = policy.generation_path(generation);
        atomic_copy(&previous, &next, StorageOperation::RotateBackup)?;
        written.push(next);
    }
    atomic_copy(source, &policy.path, StorageOperation::RotateBackup)?;
    written.push(policy.path.clone());
    written.sort();
    Ok(written)
}

fn atomic_copy(
    source: &Path,
    destination: &Path,
    operation: StorageOperation,
) -> Result<u64, StorageError> {
    create_parent(destination)?;
    let mut input = File::open(source)
        .map_err(|error| StorageError::io(StorageOperation::ReadSource, source, error))?;
    let mut temporary = TemporaryFile::create_for(destination)?;
    let copied = io::copy(&mut input, temporary.file_mut()?)
        .map_err(|error| StorageError::io(operation, destination, error))?;
    temporary.copy_permissions_from(source)?;
    temporary.sync_all()?;
    temporary.persist(destination)?;
    sync_parent(destination)?;
    Ok(copied)
}

fn lock_for(path: &Path) -> Arc<Mutex<()>> {
    let key = absolute_path(path);
    let registry = WRITE_LOCKS.get_or_init(|| Mutex::new(BTreeMap::new()));
    let mut registry = registry
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    registry.retain(|_, lock| lock.strong_count() > 0);
    if let Some(lock) = registry.get(&key).and_then(Weak::upgrade) {
        return lock;
    }
    let lock = Arc::new(Mutex::new(()));
    registry.insert(key, Arc::downgrade(&lock));
    lock
}

fn absolute_path(path: &Path) -> PathBuf {
    if path.is_absolute() {
        return path.to_path_buf();
    }
    std::env::current_dir()
        .map(|current| current.join(path))
        .unwrap_or_else(|_| path.to_path_buf())
}

fn sync_parent(path: &Path) -> Result<(), StorageError> {
    let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    else {
        return Ok(());
    };
    sync_directory(parent)
        .map_err(|error| StorageError::io(StorageOperation::SyncDirectory, parent, error))
}

#[cfg(unix)]
fn sync_directory(path: &Path) -> io::Result<()> {
    File::open(path)?.sync_all()
}

#[cfg(not(unix))]
fn sync_directory(_path: &Path) -> io::Result<()> {
    // Opening directories for sync is not portable on Windows. The temp file
    // itself is still synced before the replace.
    Ok(())
}

struct TemporaryFile {
    path: PathBuf,
    file: Option<File>,
}

impl TemporaryFile {
    fn create_for(destination: &Path) -> Result<Self, StorageError> {
        let parent = destination
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        let filename = destination
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("document");

        for _ in 0..128 {
            let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let path = parent.join(format!(
                ".miniforge-{filename}.{}.{}.tmp",
                std::process::id(),
                sequence
            ));
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(file) => {
                    return Ok(Self {
                        path,
                        file: Some(file),
                    });
                }
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => {
                    return Err(StorageError::io(
                        StorageOperation::CreateTemporary,
                        path,
                        error,
                    ));
                }
            }
        }

        Err(StorageError::invalid(
            destination,
            "could not allocate a unique temporary file after 128 attempts",
        ))
    }

    fn file_mut(&mut self) -> Result<&mut File, StorageError> {
        self.file.as_mut().ok_or_else(|| {
            StorageError::invalid(
                &self.path,
                "temporary file is no longer available for writing",
            )
        })
    }

    fn write_all(&mut self, bytes: &[u8]) -> Result<(), StorageError> {
        let path = self.path.clone();
        self.file_mut()?
            .write_all(bytes)
            .map_err(|error| StorageError::io(StorageOperation::WriteTemporary, path, error))
    }

    fn copy_permissions_from(&self, source: &Path) -> Result<(), StorageError> {
        let permissions = fs::metadata(source)
            .map_err(|error| {
                StorageError::io(StorageOperation::PreservePermissions, source, error)
            })?
            .permissions();
        fs::set_permissions(&self.path, permissions).map_err(|error| {
            StorageError::io(StorageOperation::PreservePermissions, &self.path, error)
        })
    }

    fn sync_all(&mut self) -> Result<(), StorageError> {
        let path = self.path.clone();
        let file = self.file_mut()?;
        file.flush()
            .and_then(|_| file.sync_all())
            .map_err(|error| StorageError::io(StorageOperation::SyncTemporary, path, error))
    }

    fn persist(mut self, destination: &Path) -> Result<(), StorageError> {
        self.file.take();
        match fs::rename(&self.path, destination) {
            Ok(()) => {
                self.path.clear();
                Ok(())
            }
            Err(first_error) if destination.exists() => {
                self.replace_existing(destination, first_error)
            }
            Err(error) => Err(StorageError::io(
                StorageOperation::ReplaceDestination,
                destination,
                error,
            )),
        }
    }

    fn replace_existing(
        &mut self,
        destination: &Path,
        first_error: io::Error,
    ) -> Result<(), StorageError> {
        let rollback = unique_rollback_path(destination);
        fs::rename(destination, &rollback).map_err(|error| {
            StorageError::io(StorageOperation::ReplaceDestination, destination, error)
        })?;

        match fs::rename(&self.path, destination) {
            Ok(()) => {
                self.path.clear();
                fs::remove_file(&rollback).map_err(|error| {
                    StorageError::io(StorageOperation::ReplaceDestination, rollback, error)
                })?;
                Ok(())
            }
            Err(replace_error) => {
                let restore_result = fs::rename(&rollback, destination);
                if let Err(restore_error) = restore_result {
                    return Err(StorageError {
                        operation: StorageOperation::RestoreDestination,
                        path: destination.to_path_buf(),
                        message: format!(
                            "replace failed ({replace_error}); restore failed ({restore_error}); initial replace error: {first_error}"
                        ),
                        io_kind: restore_error.kind(),
                        source: Some(Box::new(restore_error)),
                    });
                }
                Err(StorageError::io(
                    StorageOperation::ReplaceDestination,
                    destination,
                    replace_error,
                ))
            }
        }
    }
}

impl Drop for TemporaryFile {
    fn drop(&mut self) {
        if !self.path.as_os_str().is_empty() {
            let _ = fs::remove_file(&self.path);
        }
    }
}

fn unique_rollback_path(destination: &Path) -> PathBuf {
    let parent = destination.parent().unwrap_or_else(|| Path::new("."));
    let filename = destination
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("document");
    loop {
        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let candidate = parent.join(format!(
            ".miniforge-{filename}.{}.{}.rollback",
            std::process::id(),
            sequence
        ));
        if !candidate.exists() {
            return candidate;
        }
    }
}

pub fn read_bytes(path: impl AsRef<Path>) -> Result<Vec<u8>, StorageError> {
    let path = path.as_ref();
    let mut file = File::open(path)
        .map_err(|error| StorageError::io(StorageOperation::ReadSource, path, error))?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .map_err(|error| StorageError::io(StorageOperation::ReadSource, path, error))?;
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use serde::Serialize;
    use serde::ser::Error as _;
    use std::sync::{Arc, Barrier};
    use std::thread;

    use super::{BackupPolicy, ProjectStorage, TEMP_SEQUENCE};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::Ordering;
    use std::time::Duration;

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new(name: &str) -> Self {
            let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "miniforge_storage_{name}_{}_{}",
                std::process::id(),
                sequence
            ));
            fs::create_dir_all(&path).expect("test directory");
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn temporary_artifacts(directory: &Path) -> Vec<PathBuf> {
        fs::read_dir(directory)
            .expect("read test directory")
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                let name = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or_default();
                name.ends_with(".tmp") || name.ends_with(".rollback")
            })
            .collect()
    }

    #[test]
    fn atomic_write_replaces_document_and_cleans_temporary_files() {
        let directory = TestDirectory::new("replace");
        let path = directory.path().join("scene.json");
        fs::write(&path, b"old").expect("seed document");

        let report = ProjectStorage::write_atomic(&path, b"new content").expect("atomic write");

        assert!(report.replaced_existing);
        assert_eq!(report.bytes_written, 11);
        assert_eq!(fs::read(&path).expect("read document"), b"new content");
        assert!(temporary_artifacts(directory.path()).is_empty());
    }

    #[test]
    fn backup_policy_rotates_recoverable_versions() {
        let directory = TestDirectory::new("backup");
        let path = directory.path().join("main.scene");
        let backup = directory.path().join("main.scene.bak");
        fs::write(&path, b"version one").expect("seed document");

        ProjectStorage::write_atomic_with_backup(
            &path,
            b"version two",
            BackupPolicy::new(&backup, 3),
        )
        .expect("first backed write");
        ProjectStorage::write_atomic_with_backup(
            &path,
            b"version three",
            BackupPolicy::new(&backup, 3),
        )
        .expect("second backed write");

        assert_eq!(fs::read(&path).expect("current"), b"version three");
        assert_eq!(fs::read(&backup).expect("latest backup"), b"version two");
        assert_eq!(
            fs::read(backup.with_file_name("main.scene.bak.1")).expect("older backup"),
            b"version one"
        );
        assert!(temporary_artifacts(directory.path()).is_empty());
    }

    #[test]
    fn failed_backup_does_not_modify_existing_document() {
        let directory = TestDirectory::new("failure");
        let path = directory.path().join("project.json");
        let blocker = directory.path().join("not_a_directory");
        fs::write(&path, b"known good").expect("seed document");
        fs::write(&blocker, b"block parent creation").expect("seed blocker");
        let invalid_backup = blocker.join("project.json.bak");

        let error = ProjectStorage::write_atomic_with_backup(
            &path,
            b"new value",
            BackupPolicy::single(invalid_backup),
        )
        .expect_err("backup must fail");

        assert_eq!(error.io_kind(), std::io::ErrorKind::AlreadyExists);
        assert_eq!(fs::read(&path).expect("original remains"), b"known good");
        assert!(temporary_artifacts(directory.path()).is_empty());
    }

    #[test]
    fn serialization_failure_preserves_existing_document() {
        struct InvalidDocument;

        impl Serialize for InvalidDocument {
            fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                Err(S::Error::custom("intentional serialization failure"))
            }
        }

        let directory = TestDirectory::new("serialization");
        let path = directory.path().join("settings.json");
        fs::write(&path, b"known good json").expect("seed document");

        let error = ProjectStorage::write_json_atomic(&path, &InvalidDocument)
            .expect_err("serialization must fail");

        assert_eq!(error.operation, super::StorageOperation::SerializeJson);
        assert_eq!(error.io_kind(), std::io::ErrorKind::InvalidData);
        assert_eq!(
            fs::read(&path).expect("original remains"),
            b"known good json"
        );
        assert!(temporary_artifacts(directory.path()).is_empty());
    }

    #[test]
    fn concurrent_writes_are_serialized_per_document() {
        let directory = TestDirectory::new("concurrent");
        let path = Arc::new(directory.path().join("asset_metadata.json"));
        let barrier = Arc::new(Barrier::new(8));
        let mut workers = Vec::new();

        for index in 0..8 {
            let path = Arc::clone(&path);
            let barrier = Arc::clone(&barrier);
            workers.push(thread::spawn(move || {
                let payload = format!("payload-{index}");
                barrier.wait();
                ProjectStorage::write_atomic(path.as_ref(), payload.as_bytes())
                    .expect("concurrent atomic write");
            }));
        }
        for worker in workers {
            worker.join().expect("worker completed");
        }

        let value = fs::read_to_string(path.as_ref()).expect("final document");
        assert!(value.starts_with("payload-"));
        assert!(temporary_artifacts(directory.path()).is_empty());
    }

    #[test]
    fn stale_cleanup_only_removes_miniforge_artifacts() {
        let directory = TestDirectory::new("cleanup");
        let orphan = directory.path().join(".miniforge-scene.json.999999.1.tmp");
        let unrelated = directory.path().join("external.tmp");
        fs::write(&orphan, b"orphan").expect("seed orphan");
        fs::write(&unrelated, b"keep").expect("seed unrelated temp");

        let removed =
            ProjectStorage::cleanup_stale_temporary_files(directory.path(), Duration::ZERO)
                .expect("cleanup succeeds");

        assert_eq!(removed, 1);
        assert!(!orphan.exists());
        assert!(unrelated.exists());
    }
}
