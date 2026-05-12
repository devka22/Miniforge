pub const ENGINE_VERSION: &str = "0.7.0";
pub const ENGINE_CODENAME: &str = "Production Editor Update";

pub fn version_label() -> String {
    format!("MiniForge {ENGINE_VERSION} - {ENGINE_CODENAME}")
}
