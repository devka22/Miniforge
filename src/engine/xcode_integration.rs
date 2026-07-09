use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AppleBuildTarget {
    MacOS,
    IOSSimulator,
    IOSDevice,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct XcodeBuildPlan {
    pub project_root: PathBuf,
    pub workspace: PathBuf,
    pub scheme: String,
    pub configuration: String,
    pub target: AppleBuildTarget,
    pub sdk: String,
    pub destination: String,
    pub derived_data_path: PathBuf,
    #[serde(default)]
    pub actions: Vec<String>,
}

impl XcodeBuildPlan {
    pub fn macos_debug(project_root: impl AsRef<Path>, scheme: impl Into<String>) -> Self {
        Self::new(
            project_root,
            scheme,
            "Debug",
            AppleBuildTarget::MacOS,
            "macosx",
            "platform=macOS",
        )
    }

    pub fn macos_release(project_root: impl AsRef<Path>, scheme: impl Into<String>) -> Self {
        Self::new(
            project_root,
            scheme,
            "Release",
            AppleBuildTarget::MacOS,
            "macosx",
            "platform=macOS",
        )
    }

    pub fn ios_simulator(project_root: impl AsRef<Path>, scheme: impl Into<String>) -> Self {
        Self::new(
            project_root,
            scheme,
            "Debug",
            AppleBuildTarget::IOSSimulator,
            "iphonesimulator",
            "platform=iOS Simulator,name=iPhone 16",
        )
    }

    pub fn command_args(&self) -> Vec<String> {
        let mut args = vec![
            "-workspace".to_string(),
            self.workspace.to_string_lossy().to_string(),
            "-scheme".to_string(),
            self.scheme.clone(),
            "-configuration".to_string(),
            self.configuration.clone(),
            "-sdk".to_string(),
            self.sdk.clone(),
            "-destination".to_string(),
            self.destination.clone(),
            "-derivedDataPath".to_string(),
            self.derived_data_path.to_string_lossy().to_string(),
        ];
        args.extend(self.actions.clone());
        args
    }

    pub fn open_command(&self) -> Vec<String> {
        vec![
            "open".to_string(),
            self.workspace.to_string_lossy().to_string(),
        ]
    }

    pub fn validate(&self) -> Vec<String> {
        let mut issues = Vec::new();
        if self.scheme.trim().is_empty() {
            issues.push("XcodeBuildPlan necesita scheme".to_string());
        }
        if self.configuration.trim().is_empty() {
            issues.push("XcodeBuildPlan necesita configuration".to_string());
        }
        if self.workspace.extension().and_then(|value| value.to_str()) != Some("xcworkspace") {
            issues.push("workspace debe terminar en .xcworkspace".to_string());
        }
        issues
    }

    fn new(
        project_root: impl AsRef<Path>,
        scheme: impl Into<String>,
        configuration: impl Into<String>,
        target: AppleBuildTarget,
        sdk: impl Into<String>,
        destination: impl Into<String>,
    ) -> Self {
        let project_root = project_root.as_ref().to_path_buf();
        let scheme = scheme.into();
        Self {
            workspace: project_root
                .join("apple")
                .join(format!("{scheme}.xcworkspace")),
            derived_data_path: project_root.join("build").join("xcode-derived-data"),
            project_root,
            scheme,
            configuration: configuration.into(),
            target,
            sdk: sdk.into(),
            destination: destination.into(),
            actions: vec!["build".to_string()],
        }
    }
}
