use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::engine::asset_tools::AssetTools;
use crate::engine::miniforge_2d::validation::ValidationReport2D;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EditorLayout2D {
    pub theme: String,
    pub left_panel_width: f32,
    pub right_panel_width: f32,
    pub bottom_panel_height: f32,
    pub active_bottom_tab: String,
    pub scene_view_grid: bool,
    pub show_console: bool,
    pub show_problems: bool,
    pub panels: Vec<EditorPanel2D>,
    pub bottom_tabs: Vec<String>,
    #[serde(default)]
    pub main_menu: EditorMainMenu2D,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EditorPanel2D {
    pub id: String,
    pub title: String,
    pub region: String,
    pub visible: bool,
    pub dockable: bool,
    pub resizable: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct EditorMainMenu2D {
    pub groups: Vec<EditorMenuGroup2D>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EditorMenuGroup2D {
    pub title: String,
    pub items: Vec<EditorMenuItem2D>,
    #[serde(default)]
    pub submenus: Vec<EditorSubMenu2D>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EditorSubMenu2D {
    pub id: String,
    pub label: String,
    pub icon: String,
    pub items: Vec<EditorMenuItem2D>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EditorMenuItem2D {
    pub id: String,
    pub label: String,
    pub command: String,
    pub shortcut: Option<String>,
    pub icon: String,
}

impl Default for EditorLayout2D {
    fn default() -> Self {
        Self {
            theme: "dark".to_string(),
            left_panel_width: 280.0,
            right_panel_width: 340.0,
            bottom_panel_height: 260.0,
            active_bottom_tab: "Content Browser".to_string(),
            scene_view_grid: true,
            show_console: true,
            show_problems: true,
            panels: vec![
                panel("menu_bar", "Menu Bar", "top", true, false, false),
                panel("toolbar", "Toolbar", "top", true, false, false),
                panel("world_outliner", "World Outliner", "left", true, true, true),
                panel("scene_view", "Scene View 2D", "center", true, false, true),
                panel("inspector", "Details", "right", true, true, true),
                panel("ui_designer", "UI Designer", "center", false, true, true),
                panel(
                    "blueprint_editor",
                    "Blueprint Editor",
                    "center",
                    false,
                    true,
                    true,
                ),
                panel(
                    "content_browser",
                    "Content Browser",
                    "bottom",
                    true,
                    true,
                    true,
                ),
                panel("console", "Console", "bottom", true, true, true),
                panel("problems", "Problems", "bottom", true, true, true),
                panel("output", "Output", "bottom", true, true, true),
            ],
            bottom_tabs: vec![
                "Content Browser".to_string(),
                "Console".to_string(),
                "Problems".to_string(),
                "Output".to_string(),
            ],
            main_menu: EditorMainMenu2D::default_main_menu(),
        }
    }
}

impl EditorLayout2D {
    pub fn load_or_default(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref();
        if !path.exists() {
            let layout = Self::default();
            layout.save(path)?;
            return Ok(layout);
        }
        let value = AssetTools::read_json(path)?;
        Ok(serde_json::from_value(value).unwrap_or_default())
    }

    pub fn save(&self, path: impl AsRef<Path>) -> io::Result<()> {
        AssetTools::write_json(path, &serde_json::to_value(self).unwrap_or_default())
    }

    pub fn reset_to_default(&mut self) {
        *self = Self::default();
    }

    pub fn visible_panels_by_region(&self, region: &str) -> Vec<&EditorPanel2D> {
        self.panels
            .iter()
            .filter(|panel| panel.visible && panel.region == region)
            .collect()
    }

    pub fn command_palette_entries(&self) -> Vec<String> {
        self.main_menu
            .groups
            .iter()
            .flat_map(|group| {
                group.items.iter().chain(
                    group
                        .submenus
                        .iter()
                        .flat_map(|submenu| submenu.items.iter()),
                )
            })
            .map(|item| format!("{}:{}", item.label, item.command))
            .collect()
    }

    pub fn submenu_entries(&self, group_title: &str, submenu_id: &str) -> Vec<&EditorMenuItem2D> {
        self.main_menu
            .groups
            .iter()
            .find(|group| group.title == group_title)
            .and_then(|group| {
                group
                    .submenus
                    .iter()
                    .find(|submenu| submenu.id == submenu_id)
            })
            .map(|submenu| submenu.items.iter().collect())
            .unwrap_or_default()
    }

    pub fn validate(&self) -> ValidationReport2D {
        let mut report = ValidationReport2D::default();
        for required in [
            "menu_bar",
            "toolbar",
            "world_outliner",
            "scene_view",
            "inspector",
            "content_browser",
            "blueprint_editor",
            "ui_designer",
        ] {
            if !self.panels.iter().any(|panel| panel.id == required) {
                report.error(
                    "layout_missing_panel",
                    required,
                    format!("Falta panel obligatorio de editor: {required}"),
                );
            }
        }
        if self.theme != "dark" {
            report.warning(
                "layout_theme",
                "theme",
                "La guia pide tema oscuro por defecto.",
            );
        }
        if !self
            .main_menu
            .groups
            .iter()
            .any(|group| group.title == "Create")
        {
            report.warning(
                "layout_missing_create_menu",
                "main_menu",
                "Menu principal sin grupo Create.",
            );
        }
        report
    }
}

impl EditorMainMenu2D {
    pub fn default_main_menu() -> Self {
        let groups = vec![
            menu_group(
                "File",
                &[
                    (
                        "new_project",
                        "New Project",
                        "project.new",
                        Some("Ctrl+N"),
                        "file-plus",
                    ),
                    (
                        "open_project",
                        "Open Project",
                        "project.open",
                        Some("Ctrl+O"),
                        "folder-open",
                    ),
                    (
                        "save_scene",
                        "Save Scene",
                        "scene.save",
                        Some("Ctrl+S"),
                        "save",
                    ),
                    (
                        "package_release",
                        "Package Release",
                        "build.package_release",
                        None,
                        "archive",
                    ),
                ],
            ),
            menu_group_with_submenus(
                "Create",
                &[
                    ("create_actor", "Actor", "create.actor", Some("A"), "box"),
                    (
                        "create_ui_menu",
                        "Main Menu UI",
                        "create.ui.main_menu",
                        None,
                        "panel-top",
                    ),
                    (
                        "create_blueprint",
                        "Blueprint Graph",
                        "create.blueprint",
                        Some("Ctrl+G"),
                        "workflow",
                    ),
                    (
                        "create_luau",
                        "Luau Script",
                        "create.luau",
                        None,
                        "file-code",
                    ),
                ],
                &[
                    submenu(
                        "create_2d",
                        "2D",
                        "layers",
                        &[
                            (
                                "create_sprite",
                                "Sprite2D",
                                "create.sprite2d",
                                None,
                                "image",
                            ),
                            (
                                "create_tilemap",
                                "Tilemap2D",
                                "create.tilemap2d",
                                None,
                                "grid-3x3",
                            ),
                            (
                                "create_particles",
                                "Particles2D",
                                "create.particles2d",
                                None,
                                "sparkles",
                            ),
                        ],
                    ),
                    submenu(
                        "create_gameplay",
                        "Gameplay",
                        "gamepad-2",
                        &[
                            ("create_pawn", "Pawn2D", "create.pawn2d", None, "circle-dot"),
                            (
                                "create_ai",
                                "AI Controller",
                                "create.ai_controller2d",
                                None,
                                "brain",
                            ),
                            ("create_ability", "Ability", "create.ability2d", None, "zap"),
                        ],
                    ),
                ],
            ),
            menu_group_with_submenus(
                "Window",
                &[
                    (
                        "open_content",
                        "Content Browser",
                        "window.content_browser",
                        None,
                        "folder",
                    ),
                    (
                        "open_outliner",
                        "World Outliner",
                        "window.world_outliner",
                        None,
                        "list-tree",
                    ),
                    (
                        "open_details",
                        "Details",
                        "window.details",
                        None,
                        "sliders-horizontal",
                    ),
                    (
                        "open_ui_designer",
                        "UI Designer",
                        "window.ui_designer",
                        None,
                        "layout-template",
                    ),
                    (
                        "open_blueprints",
                        "Blueprint Editor",
                        "window.blueprints",
                        None,
                        "workflow",
                    ),
                ],
                &[
                    submenu(
                        "window_authoring",
                        "Authoring",
                        "panel-top",
                        &[
                            (
                                "open_tilemap_editor",
                                "Tilemap Editor",
                                "window.tilemap_editor",
                                None,
                                "grid-3x3",
                            ),
                            (
                                "open_spriteframes",
                                "SpriteFrames",
                                "window.spriteframes",
                                None,
                                "film",
                            ),
                            (
                                "open_particles",
                                "Particles",
                                "window.particles",
                                None,
                                "sparkles",
                            ),
                        ],
                    ),
                    submenu(
                        "window_debug",
                        "Debug",
                        "bug",
                        &[
                            (
                                "open_service_graph",
                                "Service Graph",
                                "window.service_graph",
                                None,
                                "network",
                            ),
                            (
                                "open_resource_loader",
                                "Resource Loader",
                                "window.resource_loader",
                                None,
                                "database",
                            ),
                            (
                                "open_overlays",
                                "Scene Overlays",
                                "window.scene_overlays",
                                None,
                                "scan-line",
                            ),
                        ],
                    ),
                ],
            ),
            menu_group(
                "Play",
                &[
                    ("play", "Play", "play.start", Some("F5"), "play"),
                    ("pause", "Pause", "play.pause", None, "pause"),
                    ("stop", "Stop", "play.stop", Some("Esc"), "square"),
                ],
            ),
            menu_group_with_submenus(
                "Tools",
                &[
                    (
                        "validate",
                        "Validate Project",
                        "tools.validate",
                        None,
                        "badge-check",
                    ),
                    (
                        "backend_plan",
                        "Backend Plan",
                        "tools.backend_plan",
                        None,
                        "server",
                    ),
                    (
                        "reload_assets",
                        "Reload Assets",
                        "tools.reload_assets",
                        None,
                        "refresh-cw",
                    ),
                ],
                &[submenu(
                    "tools_apple",
                    "Apple",
                    "apple",
                    &[
                        (
                            "xcode_debug_plan",
                            "Xcode Debug Plan",
                            "tools.xcode.debug_plan",
                            None,
                            "hammer",
                        ),
                        (
                            "xcode_release_plan",
                            "Xcode Release Plan",
                            "tools.xcode.release_plan",
                            None,
                            "package",
                        ),
                    ],
                )],
            ),
        ];
        Self { groups }
    }
}

fn panel(
    id: &str,
    title: &str,
    region: &str,
    visible: bool,
    dockable: bool,
    resizable: bool,
) -> EditorPanel2D {
    EditorPanel2D {
        id: id.to_string(),
        title: title.to_string(),
        region: region.to_string(),
        visible,
        dockable,
        resizable,
    }
}

fn menu_group(title: &str, items: &[(&str, &str, &str, Option<&str>, &str)]) -> EditorMenuGroup2D {
    menu_group_with_submenus(title, items, &[])
}

fn menu_group_with_submenus(
    title: &str,
    items: &[(&str, &str, &str, Option<&str>, &str)],
    submenus: &[EditorSubMenu2D],
) -> EditorMenuGroup2D {
    EditorMenuGroup2D {
        title: title.to_string(),
        items: items
            .iter()
            .map(|(id, label, command, shortcut, icon)| EditorMenuItem2D {
                id: (*id).to_string(),
                label: (*label).to_string(),
                command: (*command).to_string(),
                shortcut: shortcut.map(str::to_string),
                icon: (*icon).to_string(),
            })
            .collect(),
        submenus: submenus.to_vec(),
    }
}

fn submenu(
    id: &str,
    label: &str,
    icon: &str,
    items: &[(&str, &str, &str, Option<&str>, &str)],
) -> EditorSubMenu2D {
    EditorSubMenu2D {
        id: id.to_string(),
        label: label.to_string(),
        icon: icon.to_string(),
        items: menu_group(label, items).items,
    }
}
