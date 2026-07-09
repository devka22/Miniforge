//! Panic reporting for MiniForge entry points.

use std::backtrace::Backtrace;
use std::fs;
use std::io;
use std::panic::{self, PanicHookInfo};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{OnceLock, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::engine::project_storage::ProjectStorage;

static CRASH_SEQUENCE: AtomicU64 = AtomicU64::new(1);
static CRASH_CONFIG: OnceLock<RwLock<CrashReporterConfig>> = OnceLock::new();
static PANIC_HOOK_INSTALLED: OnceLock<()> = OnceLock::new();

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CrashReporterConfig {
    pub directory: PathBuf,
    pub application: String,
    pub engine_version: String,
    pub project_path: Option<PathBuf>,
    pub max_reports: usize,
    pub include_backtrace: bool,
}

impl CrashReporterConfig {
    pub fn for_project(project_path: impl AsRef<Path>, application: impl Into<String>) -> Self {
        let project_path = project_path.as_ref().to_path_buf();
        Self {
            directory: project_path.join("logs").join("crashes"),
            application: application.into(),
            engine_version: crate::engine::version::ENGINE_VERSION.to_string(),
            project_path: Some(project_path),
            max_reports: 10,
            include_backtrace: true,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct CrashContext {
    pub message: String,
    pub location: Option<String>,
    pub thread: Option<String>,
    pub backtrace: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CrashReport {
    pub schema_version: u32,
    pub timestamp_unix_ms: u128,
    pub process_id: u32,
    pub application: String,
    pub engine_version: String,
    pub project_name: Option<String>,
    pub operating_system: String,
    pub architecture: String,
    pub context: CrashContext,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrashWriteReport {
    pub path: PathBuf,
    pub retention_warning: Option<String>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct CrashReporter;

impl CrashReporter {
    /// Installs the process-wide hook once and updates its active project
    /// configuration on subsequent calls.
    pub fn install(config: CrashReporterConfig) {
        let config_lock = CRASH_CONFIG.get_or_init(|| RwLock::new(config.clone()));
        *config_lock
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = config;

        PANIC_HOOK_INSTALLED.get_or_init(|| {
            let previous_hook = panic::take_hook();
            panic::set_hook(Box::new(move |panic_info| {
                if let Some(config_lock) = CRASH_CONFIG.get() {
                    let config = config_lock
                        .read()
                        .unwrap_or_else(|poisoned| poisoned.into_inner())
                        .clone();
                    let context = context_from_panic(panic_info, config.include_backtrace);
                    if let Err(error) = Self::write_report(&config, context) {
                        eprintln!("MiniForge could not write crash report: {error}");
                    }
                }
                previous_hook(panic_info);
            }));
        });
    }

    pub fn write_report(
        config: &CrashReporterConfig,
        mut context: CrashContext,
    ) -> io::Result<CrashWriteReport> {
        sanitize_context(&mut context, config);
        let timestamp_unix_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_millis())
            .unwrap_or(0);
        let sequence = CRASH_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let report = CrashReport {
            schema_version: 1,
            timestamp_unix_ms,
            process_id: std::process::id(),
            application: truncate_text(&config.application, 128),
            engine_version: truncate_text(&config.engine_version, 64),
            project_name: config
                .project_path
                .as_deref()
                .and_then(Path::file_name)
                .and_then(|name| name.to_str())
                .map(|name| truncate_text(name, 128)),
            operating_system: std::env::consts::OS.to_string(),
            architecture: std::env::consts::ARCH.to_string(),
            context,
        };
        let path = config.directory.join(format!(
            "miniforge-crash-{timestamp_unix_ms}-{}-{sequence}.json",
            std::process::id()
        ));
        ProjectStorage::write_json_atomic(&path, &report).map_err(io::Error::from)?;
        let retention_warning = prune_reports(&config.directory, config.max_reports)
            .err()
            .map(|error| error.to_string());
        Ok(CrashWriteReport {
            path,
            retention_warning,
        })
    }
}

fn context_from_panic(info: &PanicHookInfo<'_>, include_backtrace: bool) -> CrashContext {
    let message = info
        .payload()
        .downcast_ref::<&str>()
        .map(|message| (*message).to_string())
        .or_else(|| info.payload().downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "panic payload is not a string".to_string());
    let location = info.location().map(|location| {
        format!(
            "{}:{}:{}",
            location.file(),
            location.line(),
            location.column()
        )
    });
    let thread = std::thread::current().name().map(ToString::to_string);
    let backtrace = include_backtrace.then(|| Backtrace::force_capture().to_string());
    CrashContext {
        message,
        location,
        thread,
        backtrace,
    }
}

fn sanitize_context(context: &mut CrashContext, config: &CrashReporterConfig) {
    context.message = sanitize_sensitive(&truncate_text(&context.message, 8_192), config);
    context.location = context
        .location
        .take()
        .map(|value| sanitize_sensitive(&truncate_text(&value, 1_024), config));
    context.thread = context
        .thread
        .take()
        .map(|value| truncate_text(&value, 256));
    context.backtrace = context
        .backtrace
        .take()
        .map(|value| sanitize_sensitive(&truncate_text(&value, 128 * 1024), config));
}

fn sanitize_sensitive(text: &str, config: &CrashReporterConfig) -> String {
    let mut replacements = Vec::<(String, &'static str)>::new();
    if let Some(project) = &config.project_path {
        replacements.push((project.to_string_lossy().to_string(), "<PROJECT>"));
    }
    for key in ["HOME", "USERPROFILE"] {
        if let Some(path) = std::env::var_os(key) {
            replacements.push((path.to_string_lossy().to_string(), "<HOME>"));
        }
    }
    if let Ok(current) = std::env::current_dir() {
        replacements.push((current.to_string_lossy().to_string(), "<WORKDIR>"));
    }
    replacements.sort_by_key(|entry| std::cmp::Reverse(entry.0.len()));
    replacements.dedup_by(|left, right| left.0 == right.0);

    let mut sanitized = text.to_string();
    for (sensitive, replacement) in replacements {
        if !sensitive.is_empty() {
            sanitized = sanitized.replace(&sensitive, replacement);
        }
    }
    sanitized
}

fn truncate_text(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let mut truncated = text.chars().take(max_chars).collect::<String>();
    truncated.push_str("…[truncated]");
    truncated
}

fn prune_reports(directory: &Path, max_reports: usize) -> io::Result<()> {
    if !directory.is_dir() {
        return Ok(());
    }
    let mut reports = fs::read_dir(directory)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("miniforge-crash-") && name.ends_with(".json"))
        })
        .collect::<Vec<_>>();
    reports.sort();
    let remove_count = reports.len().saturating_sub(max_reports.max(1));
    for path in reports.into_iter().take(remove_count) {
        fs::remove_file(path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::{CrashContext, CrashReporter, CrashReporterConfig};

    static TEST_SEQUENCE: AtomicU64 = AtomicU64::new(1);

    struct TestDirectory(std::path::PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let sequence = TEST_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "miniforge_crash_reporter_{}_{}",
                std::process::id(),
                sequence
            ));
            fs::create_dir_all(&path).expect("test directory");
            Self(path)
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn crash_report_hides_project_path_and_has_bounded_retention() {
        let directory = TestDirectory::new();
        let project = directory.0.join("SecretProject");
        let reports = directory.0.join("reports");
        fs::create_dir_all(&project).expect("project directory");
        let config = CrashReporterConfig {
            directory: reports.clone(),
            application: "MiniForge Test".to_string(),
            engine_version: "test".to_string(),
            project_path: Some(project.clone()),
            max_reports: 2,
            include_backtrace: false,
        };

        for index in 0..4 {
            CrashReporter::write_report(
                &config,
                CrashContext {
                    message: format!("failed inside {} at pass {index}", project.display()),
                    location: Some(format!("{}/source.rs:10", project.display())),
                    thread: Some("test-worker".to_string()),
                    backtrace: None,
                },
            )
            .expect("write crash report");
        }

        let report_paths = fs::read_dir(&reports)
            .expect("report directory")
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .collect::<Vec<_>>();
        assert_eq!(report_paths.len(), 2);
        for path in report_paths {
            let contents = fs::read_to_string(path).expect("read report");
            assert!(!contents.contains(&project.to_string_lossy().to_string()));
            assert!(contents.contains("<PROJECT>"));
            assert!(contents.contains("SecretProject"));
        }
    }
}
