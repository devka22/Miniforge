use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Logger {
    pub path: PathBuf,
}

impl Logger {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn log(&self, message: &str) -> io::Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        writeln!(file, "{message}")
    }
}
