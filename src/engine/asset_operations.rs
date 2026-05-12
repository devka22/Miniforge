use std::io;
use std::path::{Path, PathBuf};

use crate::engine::asset_tools::AssetTools;

#[derive(Debug, Clone, Default)]
pub struct AssetOperations;

impl AssetOperations {
    pub fn duplicate(
        source: impl AsRef<Path>,
        target_folder: impl AsRef<Path>,
    ) -> io::Result<PathBuf> {
        AssetTools::safe_copy_to_folder(source, target_folder)
    }
}
