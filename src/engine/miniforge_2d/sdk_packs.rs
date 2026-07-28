use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

pub const SDK_PACK_CATALOG_SCHEMA_VERSION: u32 = 1;
const GIB: u64 = 1024 * 1024 * 1024;
const MIB: u64 = 1024 * 1024;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum SdkPackKind {
    Toolchain,
    Rendering,
    Content,
    AudioFonts,
    Templates,
    DebugSymbols,
    Examples,
    PlatformSdk,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SdkPackManifest {
    pub id: String,
    pub label: String,
    pub version: String,
    pub summary: String,
    pub kind: SdkPackKind,
    pub download_bytes: u64,
    pub installed_bytes: u64,
    pub dependencies: Vec<String>,
    pub platforms: Vec<String>,
    pub capabilities: Vec<String>,
    pub archive_format: String,
    pub checksum_policy: String,
    pub source_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SdkPackProfile {
    pub id: String,
    pub label: String,
    pub summary: String,
    pub pack_ids: Vec<String>,
    pub target_min_bytes: u64,
    pub target_max_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InstalledSdkPack {
    pub id: String,
    pub version: String,
    pub installed_bytes: u64,
    pub verified: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SdkPackRegistry {
    pub installed: Vec<InstalledSdkPack>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SdkPackInstallItem {
    pub id: String,
    pub label: String,
    pub version: String,
    pub reason: String,
    pub download_bytes: u64,
    pub installed_bytes: u64,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SdkPackInstallPlan {
    pub profile_id: String,
    pub profile_label: String,
    pub install: Vec<SdkPackInstallItem>,
    pub already_installed: Vec<String>,
    pub total_download_bytes: u64,
    pub total_installed_bytes: u64,
    pub projected_installed_bytes: u64,
    pub target_min_bytes: u64,
    pub target_max_bytes: u64,
    pub meets_profile_target: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SdkPackCatalogValidation {
    pub valid: bool,
    pub issues: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SdkPackCatalog {
    pub schema_version: u32,
    pub packs: Vec<SdkPackManifest>,
    pub profiles: Vec<SdkPackProfile>,
    pub total_available_installed_bytes: u64,
}

impl SdkPackCatalog {
    pub fn builtin() -> Self {
        let packs = vec![
            pack(
                "rust-build-toolchain",
                "Rust Build Toolchain",
                SdkPackKind::Toolchain,
                650 * MIB,
                1_400 * MIB,
                &[],
                &["rust", "cargo", "native compilation", "shipping profile"],
            ),
            pack(
                "wgpu-cross-platform-sdk",
                "wgpu Cross-platform SDK",
                SdkPackKind::Rendering,
                520 * MIB,
                1_100 * MIB,
                &["rust-build-toolchain"],
                &[
                    "wgpu shaders",
                    "Metal",
                    "Vulkan",
                    "DirectX",
                    "render diagnostics",
                ],
            ),
            pack(
                "2d-production-library",
                "2D Production Library",
                SdkPackKind::Content,
                950 * MIB,
                2_000 * MIB,
                &[],
                &[
                    "materials",
                    "tile surfaces",
                    "particles",
                    "UI skins",
                    "shader examples",
                ],
            ),
            pack(
                "audio-font-library",
                "Audio and Font Library",
                SdkPackKind::AudioFonts,
                450 * MIB,
                950 * MIB,
                &[],
                &[
                    "sound effects",
                    "ambiences",
                    "UI audio",
                    "multilingual fonts",
                ],
            ),
            pack(
                "genre-template-library",
                "Cross-genre Template Library",
                SdkPackKind::Templates,
                320 * MIB,
                750 * MIB,
                &["2d-production-library", "audio-font-library"],
                &[
                    "platformer",
                    "top-down",
                    "RPG",
                    "survival",
                    "puzzle",
                    "RTS",
                    "visual novel",
                ],
            ),
            pack(
                "native-debug-symbols",
                "Native Debug Symbols",
                SdkPackKind::DebugSymbols,
                800 * MIB,
                1_550 * MIB,
                &["rust-build-toolchain", "wgpu-cross-platform-sdk"],
                &["symbolicated crashes", "profiling", "GPU diagnostics"],
            ),
            pack(
                "learning-examples",
                "Engine Learning Examples",
                SdkPackKind::Examples,
                350 * MIB,
                800 * MIB,
                &["genre-template-library"],
                &[
                    "sample scenes",
                    "Luau examples",
                    "visual graphs",
                    "API recipes",
                ],
            ),
            pack(
                "platform-export-sdks",
                "Platform Export SDKs",
                SdkPackKind::PlatformSdk,
                300 * MIB,
                700 * MIB,
                &["rust-build-toolchain"],
                &[
                    "desktop export",
                    "web toolchain",
                    "package signing metadata",
                ],
            ),
        ];
        let profiles = vec![
            profile(
                "lean",
                "Lean Development",
                "Compiler and renderer SDK for contributors who bring their own content.",
                &["rust-build-toolchain", "wgpu-cross-platform-sdk"],
                2 * GIB,
                4 * GIB,
            ),
            profile(
                "creator",
                "Creator 2D",
                "Complete authoring content, audio, fonts and cross-genre templates.",
                &[
                    "rust-build-toolchain",
                    "wgpu-cross-platform-sdk",
                    "2d-production-library",
                    "audio-font-library",
                    "genre-template-library",
                ],
                5 * GIB,
                7 * GIB,
            ),
            profile(
                "studio-heavy",
                "Studio Heavy",
                "Full production SDK with symbols, examples and platform export toolchains.",
                &[
                    "rust-build-toolchain",
                    "wgpu-cross-platform-sdk",
                    "2d-production-library",
                    "audio-font-library",
                    "genre-template-library",
                    "native-debug-symbols",
                    "learning-examples",
                    "platform-export-sdks",
                ],
                8 * GIB,
                10 * GIB,
            ),
        ];
        let total_available_installed_bytes = packs.iter().map(|pack| pack.installed_bytes).sum();
        Self {
            schema_version: SDK_PACK_CATALOG_SCHEMA_VERSION,
            packs,
            profiles,
            total_available_installed_bytes,
        }
    }

    pub fn pack(&self, id: &str) -> Option<&SdkPackManifest> {
        self.packs.iter().find(|pack| pack.id == id)
    }

    pub fn profile(&self, id: &str) -> Option<&SdkPackProfile> {
        self.profiles.iter().find(|profile| profile.id == id)
    }

    pub fn install_plan(
        &self,
        profile_id: &str,
        registry: &SdkPackRegistry,
    ) -> Result<SdkPackInstallPlan, String> {
        let profile = self
            .profile(profile_id)
            .ok_or_else(|| format!("Unknown SDK pack profile: {profile_id}"))?;
        let required_ids = self.resolve_dependencies(&profile.pack_ids)?;
        let installed = registry
            .installed
            .iter()
            .map(|pack| (pack.id.as_str(), pack))
            .collect::<BTreeMap<_, _>>();
        let mut install = Vec::new();
        let mut already_installed = Vec::new();
        let mut total_download_bytes = 0u64;
        let mut total_installed_bytes = 0u64;
        let mut projected_installed_bytes = 0u64;

        for id in required_ids {
            let manifest = self
                .pack(&id)
                .ok_or_else(|| format!("SDK pack dependency is missing: {id}"))?;
            projected_installed_bytes =
                projected_installed_bytes.saturating_add(manifest.installed_bytes);
            if installed.get(id.as_str()).is_some_and(|installed| {
                installed.version == manifest.version && installed.verified
            }) {
                already_installed.push(id);
                continue;
            }
            let reason = if installed.contains_key(id.as_str()) {
                "update_or_repair"
            } else if profile.pack_ids.contains(&id) {
                "profile"
            } else {
                "dependency"
            };
            total_download_bytes = total_download_bytes.saturating_add(manifest.download_bytes);
            total_installed_bytes = total_installed_bytes.saturating_add(manifest.installed_bytes);
            install.push(SdkPackInstallItem {
                id: manifest.id.clone(),
                label: manifest.label.clone(),
                version: manifest.version.clone(),
                reason: reason.to_string(),
                download_bytes: manifest.download_bytes,
                installed_bytes: manifest.installed_bytes,
                dependencies: manifest.dependencies.clone(),
            });
        }

        already_installed.sort();
        let meets_profile_target = projected_installed_bytes >= profile.target_min_bytes
            && projected_installed_bytes <= profile.target_max_bytes;
        Ok(SdkPackInstallPlan {
            profile_id: profile.id.clone(),
            profile_label: profile.label.clone(),
            install,
            already_installed,
            total_download_bytes,
            total_installed_bytes,
            projected_installed_bytes,
            target_min_bytes: profile.target_min_bytes,
            target_max_bytes: profile.target_max_bytes,
            meets_profile_target,
        })
    }

    pub fn validate(&self) -> SdkPackCatalogValidation {
        let mut issues = Vec::new();
        let mut pack_ids = BTreeSet::new();
        for pack in &self.packs {
            if !pack_ids.insert(pack.id.as_str()) {
                issues.push(format!("Duplicate SDK pack id: {}", pack.id));
            }
            if pack.version.trim().is_empty() {
                issues.push(format!("SDK pack {} has no version", pack.id));
            }
            if pack.download_bytes == 0 || pack.installed_bytes == 0 {
                issues.push(format!("SDK pack {} has an invalid size", pack.id));
            }
            if pack.download_bytes > pack.installed_bytes {
                issues.push(format!(
                    "SDK pack {} download is larger than its installed size",
                    pack.id
                ));
            }
            for dependency in &pack.dependencies {
                if self.pack(dependency).is_none() {
                    issues.push(format!(
                        "SDK pack {} references missing dependency {dependency}",
                        pack.id
                    ));
                }
            }
        }

        let mut profile_ids = BTreeSet::new();
        for profile in &self.profiles {
            if !profile_ids.insert(profile.id.as_str()) {
                issues.push(format!("Duplicate SDK profile id: {}", profile.id));
            }
            match self.install_plan(&profile.id, &SdkPackRegistry::default()) {
                Ok(plan) if !plan.meets_profile_target => issues.push(format!(
                    "SDK profile {} resolves outside its size target",
                    profile.id
                )),
                Ok(_) => {}
                Err(error) => issues.push(error),
            }
        }

        for pack in &self.packs {
            if self
                .resolve_dependencies(std::slice::from_ref(&pack.id))
                .is_err()
            {
                issues.push(format!("SDK pack {} has a dependency cycle", pack.id));
            }
        }
        issues.sort();
        issues.dedup();
        SdkPackCatalogValidation {
            valid: issues.is_empty(),
            issues,
        }
    }

    fn resolve_dependencies(&self, roots: &[String]) -> Result<Vec<String>, String> {
        fn visit(
            catalog: &SdkPackCatalog,
            id: &str,
            visiting: &mut BTreeSet<String>,
            resolved: &mut BTreeSet<String>,
            ordered: &mut Vec<String>,
        ) -> Result<(), String> {
            if resolved.contains(id) {
                return Ok(());
            }
            if !visiting.insert(id.to_string()) {
                return Err(format!("SDK pack dependency cycle at {id}"));
            }
            let pack = catalog
                .pack(id)
                .ok_or_else(|| format!("SDK pack dependency is missing: {id}"))?;
            for dependency in &pack.dependencies {
                visit(catalog, dependency, visiting, resolved, ordered)?;
            }
            visiting.remove(id);
            if resolved.insert(id.to_string()) {
                ordered.push(id.to_string());
            }
            Ok(())
        }

        let mut visiting = BTreeSet::new();
        let mut resolved = BTreeSet::new();
        let mut ordered = Vec::new();
        for id in roots {
            visit(self, id, &mut visiting, &mut resolved, &mut ordered)?;
        }
        Ok(ordered)
    }
}

fn pack(
    id: &str,
    label: &str,
    kind: SdkPackKind,
    download_bytes: u64,
    installed_bytes: u64,
    dependencies: &[&str],
    capabilities: &[&str],
) -> SdkPackManifest {
    SdkPackManifest {
        id: id.to_string(),
        label: label.to_string(),
        version: "0.9.3.4".to_string(),
        summary: format!("{label} optional package for the MiniForge production SDK."),
        kind,
        download_bytes,
        installed_bytes,
        dependencies: dependencies.iter().map(ToString::to_string).collect(),
        platforms: vec![
            "macos-arm64".to_string(),
            "macos-x86_64".to_string(),
            "linux-x86_64".to_string(),
            "windows-x86_64".to_string(),
        ],
        capabilities: capabilities.iter().map(ToString::to_string).collect(),
        archive_format: "zip".to_string(),
        checksum_policy: "sha256-release-manifest-required".to_string(),
        source_path: format!("packs/0.9.3.4/{id}-{{platform}}.zip"),
    }
}

fn profile(
    id: &str,
    label: &str,
    summary: &str,
    pack_ids: &[&str],
    target_min_bytes: u64,
    target_max_bytes: u64,
) -> SdkPackProfile {
    SdkPackProfile {
        id: id.to_string(),
        label: label.to_string(),
        summary: summary.to_string(),
        pack_ids: pack_ids.iter().map(ToString::to_string).collect(),
        target_min_bytes,
        target_max_bytes,
    }
}

#[cfg(test)]
mod tests {
    use super::{InstalledSdkPack, SdkPackCatalog, SdkPackRegistry};

    #[test]
    fn builtin_sdk_catalog_is_valid_and_heavy_profiles_hit_their_targets() {
        let catalog = SdkPackCatalog::builtin();
        let validation = catalog.validate();
        assert!(validation.valid, "{:?}", validation.issues);

        let creator = catalog
            .install_plan("creator", &SdkPackRegistry::default())
            .expect("creator plan");
        assert!(creator.meets_profile_target);
        assert!(creator.projected_installed_bytes >= 5 * 1024 * 1024 * 1024);

        let studio = catalog
            .install_plan("studio-heavy", &SdkPackRegistry::default())
            .expect("studio plan");
        assert!(studio.meets_profile_target);
        assert!(studio.projected_installed_bytes >= 8 * 1024 * 1024 * 1024);
        assert!(studio.projected_installed_bytes <= 10 * 1024 * 1024 * 1024);
    }

    #[test]
    fn install_plan_skips_verified_matching_packs_and_repairs_old_versions() {
        let catalog = SdkPackCatalog::builtin();
        let registry = SdkPackRegistry {
            installed: vec![
                InstalledSdkPack {
                    id: "rust-build-toolchain".to_string(),
                    version: "0.9.3.4".to_string(),
                    installed_bytes: 1_400 * 1024 * 1024,
                    verified: true,
                },
                InstalledSdkPack {
                    id: "wgpu-cross-platform-sdk".to_string(),
                    version: "0.9.2".to_string(),
                    installed_bytes: 1_000 * 1024 * 1024,
                    verified: true,
                },
            ],
        };
        let plan = catalog.install_plan("lean", &registry).expect("lean plan");
        assert_eq!(
            plan.already_installed,
            vec!["rust-build-toolchain".to_string()]
        );
        assert_eq!(plan.install.len(), 1);
        assert_eq!(plan.install[0].id, "wgpu-cross-platform-sdk");
        assert_eq!(plan.install[0].reason, "update_or_repair");
    }
}
