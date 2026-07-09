#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuItem {
    pub id: String,
    pub label: String,
    pub command: String,
    pub shortcut: Option<String>,
    pub enabled: bool,
    pub checked: bool,
    pub separator_before: bool,
}

impl MenuItem {
    pub fn new(id: &str, label: &str, command: &str) -> Self {
        Self {
            id: id.to_string(),
            label: label.to_string(),
            command: command.to_string(),
            shortcut: None,
            enabled: true,
            checked: false,
            separator_before: false,
        }
    }

    pub fn shortcut(mut self, shortcut: &str) -> Self {
        self.shortcut = Some(shortcut.to_string());
        self
    }

    pub fn separated(mut self) -> Self {
        self.separator_before = true;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Menu {
    pub id: String,
    pub label: String,
    pub items: Vec<MenuItem>,
}

impl Menu {
    pub fn new(id: &str, label: &str, items: Vec<MenuItem>) -> Self {
        Self {
            id: id.to_string(),
            label: label.to_string(),
            items,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MenuBar {
    pub open_menu: Option<String>,
    pub menus: Vec<Menu>,
    pub focused_item: Option<usize>,
}

impl MenuBar {
    pub fn editor_default() -> Self {
        Self {
            menus: default_editor_menus(),
            ..Default::default()
        }
    }

    pub fn register(&mut self, menu: Menu) {
        if let Some(existing) = self.menus.iter_mut().find(|item| item.id == menu.id) {
            *existing = menu;
        } else {
            self.menus.push(menu);
        }
    }

    pub fn open(&mut self, name: &str) -> bool {
        if self.menus.iter().any(|menu| menu.id == name) {
            self.open_menu = Some(name.to_string());
            self.focused_item = first_enabled_index(self.items(name));
            true
        } else {
            false
        }
    }

    pub fn toggle(&mut self, name: &str) -> bool {
        if self.is_open(name) {
            self.close();
            false
        } else {
            self.open(name)
        }
    }

    pub fn close(&mut self) {
        self.open_menu = None;
        self.focused_item = None;
    }

    pub fn is_open(&self, name: &str) -> bool {
        self.open_menu.as_deref() == Some(name)
    }

    pub fn open_menu(&self) -> Option<&Menu> {
        let id = self.open_menu.as_deref()?;
        self.menus.iter().find(|menu| menu.id == id)
    }

    pub fn menu(&self, name: &str) -> Option<&Menu> {
        self.menus.iter().find(|menu| menu.id == name)
    }

    pub fn items(&self, name: &str) -> &[MenuItem] {
        self.menu(name)
            .map(|menu| menu.items.as_slice())
            .unwrap_or_default()
    }

    pub fn set_item_enabled(&mut self, command: &str, enabled: bool) -> bool {
        let Some(item) = self
            .menus
            .iter_mut()
            .flat_map(|menu| menu.items.iter_mut())
            .find(|item| item.command == command)
        else {
            return false;
        };
        item.enabled = enabled;
        true
    }

    pub fn set_item_checked(&mut self, command: &str, checked: bool) -> bool {
        let Some(item) = self
            .menus
            .iter_mut()
            .flat_map(|menu| menu.items.iter_mut())
            .find(|item| item.command == command)
        else {
            return false;
        };
        item.checked = checked;
        true
    }

    pub fn move_focus(&mut self, delta: isize) -> Option<usize> {
        let menu = self.open_menu()?.clone();
        if menu.items.is_empty() {
            self.focused_item = None;
            return None;
        }
        let enabled = menu
            .items
            .iter()
            .enumerate()
            .filter_map(|(index, item)| item.enabled.then_some(index))
            .collect::<Vec<_>>();
        if enabled.is_empty() {
            self.focused_item = None;
            return None;
        }
        let current = self
            .focused_item
            .and_then(|index| enabled.iter().position(|candidate| *candidate == index))
            .unwrap_or(0);
        let next = (current as isize + delta).rem_euclid(enabled.len() as isize) as usize;
        self.focused_item = Some(enabled[next]);
        self.focused_item
    }

    pub fn activate(&mut self, menu_id: &str, item_index: usize) -> Option<String> {
        let command = self
            .items(menu_id)
            .get(item_index)
            .filter(|item| item.enabled)
            .map(|item| item.command.clone());
        if command.is_some() {
            self.close();
        }
        command
    }

    pub fn activate_focused(&mut self) -> Option<String> {
        let menu = self.open_menu.clone()?;
        self.activate(&menu, self.focused_item?)
    }
}

fn first_enabled_index(items: &[MenuItem]) -> Option<usize> {
    items.iter().position(|item| item.enabled)
}

fn item(id: &str, label: &str, command: &str) -> MenuItem {
    MenuItem::new(id, label, command)
}

fn default_editor_menus() -> Vec<Menu> {
    vec![
        Menu::new(
            "file",
            "File",
            vec![
                item("open-launcher", "Open Launcher", "open_launcher"),
                item("save-project", "Save Project", "save_project").shortcut("Cmd+Shift+S"),
                item("save-scene", "Save Scene", "save").shortcut("Cmd+S"),
                item("new-scene", "New Scene", "new_scene").shortcut("Cmd+N"),
                item("recover-autosave", "Recover Autosave", "recover_autosave").separated(),
                item("export-debug", "Export Debug", "export_debug"),
                item("export-release", "Export Release", "export_release"),
                item("export-project", "Export Project Zip", "export_project_zip"),
                item("import-project", "Import Project Zip", "import_project_zip"),
                item("package-debug", "Package Debug", "package_debug").separated(),
                item("package-release", "Package Release", "package_release"),
                item("refresh-assets", "Refresh Assets", "refresh"),
            ],
        ),
        Menu::new(
            "create",
            "Create",
            vec![
                item("game-object", "GameObject", "spawn_object"),
                item("unit", "Unit", "spawn_unit"),
                item("sprite", "Sprite Entity", "spawn_sprite"),
                item("ui-hud", "UI Canvas HUD", "ui_canvas_hud").separated(),
                item("ui-label", "UI Canvas Label", "ui_canvas_label"),
                item("sound", "Sound Cue", "asset_sound").separated(),
                item("material", "Material", "asset_material"),
            ],
        ),
        Menu::new(
            "view",
            "View",
            vec![
                item("preferences", "Preferences", "preferences"),
                item("command-palette", "Command Palette", "command_palette").shortcut("Cmd+P"),
                item("script-window", "Script Window", "script_window").separated(),
                item("blueprint-picker", "Blueprint Picker", "blueprint_picker"),
                item("play-window", "Play Window", "play_window"),
                item("toggle-browser", "Toggle Browser", "toggle_browser").separated(),
                item("scene-browser", "Scene Browser", "scene_browser"),
                item("sprite-editor", "Sprite Editor Window", "sprite_editor"),
                item("python-tools", "Python Tools", "python_tools"),
                item("toggle-hierarchy", "Toggle Hierarchy", "toggle_hierarchy").separated(),
                item("toggle-inspector", "Toggle Inspector", "toggle_inspector"),
                item("smart-snap", "Smart Snapping", "toggle_smart_snap"),
                item(
                    "collisions",
                    "Collision Overlay",
                    "toggle_collision_overlay",
                ),
                item("camera-frame", "Camera Frame", "toggle_camera_frame"),
            ],
        ),
        Menu::new(
            "project",
            "Project",
            vec![
                item("validate", "Validate", "validate"),
                item("manifest", "Build Manifest", "manifest"),
                item("topdown", "TopDown Starter", "starter_topdown").separated(),
                item("platformer", "Platformer Starter", "starter_platformer"),
                item(
                    "inventory-graph",
                    "Inventory/Economy Graph",
                    "create_graph_inventory",
                ),
                item("quest-graph", "Quest/Ability Graph", "create_graph_quest"),
                item("scene-report", "Python Scene Report", "python_scene_report").separated(),
                item("automation", "Python Automation Tools", "python_tools"),
            ],
        ),
        Menu::new(
            "rts",
            "RTS",
            vec![
                item("skirmish", "RTS Skirmish", "rts_skirmish"),
                item("command-center", "Command Center", "spawn_rts_base"),
                item("queue-worker", "Queue Worker", "queue_worker"),
                item("barracks", "Place Barracks", "place_barracks"),
                item(
                    "production-graph",
                    "RTS Production Graph",
                    "create_graph_rts_economy",
                ),
            ],
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_menu_routes_visible_items_to_commands() {
        let mut bar = MenuBar::editor_default();
        assert!(bar.open("file"));
        assert_eq!(
            bar.open_menu().map(|menu| menu.label.as_str()),
            Some("File")
        );
        assert_eq!(bar.activate("file", 2).as_deref(), Some("save"));
        assert!(bar.open_menu.is_none());
    }

    #[test]
    fn keyboard_focus_skips_disabled_items_and_wraps() {
        let mut bar = MenuBar::editor_default();
        bar.set_item_enabled("save_project", false);
        bar.open("file");
        assert_eq!(bar.focused_item, Some(0));
        assert_eq!(bar.move_focus(1), Some(2));
        assert_eq!(bar.move_focus(-1), Some(0));
    }
}
