use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

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
}

impl Logger {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
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
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        let seconds = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or(0);
        writeln!(file, "[{seconds}][{}][{channel}] {message}", level.as_str())
    }
}
