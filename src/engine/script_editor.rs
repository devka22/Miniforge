use std::fs;
use std::io;
use std::path::PathBuf;

#[derive(Debug, Clone, Default)]
pub struct ScriptDocument {
    pub path: Option<PathBuf>,
    pub syntax_error: Option<String>,
    pub dirty: bool,
}

#[derive(Debug, Clone, Default)]
pub struct ScriptEditor {
    pub document: ScriptDocument,
    pub lines: Vec<String>,
    pub tabs: Vec<PathBuf>,
}

impl ScriptEditor {
    pub fn open(&mut self, path: PathBuf) -> io::Result<()> {
        let text = fs::read_to_string(&path)?;
        self.lines = text.lines().map(ToString::to_string).collect();
        self.document.path = Some(path.clone());
        if !self.tabs.contains(&path) {
            self.tabs.push(path);
        }
        Ok(())
    }

    pub fn save(&mut self) -> io::Result<()> {
        if let Some(path) = &self.document.path {
            fs::write(path, self.lines.join("\n"))?;
            self.document.dirty = false;
        }
        Ok(())
    }

    pub fn validate(&mut self) -> bool {
        let source = self.lines.join("\n");
        let mut paren = 0i32;
        for (line_number, ch) in source.chars().enumerate() {
            match ch {
                '(' | '[' | '{' => paren += 1,
                ')' | ']' | '}' => paren -= 1,
                _ => {}
            }
            if paren < 0 {
                self.document.syntax_error =
                    Some(format!("Unbalanced closing delimiter near {line_number}"));
                return false;
            }
        }
        for (index, line) in self.lines.iter().enumerate() {
            let trimmed = line.trim_end();
            if trimmed.starts_with("def ") && !trimmed.ends_with(':') {
                self.document.syntax_error = Some(format!(
                    "Function declaration missing ':' at line {}",
                    index + 1
                ));
                return false;
            }
        }
        if paren != 0 {
            self.document.syntax_error = Some("Unbalanced delimiters".to_string());
            return false;
        }
        self.document.syntax_error = None;
        true
    }
}
