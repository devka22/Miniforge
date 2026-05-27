use std::path::PathBuf;

use crate::engine::logger::{LogLevel, Logger};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsoleEntry {
    pub channel: String,
    pub message: String,
    pub severity: ConsoleSeverity,
    pub frame: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConsoleSeverity {
    Debug,
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Default)]
pub struct DeveloperConsole {
    pub entries: Vec<(String, String)>,
    pub structured_entries: Vec<ConsoleEntry>,
    pub log_path: Option<PathBuf>,
    pub frame: u64,
    pub error_count: usize,
    pub warning_count: usize,
}

impl DeveloperConsole {
    pub fn with_log_file(path: impl Into<PathBuf>) -> Self {
        Self {
            entries: Vec::new(),
            structured_entries: Vec::new(),
            log_path: Some(path.into()),
            frame: 0,
            error_count: 0,
            warning_count: 0,
        }
    }

    pub fn set_log_file(&mut self, path: impl Into<PathBuf>) {
        self.log_path = Some(path.into());
    }

    pub fn log(&mut self, message: impl Into<String>, channel: impl Into<String>) {
        let channel = channel.into();
        let message = message.into();
        let severity = ConsoleSeverity::from_channel(&channel);
        if let Some(path) = &self.log_path {
            let _ = Logger::new(path.clone()).log_level(
                LogLevel::from_channel(&channel),
                &channel,
                &message,
            );
        }
        self.push_entry(channel, message, severity);
    }

    pub fn debug(&mut self, message: impl Into<String>, channel: impl Into<String>) {
        let channel = channel.into();
        let message = message.into();
        if let Some(path) = &self.log_path {
            let _ = Logger::new(path.clone()).debug(&channel, &message);
        }
        self.push_entry(channel, message, ConsoleSeverity::Debug);
    }

    pub fn warning(&mut self, message: impl Into<String>, channel: impl Into<String>) {
        let channel = channel.into();
        let message = message.into();
        if let Some(path) = &self.log_path {
            let _ = Logger::new(path.clone()).log_level(LogLevel::Warning, &channel, &message);
        }
        self.push_entry(channel, message, ConsoleSeverity::Warning);
    }

    pub fn error(&mut self, message: impl Into<String>, channel: impl Into<String>) {
        let channel = channel.into();
        let message = message.into();
        if let Some(path) = &self.log_path {
            let _ = Logger::new(path.clone()).error(&channel, &message);
        }
        self.push_entry(channel, message, ConsoleSeverity::Error);
    }

    pub fn report_error(
        &mut self,
        channel: impl Into<String>,
        context: impl Into<String>,
        error: impl std::fmt::Display,
    ) {
        self.error(format!("{}: {}", context.into(), error), channel);
    }

    pub fn advance_frame(&mut self) {
        self.frame = self.frame.saturating_add(1);
    }

    pub fn clear_channel(&mut self, channel: &str) {
        self.entries
            .retain(|(entry_channel, _)| entry_channel != channel);
        self.structured_entries
            .retain(|entry| entry.channel != channel);
        self.recount();
    }

    pub fn search(&self, query: &str, min_severity: ConsoleSeverity) -> Vec<ConsoleEntry> {
        let query = query.trim().to_lowercase();
        self.structured_entries
            .iter()
            .filter(|entry| {
                entry.severity >= min_severity
                    && (query.is_empty()
                        || entry.channel.to_lowercase().contains(&query)
                        || entry.message.to_lowercase().contains(&query))
            })
            .cloned()
            .collect()
    }

    pub fn summary(&self) -> String {
        format!(
            "{} logs | {} warnings | {} errors",
            self.entries.len(),
            self.warning_count,
            self.error_count
        )
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.structured_entries.clear();
        self.error_count = 0;
        self.warning_count = 0;
    }

    fn push_entry(&mut self, channel: String, message: String, severity: ConsoleSeverity) {
        if severity == ConsoleSeverity::Error {
            self.error_count += 1;
        } else if severity == ConsoleSeverity::Warning {
            self.warning_count += 1;
        }
        self.entries.push((channel.clone(), message.clone()));
        self.structured_entries.push(ConsoleEntry {
            channel,
            message,
            severity,
            frame: self.frame,
        });
        self.truncate();
    }

    fn truncate(&mut self) {
        if self.entries.len() > 1000 {
            let excess = self.entries.len() - 1000;
            self.entries.drain(0..excess);
        }
        if self.structured_entries.len() > 1000 {
            let excess = self.structured_entries.len() - 1000;
            self.structured_entries.drain(0..excess);
            self.recount();
        }
    }

    fn recount(&mut self) {
        self.error_count = self
            .structured_entries
            .iter()
            .filter(|entry| entry.severity == ConsoleSeverity::Error)
            .count();
        self.warning_count = self
            .structured_entries
            .iter()
            .filter(|entry| entry.severity == ConsoleSeverity::Warning)
            .count();
    }
}

impl ConsoleSeverity {
    pub fn from_channel(channel: &str) -> Self {
        match channel {
            "ERROR" => Self::Error,
            "WARNING" | "VALIDATOR" => Self::Warning,
            "DEBUG" => Self::Debug,
            _ => Self::Info,
        }
    }
}
