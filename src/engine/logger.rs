use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

static LOG_WRITE_LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LogRotationPolicy {
    pub max_bytes: u64,
    /// Number of rotated files kept in addition to the active log.
    pub max_files: usize,
}

impl Default for LogRotationPolicy {
    fn default() -> Self {
        Self {
            max_bytes: 5 * 1024 * 1024,
            max_files: 5,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LogRotationReport {
    pub rotated: bool,
    pub retained_files: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
}

impl LogLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Debug => "debug",
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Error => "error",
        }
    }

    pub fn from_channel(channel: &str) -> Self {
        match channel.to_ascii_uppercase().as_str() {
            "ERROR" => Self::Error,
            "WARNING" | "WARN" => Self::Warning,
            "DEBUG" => Self::Debug,
            _ => Self::Info,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Logger {
    pub path: PathBuf,
    pub rotation: LogRotationPolicy,
}

impl Logger {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            rotation: LogRotationPolicy::default(),
        }
    }

    pub fn with_rotation(path: PathBuf, rotation: LogRotationPolicy) -> Self {
        Self { path, rotation }
    }

    pub fn log(&self, message: &str) -> io::Result<()> {
        self.info("ENGINE", message)
    }

    pub fn debug(&self, channel: &str, message: &str) -> io::Result<()> {
        self.log_level(LogLevel::Debug, channel, message)
    }

    pub fn info(&self, channel: &str, message: &str) -> io::Result<()> {
        self.log_level(LogLevel::Info, channel, message)
    }

    pub fn warning(&self, channel: &str, message: &str) -> io::Result<()> {
        self.log_level(LogLevel::Warning, channel, message)
    }

    pub fn error(&self, channel: &str, message: &str) -> io::Result<()> {
        self.log_level(LogLevel::Error, channel, message)
    }

    pub fn log_level(&self, level: LogLevel, channel: &str, message: &str) -> io::Result<()> {
        let _guard = LOG_WRITE_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let seconds = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or(0);
        let channel = sanitize_log_text(channel);
        let message = sanitize_log_text(message);
        let line = format!("[{seconds}][{}][{channel}] {message}\n", level.as_str());
        self.rotate_if_needed(line.len() as u64)?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        file.write_all(line.as_bytes())?;
        file.flush()
    }

    pub fn rotate_if_needed(&self, incoming_bytes: u64) -> io::Result<LogRotationReport> {
        if self.rotation.max_bytes == 0 || !self.path.is_file() {
            return Ok(LogRotationReport::default());
        }
        let current_bytes = fs::metadata(&self.path)?.len();
        if current_bytes.saturating_add(incoming_bytes) <= self.rotation.max_bytes {
            return Ok(LogRotationReport::default());
        }

        if self.rotation.max_files == 0 {
            OpenOptions::new()
                .write(true)
                .truncate(true)
                .open(&self.path)?;
            return Ok(LogRotationReport {
                rotated: true,
                retained_files: 0,
            });
        }

        for generation in (1..=self.rotation.max_files).rev() {
            let source = if generation == 1 {
                self.path.clone()
            } else {
                self.rotated_path(generation - 1)
            };
            if !source.is_file() {
                continue;
            }
            let destination = self.rotated_path(generation);
            if destination.exists() {
                fs::remove_file(&destination)?;
            }
            fs::rename(source, destination)?;
        }

        let retained_files = (1..=self.rotation.max_files)
            .filter(|generation| self.rotated_path(*generation).is_file())
            .count();
        Ok(LogRotationReport {
            rotated: true,
            retained_files,
        })
    }

    pub fn rotated_path(&self, generation: usize) -> PathBuf {
        let filename = self
            .path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("miniforge.log");
        self.path.with_file_name(format!("{filename}.{generation}"))
    }
}

fn sanitize_log_text(text: &str) -> String {
    text.replace('\r', "\\r").replace('\n', "\\n")
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::{LogRotationPolicy, Logger};

    static TEST_SEQUENCE: AtomicU64 = AtomicU64::new(1);

    fn test_log(name: &str) -> PathBufGuard {
        let sequence = TEST_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "miniforge_logger_{name}_{}_{}",
            std::process::id(),
            sequence
        ));
        fs::create_dir_all(&root).expect("test log directory");
        PathBufGuard(root)
    }

    struct PathBufGuard(std::path::PathBuf);

    impl Drop for PathBufGuard {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn logger_rotates_and_retains_bounded_history() {
        let root = test_log("rotation");
        let path = root.0.join("engine.log");
        let logger = Logger::with_rotation(
            path.clone(),
            LogRotationPolicy {
                max_bytes: 80,
                max_files: 2,
            },
        );

        for index in 0..12 {
            logger
                .info("ENGINE", &format!("message {index} with enough content"))
                .expect("log write");
        }

        assert!(path.is_file());
        assert!(logger.rotated_path(1).is_file());
        assert!(logger.rotated_path(2).is_file());
        assert!(!logger.rotated_path(3).exists());
    }

    #[test]
    fn logger_escapes_multiline_input() {
        let root = test_log("sanitize");
        let path = root.0.join("engine.log");
        Logger::new(path.clone())
            .warning("USER\nFORGED", "first line\r\nsecond line")
            .expect("log write");

        let log = fs::read_to_string(path).expect("read log");
        assert_eq!(log.lines().count(), 1);
        assert!(log.contains("USER\\nFORGED"));
        assert!(log.contains("first line\\r\\nsecond line"));
    }
}
