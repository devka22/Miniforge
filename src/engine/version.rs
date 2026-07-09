pub const ENGINE_VERSION: &str = "0.9.3.4";
pub const ENGINE_CODENAME: &str = "2D Workflow Foundations";
pub const ENGINE_STREAM_VERSION: &str = "0.9.3.4";
/// Compatibility alias for tools that learned about 0.9.3.4 while it was a
/// development target.
pub const DEVELOPMENT_VERSION: &str = ENGINE_VERSION;
pub const DEVELOPMENT_CODENAME: &str = ENGINE_CODENAME;

pub fn version_label() -> String {
    format!("MiniForge {ENGINE_VERSION} - {ENGINE_CODENAME}")
}

pub fn development_version_label() -> String {
    format!("MiniForge {DEVELOPMENT_VERSION} - {DEVELOPMENT_CODENAME} (released)")
}
