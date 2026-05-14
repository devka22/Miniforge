pub const ENGINE_VERSION: &str = "0.8.0";
pub const ENGINE_CODENAME: &str = "Developer Stability Update";

pub fn version_label() -> String {
    format!("MiniForge {ENGINE_VERSION} - {ENGINE_CODENAME}")
}
