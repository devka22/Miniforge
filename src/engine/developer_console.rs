use std::path::PathBuf;

use crate::engine::logger::{LogLevel, Logger};

#[derive(Debug, Clone, Default)]
pub struct DeveloperConsole {
    pub entries: Vec<(String, String)>,
    pub log_path: Option<PathBuf>,
}

impl DeveloperConsole {
    pub fn with_log_file(path: impl Into<PathBuf>) -> Self {
        Self {
            entries: Vec::new(),
            log_path: Some(path.into()),
        }
    }

    pub fn set_log_file(&mut self, path: impl Into<PathBuf>) {
        self.log_path = Some(path.into());
    }

    pub fn log(&mut self, message: impl Into<String>, channel: impl Into<String>) {
        let channel = channel.into();
        let message = message.into();
        if let Some(path) = &self.log_path {
            let _ = Logger::new(path.clone()).log_level(
                LogLevel::from_channel(&channel),
                &channel,
                &message,
            );
        }
        self.entries.push((channel, message));
        if self.entries.len() > 1000 {
            let excess = self.entries.len() - 1000;
            self.entries.drain(0..excess);
        }
    }

    pub fn debug(&mut self, message: impl Into<String>, channel: impl Into<String>) {
        let channel = channel.into();
        let message = message.into();
        if let Some(path) = &self.log_path {
            let _ = Logger::new(path.clone()).debug(&channel, &message);
        }
        self.entries.push((channel, message));
    }

    pub fn warning(&mut self, message: impl Into<String>, channel: impl Into<String>) {
        self.log(message, channel);
    }

    pub fn error(&mut self, message: impl Into<String>, channel: impl Into<String>) {
        let channel = channel.into();
        let message = message.into();
        if let Some(path) = &self.log_path {
            let _ = Logger::new(path.clone()).error(&channel, &message);
        }
        self.entries.push((channel, message));
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}
