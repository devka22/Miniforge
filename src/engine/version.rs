pub const ENGINE_VERSION: &str = "0.9.1.1";
pub const ENGINE_CODENAME: &str = "Interface Overhaul Patch";
pub const ENGINE_STREAM_VERSION: &str = "0.9.2";

pub fn version_label() -> String {
    format!("MiniForge {ENGINE_VERSION} - {ENGINE_CODENAME}")
}
