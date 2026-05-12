#[derive(Debug, Clone, Default)]
pub struct ErrorHandler {
    pub errors: Vec<String>,
    pub last_call_failed: bool,
}

impl ErrorHandler {
    pub fn safe_call<F>(&mut self, name: &str, mut callback: F)
    where
        F: FnMut() -> Result<(), String>,
    {
        match callback() {
            Ok(()) => self.last_call_failed = false,
            Err(error) => {
                self.last_call_failed = true;
                self.errors.push(format!("{name}: {error}"));
            }
        }
    }
}
