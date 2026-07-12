use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::{env, fs, io};

use macroquad::prelude::*;

use crate::core::game::Game;
use crate::engine::asset_database::AssetRecord;
use crate::engine::asset_importers::SpriteSheetImporter;
use crate::engine::asset_tools::AssetTools;
use crate::engine::command_palette::CommandPalette;
use crate::engine::component::{
    advanced_component_category, advanced_component_types, default_component,
};
use crate::engine::content_drag::{DragPayload, DropOutcome};
use crate::engine::crash_reporter::{CrashReporter, CrashReporterConfig};
use crate::engine::document_manager::CloseDocumentChoice;
use crate::engine::editor_command::{EditorCommandKind, EditorSnapshot};
use crate::engine::editor_python::{
    PythonAutomationHost, PythonBatchReport, PythonEditorContext, PythonEditorOperation,
    batch_convert_sprites, batch_import_assets, generate_paged_sprite_atlases,
};
use crate::engine::editor_spatial_tools_2d::{
    AlignMode2D, CameraFrame2D, EditorSpatialTools2D, SmartSnapGuide2D, SmartSnapSettings2D,
};
use crate::engine::editor_ui::{
    EditorClipboard, EditorFileWatcher, EditorIcon, EditorSyntaxHighlighter, fuzzy_rank,
    open_in_default_application,
};
use crate::engine::editor_workspace::WorkspaceMode;
use crate::engine::engine_programming::{VisualGraphNodeView, VisualGraphView};
use crate::engine::inspector_editor::{InspectorEditor, InspectorField};
use crate::engine::miniforge_2d::paper2d::SpriteFrames2D;
use crate::engine::project_launcher::{EguiProjectLauncher, LauncherTemplate};
use crate::engine::project_templates::ProjectTemplates;
use crate::engine::runtime_exporter::ExportProfile;
use crate::engine::runtime_manifest_loader::RuntimeManifestLoader;
use crate::engine::safe_mode::SafeModeSettings;
use crate::engine::scene_view_tools::SceneViewTools;
use crate::engine::session_recovery::{
    EditorSessionSnapshot, SessionRecoveryManager, SessionUiState,
};
use crate::engine::sprite_editor::{
    SpriteAnimationClipDraft, SpriteAnimationPlaybackSample, SpriteColor,
};
use crate::engine::tile_brush::TileBrushMode;
use crate::engine::ui::{Button as UiButton, EditorTool, MenuBar, Toolbar as EditorToolbar};
use crate::engine::ui_canvas::{
    UiCanvasElement, UiCanvasGizmoHandleKind, UiCanvasRoot, layout_element_pixels,
    ui_canvases_from_value,
};
use crate::engine::vector_canvas_2d::{
    VectorGeometry2D, VectorMesh2D, VectorPath2D, VectorPoint2D, VectorStyle2D, translation_gizmo,
};
use crate::entities::game_object::GameObject;
use crate::systems::command_system::CommandSystem;
use crate::systems::rts_system::RTSSystem;
use serde_json::{Value, json};

#[derive(Debug, Default, Clone)]
struct Args {
    project: Option<PathBuf>,
    runtime: bool,
    no_launcher: bool,
    force_launcher: bool,
    headless_once: bool,
    create_project: Option<PathBuf>,
    create_template: String,
    force_create_project: bool,
    safe_mode: bool,
}

#[derive(Debug, Clone, Copy)]
struct RectSpec {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

#[derive(Debug, Clone, Copy)]
enum TextAlign {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Copy)]
struct Viewport {
    rect: RectSpec,
    tile: f32,
    zoom: f32,
    camera_x: f32,
    camera_y: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BottomTab {
    Console,
    Assets,
    Programming,
    Prefabs,
    Scenes,
    Sprites,
    Profiler,
}

impl BottomTab {
    fn label(self) -> &'static str {
        match self {
            Self::Console => "Console",
            Self::Assets => "Browser",
            Self::Programming => "Graph",
            Self::Prefabs => "Prefabs",
            Self::Scenes => "Scenes",
            Self::Sprites => "Sprites",
            Self::Profiler => "Profiler",
        }
    }

    fn next(self) -> Self {
        match self {
            Self::Console => Self::Assets,
            Self::Assets => Self::Programming,
            Self::Programming => Self::Prefabs,
            Self::Prefabs => Self::Scenes,
            Self::Scenes => Self::Sprites,
            Self::Sprites => Self::Profiler,
            Self::Profiler => Self::Console,
        }
    }

    fn from_label(label: &str) -> Self {
        match label {
            "Browser" => Self::Assets,
            "Graph" => Self::Programming,
            "Prefabs" => Self::Prefabs,
            "Scenes" => Self::Scenes,
            "Sprites" => Self::Sprites,
            "Profiler" => Self::Profiler,
            _ => Self::Console,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TopMenu {
    File,
    Create,
    View,
    Project,
    Rts,
}

impl TopMenu {
    fn label(self) -> &'static str {
        match self {
            Self::File => "File",
            Self::Create => "Create",
            Self::View => "View",
            Self::Project => "Project",
            Self::Rts => "RTS",
        }
    }

    fn id(self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Create => "create",
            Self::View => "view",
            Self::Project => "project",
            Self::Rts => "rts",
        }
    }

    fn from_id(id: &str) -> Option<Self> {
        match id {
            "file" => Some(Self::File),
            "create" => Some(Self::Create),
            "view" => Some(Self::View),
            "project" => Some(Self::Project),
            "rts" => Some(Self::Rts),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FloatingWindowKind {
    Script,
    Play,
    BlueprintPicker,
    Sprite,
    PythonTools,
}

#[derive(Debug, Clone)]
struct TextEditState {
    entity_id: u64,
    target: String,
    key: String,
    buffer: String,
}

#[derive(Debug, Clone)]
struct EditorPreferences {
    prefer_vscode: bool,
    external_editor_command: String,
    open_external_on_script: bool,
    keyboard_layout_hint: String,
}

impl Default for EditorPreferences {
    fn default() -> Self {
        Self {
            prefer_vscode: false,
            external_editor_command: "code".to_string(),
            open_external_on_script: false,
            keyboard_layout_hint: "international".to_string(),
        }
    }
}

struct EditorState {
    paused: bool,
    show_console: bool,
    show_grid: bool,
    show_hierarchy: bool,
    show_inspector: bool,
    command_palette: CommandPalette,
    menu_bar: MenuBar,
    toolbar: EditorToolbar,
    bottom_tab: BottomTab,
    tile_brush: i32,
    tile_brush_mode: TileBrushMode,
    snap_to_grid: bool,
    smart_snap: bool,
    show_collisions: bool,
    show_camera_frame: bool,
    zoom_target: f64,
    snap_guides: Vec<SmartSnapGuide2D>,
    drag_entity: Option<u64>,
    drag_world_last: (f32, f32),
    drag_collision_vertex: Option<(u64, usize)>,
    drag_entity_before: Option<EditorSnapshot>,
    paint_start: Option<(usize, usize)>,
    last_painted_cell: Option<(usize, usize)>,
    selected_asset_path: Option<String>,
    content_source: String,
    content_type_filter: Option<String>,
    content_search: String,
    content_search_active: bool,
    content_scroll: f32,
    drag_payload: Option<DragPayload>,
    active_text_field: Option<TextEditState>,
    code_editor_active: bool,
    code_cursor_line: usize,
    code_cursor_column: usize,
    code_scroll_line: usize,
    graph_selected_node: Option<String>,
    graph_connect_from: Option<String>,
    graph_connect_pin: String,
    graph_node_search: String,
    graph_node_search_active: bool,
    graph_template_search: String,
    graph_template_search_active: bool,
    script_search: String,
    script_search_active: bool,
    graph_drag_node: Option<String>,
    graph_drag_offset: (f32, f32),
    graph_pan: (f32, f32),
    graph_panning: bool,
    graph_pan_last: (f32, f32),
    scene_panning: bool,
    scene_pan_last: (f32, f32),
    scene_pan_moved: bool,
    hierarchy_scroll: f32,
    hierarchy_context_entity: Option<u64>,
    hierarchy_context_pos: (f32, f32),
    drag_ui_offset: (f32, f32),
    selected_ui_canvas_id: Option<String>,
    selected_ui_element_id: Option<String>,
    drag_ui_canvas_element: bool,
    drag_ui_canvas_handle: Option<UiCanvasGizmoHandleKind>,
    drag_ui_canvas_before: Option<EditorSnapshot>,
    drag_ui_canvas_last: (f32, f32),
    script_window_open: bool,
    script_window_rect: RectSpec,
    pending_script_close: Option<PathBuf>,
    play_window_open: bool,
    play_window_rect: RectSpec,
    blueprint_picker_open: bool,
    blueprint_picker_rect: RectSpec,
    sprite_window_open: bool,
    sprite_window_rect: RectSpec,
    python_tools_open: bool,
    python_tools_rect: RectSpec,
    preferences_open: bool,
    editor_preferences: EditorPreferences,
    floating_drag: Option<(FloatingWindowKind, f32, f32)>,
    launcher_overlay: Option<LauncherUiState>,
    external_play_child: Option<Child>,
    external_play_path: Option<PathBuf>,
    phosphor_font: Option<Font>,
    syntax_highlighter: EditorSyntaxHighlighter,
    file_watcher: Option<EditorFileWatcher>,
    session_recovery: Option<SessionRecoveryManager>,
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            paused: false,
            show_console: true,
            show_grid: true,
            show_hierarchy: true,
            show_inspector: true,
            command_palette: CommandPalette::with_commands(
                COMMAND_PALETTE_ITEMS
                    .iter()
                    .map(|(label, command)| format!("{label} {command}")),
            ),
            menu_bar: MenuBar::editor_default(),
            toolbar: EditorToolbar::default(),
            bottom_tab: BottomTab::Console,
            tile_brush: 1,
            tile_brush_mode: TileBrushMode::Pencil,
            snap_to_grid: true,
            smart_snap: true,
            show_collisions: false,
            show_camera_frame: false,
            zoom_target: 1.0,
            snap_guides: Vec::new(),
            drag_entity: None,
            drag_world_last: (0.0, 0.0),
            drag_collision_vertex: None,
            drag_entity_before: None,
            paint_start: None,
            last_painted_cell: None,
            selected_asset_path: None,
            content_source: "Content".to_string(),
            content_type_filter: None,
            content_search: String::new(),
            content_search_active: false,
            content_scroll: 0.0,
            drag_payload: None,
            active_text_field: None,
            code_editor_active: false,
            code_cursor_line: 0,
            code_cursor_column: 0,
            code_scroll_line: 0,
            graph_selected_node: None,
            graph_connect_from: None,
            graph_connect_pin: "exec".to_string(),
            graph_node_search: String::new(),
            graph_node_search_active: false,
            graph_template_search: String::new(),
            graph_template_search_active: false,
            script_search: String::new(),
            script_search_active: false,
            graph_drag_node: None,
            graph_drag_offset: (0.0, 0.0),
            graph_pan: (24.0, 24.0),
            graph_panning: false,
            graph_pan_last: (0.0, 0.0),
            scene_panning: false,
            scene_pan_last: (0.0, 0.0),
            scene_pan_moved: false,
            hierarchy_scroll: 0.0,
            hierarchy_context_entity: None,
            hierarchy_context_pos: (0.0, 0.0),
            drag_ui_offset: (0.0, 0.0),
            selected_ui_canvas_id: None,
            selected_ui_element_id: None,
            drag_ui_canvas_element: false,
            drag_ui_canvas_handle: None,
            drag_ui_canvas_before: None,
            drag_ui_canvas_last: (0.0, 0.0),
            script_window_open: false,
            script_window_rect: RectSpec {
                x: 86.0,
                y: 96.0,
                w: 920.0,
                h: 560.0,
            },
            pending_script_close: None,
            play_window_open: false,
            play_window_rect: RectSpec {
                x: 126.0,
                y: 112.0,
                w: 960.0,
                h: 540.0,
            },
            blueprint_picker_open: false,
            blueprint_picker_rect: RectSpec {
                x: 180.0,
                y: 130.0,
                w: 760.0,
                h: 520.0,
            },
            sprite_window_open: false,
            sprite_window_rect: RectSpec {
                x: 96.0,
                y: 72.0,
                w: 1120.0,
                h: 650.0,
            },
            python_tools_open: false,
            python_tools_rect: RectSpec {
                x: 210.0,
                y: 110.0,
                w: 780.0,
                h: 560.0,
            },
            preferences_open: false,
            editor_preferences: EditorPreferences::default(),
            floating_drag: None,
            launcher_overlay: None,
            external_play_child: None,
            external_play_path: None,
            phosphor_font: load_ttf_font_from_bytes(egui_phosphor::Variant::Regular.font_bytes())
                .ok(),
            syntax_highlighter: EditorSyntaxHighlighter::default(),
            file_watcher: None,
            session_recovery: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LauncherTextField {
    ProjectName,
    ProjectLocation,
    OpenPath,
}

struct LauncherUiState {
    launcher: EguiProjectLauncher,
    active_field: Option<LauncherTextField>,
    close_requested: bool,
}

fn parse_args() -> Args {
    let mut parsed = Args::default();
    let mut args = env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--project" => {
                if let Some(path) = args.next() {
                    parsed.project = Some(PathBuf::from(path));
                }
            }
            "--runtime" => parsed.runtime = true,
            "--no-launcher" => parsed.no_launcher = true,
            "--launcher" => parsed.force_launcher = true,
            "--headless-once" => parsed.headless_once = true,
            "--create-project" => {
                if let Some(path) = args.next() {
                    parsed.create_project = Some(PathBuf::from(path));
                }
            }
            "--template" => {
                if let Some(template) = args.next() {
                    parsed.create_template = template;
                }
            }
            "--force" | "--overwrite" => parsed.force_create_project = true,
            "--safe-mode" => parsed.safe_mode = true,
            "--help" | "-h" => {
                println!("MiniForge editor/runtime launcher");
                println!("  --project <path>  Project path to open");
                println!("  --runtime         Start in play/runtime mode");
                println!("  --launcher        Force the project launcher");
                println!("  --no-launcher     Open project directly");
                println!("  --headless-once   Run one frame and exit, useful for CI");
                println!("  --safe-mode       Disable scripts, graphs and native plugins");
                println!("  --create-project <path> --template <name> [--force]");
                println!("                    Create a project with a MiniForge template and exit");
                std::process::exit(0);
            }
            _ => {}
        }
    }

    parsed
}

pub fn editor_window_conf() -> Conf {
    Conf {
        window_title: crate::version_label(),
        window_width: 1360,
        window_height: 820,
        high_dpi: true,
        sample_count: 4,
        window_resizable: true,
        ..Default::default()
    }
}

pub fn runtime_player_window_conf() -> Conf {
    Conf {
        window_title: "MiniForge Runtime Player".to_string(),
        window_width: 1280,
        window_height: 720,
        high_dpi: true,
        sample_count: 2,
        window_resizable: true,
        ..Default::default()
    }
}

async fn run_startup_launcher() -> Option<PathBuf> {
    let mut state = new_launcher_state();
    loop {
        if is_key_pressed(KeyCode::Escape) {
            return None;
        }
        handle_launcher_text_input(&mut state);

        clear_background(ui_bg());
        draw_launcher_background(screen_width(), screen_height());
        if let Some(path) = draw_launcher_ui(&mut state) {
            return Some(path);
        }
        if state.close_requested {
            return None;
        }

        next_frame().await;
    }
}

fn new_launcher_state() -> LauncherUiState {
    let workspace_root = env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("projects");
    let mut state = LauncherUiState {
        launcher: EguiProjectLauncher::new(&workspace_root),
        active_field: None,
        close_requested: false,
    };
    let _ = state.launcher.discover_recent_projects();
    state
}

fn handle_launcher_text_input(state: &mut LauncherUiState) {
    let Some(active) = state.active_field else {
        return;
    };
    let buffer = match active {
        LauncherTextField::ProjectName => &mut state.launcher.project_name,
        LauncherTextField::ProjectLocation => &mut state.launcher.project_location,
        LauncherTextField::OpenPath => &mut state.launcher.open_path,
    };
    while let Some(character) = get_char_pressed() {
        if !character.is_control() {
            buffer.push(character);
        }
    }
    if is_key_pressed(KeyCode::Backspace) {
        buffer.pop();
    }
    if is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter) {
        state.active_field = None;
    }
}

fn draw_launcher_background(sw: f32, sh: f32) {
    draw_gradient_rect(
        RectSpec {
            x: 0.0,
            y: 0.0,
            w: sw,
            h: sh,
        },
        Color::from_rgba(15, 18, 24, 255),
        Color::from_rgba(8, 10, 14, 255),
    );
    let band_h = (sh * 0.30).max(180.0);
    draw_gradient_rect(
        RectSpec {
            x: 0.0,
            y: 0.0,
            w: sw,
            h: band_h,
        },
        Color::from_rgba(25, 32, 44, 255),
        Color::from_rgba(14, 18, 26, 255),
    );
    draw_rectangle(
        0.0,
        band_h - 1.0,
        sw,
        1.0,
        Color::from_rgba(111, 226, 196, 95),
    );
}

fn draw_launcher_ui(state: &mut LauncherUiState) -> Option<PathBuf> {
    let sw = screen_width();
    let sh = screen_height();
    let panel = RectSpec {
        x: (sw * 0.5 - 520.0).max(18.0),
        y: (sh * 0.5 - 325.0).max(18.0),
        w: sw.min(1040.0) - 36.0,
        h: sh.min(650.0) - 36.0,
    };
    draw_surface(panel, true);
    draw_gradient_rect(
        RectSpec {
            x: panel.x,
            y: panel.y,
            w: panel.w,
            h: 58.0,
        },
        Color::from_rgba(35, 46, 65, 255),
        Color::from_rgba(25, 31, 43, 255),
    );
    draw_rectangle_lines(
        panel.x,
        panel.y,
        panel.w,
        panel.h,
        1.0,
        Color::from_rgba(87, 107, 138, 255),
    );
    draw_macos_chrome(RectSpec {
        x: panel.x + 8.0,
        y: panel.y + 8.0,
        w: panel.w,
        h: 44.0,
    });
    draw_text("MiniForge", panel.x + 82.0, panel.y + 34.0, 25.0, ui_text());
    draw_text(
        &crate::version_label(),
        panel.x + panel.w - 310.0,
        panel.y + 32.0,
        15.0,
        ui_text_muted(),
    );

    let left = RectSpec {
        x: panel.x + 22.0,
        y: panel.y + 76.0,
        w: panel.w * 0.48,
        h: panel.h - 98.0,
    };
    let right = RectSpec {
        x: left.x + left.w + 24.0,
        y: left.y,
        w: panel.w - left.w - 68.0,
        h: left.h,
    };

    draw_text("Crear proyecto", left.x, left.y, 22.0, ui_text());
    draw_launcher_text_field(
        &mut state.active_field,
        LauncherTextField::ProjectName,
        RectSpec {
            x: left.x,
            y: left.y + 22.0,
            w: left.w,
            h: 34.0,
        },
        "Nombre",
        &state.launcher.project_name,
    );
    draw_launcher_text_field(
        &mut state.active_field,
        LauncherTextField::ProjectLocation,
        RectSpec {
            x: left.x,
            y: left.y + 68.0,
            w: left.w,
            h: 34.0,
        },
        "Ubicacion",
        &state.launcher.project_location,
    );
    let mut tx = left.x;
    for template in LauncherTemplate::all() {
        let width = match template {
            LauncherTemplate::Platformer => 92.0,
            LauncherTemplate::TopDown => 84.0,
            _ => 68.0,
        };
        if button(
            tx,
            left.y + 118.0,
            width,
            26.0,
            template.label(),
            state.launcher.selected_template == template,
        ) {
            state.launcher.selected_template = template;
        }
        tx += width + 8.0;
    }

    if button(left.x, left.y + 162.0, 132.0, 30.0, "Crear", false) {
        match state.launcher.create_new_project() {
            Ok(path) => return Some(path),
            Err(error) => state.launcher.status = error.to_string(),
        }
    }
    if button(
        left.x + 142.0,
        left.y + 162.0,
        148.0,
        30.0,
        "Buscar locales",
        false,
    ) {
        match state.launcher.discover_recent_projects() {
            Ok(count) => state.launcher.status = format!("{count} proyectos encontrados"),
            Err(error) => state.launcher.status = error.to_string(),
        }
    }

    draw_text("Abrir proyecto", left.x, left.y + 228.0, 20.0, ui_text());
    draw_launcher_text_field(
        &mut state.active_field,
        LauncherTextField::OpenPath,
        RectSpec {
            x: left.x,
            y: left.y + 250.0,
            w: left.w,
            h: 34.0,
        },
        "Ruta",
        &state.launcher.open_path,
    );
    if button(left.x, left.y + 298.0, 132.0, 30.0, "Abrir", false) {
        match state.launcher.open_typed_project() {
            Ok(path) => return Some(path),
            Err(error) => state.launcher.status = error.to_string(),
        }
    }
    if button(
        left.x + 142.0,
        left.y + 298.0,
        150.0,
        30.0,
        "Default",
        false,
    ) {
        match state
            .launcher
            .open_project(AssetTools::default_project_path())
        {
            Ok(path) => return Some(path),
            Err(error) => state.launcher.status = error.to_string(),
        }
    }
    if button(left.x + 302.0, left.y + 298.0, 86.0, 30.0, "Repair", false) {
        let path = state.launcher.typed_or_default_path();
        match state.launcher.repair_project(&path) {
            Ok(notes) => state.launcher.status = format!("Repair listo: {} notas", notes.len()),
            Err(error) => state.launcher.status = error.to_string(),
        }
    }
    if button(left.x, left.y + 336.0, 132.0, 28.0, "Export Debug", false) {
        let path = state.launcher.typed_or_default_path();
        match state.launcher.export_game(&path) {
            Ok(report) => {
                state.launcher.status = format!("Export listo: {}", report.output_path.display())
            }
            Err(error) => state.launcher.status = error.to_string(),
        }
    }

    draw_text("Recientes", left.x, left.y + 370.0, 18.0, ui_text());
    let mut y = left.y + 396.0;
    for path in state.launcher.recent_projects.clone().iter().take(5) {
        let row = RectSpec {
            x: left.x,
            y: y - 18.0,
            w: left.w,
            h: 28.0,
        };
        let hovered = contains_mouse(row);
        draw_rect(
            row,
            if hovered {
                Color::from_rgba(45, 60, 82, 255)
            } else {
                Color::from_rgba(28, 35, 49, 255)
            },
        );
        draw_text(
            &ellipsize(&path.display().to_string(), 58),
            row.x + 8.0,
            y,
            13.0,
            ui_text(),
        );
        if hovered && is_mouse_button_pressed(MouseButton::Left) {
            match state.launcher.open_project(path) {
                Ok(path) => return Some(path),
                Err(error) => state.launcher.status = error.to_string(),
            }
        }
        y += 32.0;
    }

    draw_text("Notas del parche", right.x, right.y, 22.0, ui_text());
    let notes = state.launcher.patch_notes.clone();
    let mut chip_x = right.x;
    let mut chip_y = right.y + 34.0;
    for (index, note) in notes.iter().enumerate() {
        let active = state.launcher.selected_patch_note == index;
        let chip_w = (note.version.len() as f32 * 8.0 + 72.0).clamp(86.0, 132.0);
        if chip_x + chip_w > right.x + right.w {
            chip_x = right.x;
            chip_y += 30.0;
        }
        if button(chip_x, chip_y - 18.0, chip_w, 25.0, &note.version, active) {
            state.launcher.selected_patch_note = index;
        }
        chip_x += chip_w + 8.0;
    }
    let notes_bottom = chip_y + 22.0;
    if let Some(note) = state.launcher.active_patch_note() {
        let card = RectSpec {
            x: right.x,
            y: notes_bottom + 12.0,
            w: right.w,
            h: (right.h * 0.36).clamp(160.0, 220.0),
        };
        draw_surface(card, false);
        draw_rectangle_lines(card.x, card.y, card.w, card.h, 1.0, ui_line_soft());
        draw_text(
            &ellipsize(&format!("{} - {}", note.date, note.title), 46),
            card.x + 14.0,
            card.y + 28.0,
            17.0,
            ui_text(),
        );
        let mut ny = card.y + 62.0;
        for highlight in note.highlights.iter().take(5) {
            draw_text(
                &ellipsize(highlight, 58),
                card.x + 18.0,
                ny,
                14.0,
                ui_text_muted(),
            );
            ny += 26.0;
        }
        draw_launcher_backend_panel(
            state,
            RectSpec {
                x: right.x,
                y: card.y + card.h + 12.0,
                w: right.w,
                h: (panel.y + panel.h - 64.0) - (card.y + card.h + 12.0),
            },
        );
    }

    if !state.launcher.status.is_empty() {
        draw_text(
            &ellipsize(&state.launcher.status, 96),
            right.x,
            panel.y + panel.h - 24.0,
            13.0,
            ui_warning(),
        );
    }
    if button(
        panel.x + panel.w - 86.0,
        panel.y + panel.h - 42.0,
        64.0,
        26.0,
        "Salir",
        false,
    ) {
        state.close_requested = true;
        return None;
    }
    None
}

fn draw_launcher_text_field(
    active_field: &mut Option<LauncherTextField>,
    field: LauncherTextField,
    rect: RectSpec,
    label: &str,
    value: &str,
) {
    let active = *active_field == Some(field);
    draw_rect(
        rect,
        if active {
            Color::from_rgba(26, 36, 54, 255)
        } else {
            Color::from_rgba(18, 24, 35, 255)
        },
    );
    draw_rectangle_lines(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        1.0,
        if active { ui_accent() } else { ui_line_soft() },
    );
    draw_text(label, rect.x + 10.0, rect.y + 22.0, 13.0, ui_text_muted());
    let text = if value.is_empty() { "..." } else { value };
    draw_text(
        &ellipsize(text, 60),
        rect.x + 94.0,
        rect.y + 22.0,
        14.0,
        ui_text(),
    );
    if contains_mouse(rect) && is_mouse_button_pressed(MouseButton::Left) {
        *active_field = Some(field);
    }
}

fn draw_launcher_backend_panel(state: &mut LauncherUiState, rect: RectSpec) {
    if rect.h < 120.0 {
        return;
    }
    draw_surface(rect, false);
    draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 1.0, ui_line_soft());
    draw_text("Backend", rect.x + 14.0, rect.y + 28.0, 18.0, ui_text());
    if button(
        rect.x + rect.w - 104.0,
        rect.y + 10.0,
        90.0,
        26.0,
        "Analizar",
        false,
    ) {
        match state.launcher.refresh_typed_project_status() {
            Ok(summary) => state.launcher.status = summary,
            Err(error) => state.launcher.status = error.to_string(),
        }
    }

    let mut y = rect.y + 58.0;
    if let Some(plan) = state.launcher.backend_plan.as_ref() {
        let score_color = if plan.system_audit.total_score >= 80 {
            ui_accent_2()
        } else if plan.system_audit.total_score >= 60 {
            ui_warning()
        } else {
            Color::from_rgba(255, 126, 126, 255)
        };
        draw_text(
            &format!("Readiness {}%", plan.system_audit.total_score),
            rect.x + 14.0,
            y,
            18.0,
            score_color,
        );
        draw_text(
            &format!(
                "Editor {}   Runtime {}   Export {}",
                readiness_text(plan.editor_ready),
                readiness_text(plan.runtime_ready),
                readiness_text(plan.export_ready)
            ),
            rect.x + 14.0,
            y + 24.0,
            13.0,
            ui_text_muted(),
        );
        y += 54.0;
        for action in state.launcher.backend_actions.iter().take(4) {
            draw_text(
                &ellipsize(action, 58),
                rect.x + 18.0,
                y,
                13.0,
                ui_text_muted(),
            );
            y += 22.0;
            if y > rect.y + rect.h - 44.0 {
                break;
            }
        }
    } else {
        draw_text(
            "Sin analisis todavia",
            rect.x + 14.0,
            y,
            14.0,
            ui_text_muted(),
        );
        draw_text(
            &ellipsize(
                &state.launcher.typed_or_default_path().display().to_string(),
                56,
            ),
            rect.x + 14.0,
            y + 24.0,
            13.0,
            ui_text_muted(),
        );
    }

    let toggle_y = rect.y + rect.h - 32.0;
    let validate_label = if state.launcher.settings.validate_on_open {
        "Validar ON"
    } else {
        "Validar OFF"
    };
    if button(
        rect.x + 14.0,
        toggle_y,
        104.0,
        24.0,
        validate_label,
        state.launcher.settings.validate_on_open,
    ) {
        state.launcher.settings.validate_on_open = !state.launcher.settings.validate_on_open;
        let _ = state.launcher.save_state();
    }
    let export_label = if state.launcher.settings.analyze_before_export {
        "Preflight ON"
    } else {
        "Preflight OFF"
    };
    if button(
        rect.x + 126.0,
        toggle_y,
        112.0,
        24.0,
        export_label,
        state.launcher.settings.analyze_before_export,
    ) {
        state.launcher.settings.analyze_before_export =
            !state.launcher.settings.analyze_before_export;
        let _ = state.launcher.save_state();
    }
    let recent_label = if state.launcher.settings.remember_recent {
        "Recientes ON"
    } else {
        "Recientes OFF"
    };
    if button(
        rect.x + 246.0,
        toggle_y,
        116.0,
        24.0,
        recent_label,
        state.launcher.settings.remember_recent,
    ) {
        state.launcher.settings.remember_recent = !state.launcher.settings.remember_recent;
        let _ = state.launcher.save_state();
    }
}

fn readiness_text(value: bool) -> &'static str {
    if value { "OK" } else { "WATCH" }
}

pub async fn run_editor_async() {
    let args = parse_args();
    if let Some(create_path) = args.create_project.clone() {
        if create_path.exists() && !args.force_create_project {
            eprintln!(
                "El proyecto ya existe: {}. Usa --force para recrear plantillas encima.",
                create_path.display()
            );
            return;
        }
        if let Err(error) = AssetTools::ensure_project_folders(&create_path).and_then(|_| {
            ProjectTemplates::create(
                &create_path,
                if args.create_template.is_empty() {
                    "empty"
                } else {
                    &args.create_template
                },
            )
            .map(|created| {
                println!(
                    "{} proyecto creado en {} con template {} ({} archivos base)",
                    crate::version_label(),
                    create_path.display(),
                    if args.create_template.is_empty() {
                        "empty"
                    } else {
                        &args.create_template
                    },
                    created.len()
                );
            })
        }) {
            eprintln!("No se pudo crear el proyecto MiniForge: {error}");
        }
        return;
    }
    let project_path = if args.force_launcher && !args.runtime && !args.headless_once {
        match run_startup_launcher().await {
            Some(path) => path,
            None => return,
        }
    } else if let Some(path) = args.project.clone() {
        path
    } else if args.no_launcher || args.runtime || args.headless_once {
        PathBuf::from("projects").join("DefaultProject")
    } else {
        match run_startup_launcher().await {
            Some(path) => path,
            None => return,
        }
    };

    if let Err(error) = AssetTools::ensure_project_folders(&project_path) {
        eprintln!("No se pudo preparar el proyecto: {error}");
        return;
    }
    CrashReporter::install(CrashReporterConfig::for_project(
        &project_path,
        if args.runtime {
            "MiniForge Editor Runtime"
        } else {
            "MiniForge Editor"
        },
    ));

    let safe_mode = if args.safe_mode {
        SafeModeSettings::for_recovery("solicitado desde --safe-mode")
    } else {
        SafeModeSettings::default()
    };
    let mut game = match Game::from_project_with_safe_mode(&project_path, args.runtime, safe_mode) {
        Ok(game) => game,
        Err(error) => {
            eprintln!("No se pudo iniciar MiniForge Rust: {error}");
            return;
        }
    };

    if args.headless_once {
        game.run_headless_once(1.0 / 60.0);
        println!(
            "{} headless listo en {}",
            crate::version_label(),
            game.project_path.display()
        );
        return;
    }

    game.refresh_assets().ok();
    game.console.log("Ventana Rust iniciada", "ENGINE");
    game.console
        .log("Ctrl+P abre comandos. 1-5 cambia herramienta.", "EDITOR");
    let mut state = EditorState {
        tile_brush: game.tile_brush.max(1),
        zoom_target: game.camera.zoom,
        editor_preferences: load_editor_preferences(&game),
        file_watcher: EditorFileWatcher::watch(&game.project_path).ok(),
        ..Default::default()
    };
    if game.safe_mode.enabled {
        state.script_window_open = false;
        state.sprite_window_open = false;
        state.blueprint_picker_open = false;
        state.show_hierarchy = true;
        state.show_inspector = true;
        state.show_console = true;
        game.console.warning(
            game.safe_mode
                .report()
                .warnings
                .first()
                .cloned()
                .unwrap_or_else(|| "Safe Mode activo".to_string()),
            "SAFE_MODE",
        );
    }
    configure_session_recovery(&mut game, &mut state);

    loop {
        let dt = get_frame_time() as f64;
        let sw = screen_width();
        let sh = screen_height();
        let launcher_overlay_open = state.launcher_overlay.is_some();
        let preferences_open = state.preferences_open;

        if !launcher_overlay_open && !preferences_open {
            handle_text_edit_input(&mut game, &mut state);
            if handle_shortcuts(&mut game, &mut state) {
                break;
            }
            poll_external_play_window(&mut game, &mut state);
            poll_editor_file_changes(&mut game, &mut state);

            handle_camera_input(&mut game, dt, &mut state);
            handle_character_input(&mut game);
        }

        let layout = layout(
            sw,
            sh,
            state.show_hierarchy,
            state.show_inspector,
            state.show_console,
        );
        let viewport = Viewport {
            rect: layout.scene,
            tile: game.grid.tile_size as f32,
            zoom: game.camera.zoom as f32,
            camera_x: game.camera.x as f32,
            camera_y: game.camera.y as f32,
        };

        if !launcher_overlay_open && !preferences_open {
            handle_scene_mouse(&mut game, &mut state, viewport);
        }

        if !state.paused && !launcher_overlay_open {
            game.run_headless_once(dt);
        } else {
            game.diagnostics.update(dt);
        }

        clear_background(ui_bg());
        draw_editor_backdrop(sw, sh);
        draw_scene(&game, &state, viewport);
        if state.show_hierarchy {
            draw_hierarchy(&mut game, &mut state, layout.left);
        }
        if state.show_inspector {
            draw_inspector(&mut game, &mut state, layout.right);
        }
        draw_top_bar(&mut game, &mut state, layout.top);
        draw_status_bar(&game, &state, layout.status);
        if state.show_console {
            draw_bottom_panel(&mut game, &mut state, layout.console);
        }
        draw_floating_play_window(&mut game, &mut state, sw, sh);
        draw_floating_script_window(&mut game, &mut state, sw, sh);
        draw_blueprint_picker_window(&mut game, &mut state, sw, sh);
        draw_floating_sprite_window(&mut game, &mut state, sw, sh);
        draw_python_tools_window(&mut game, &mut state, sw, sh);
        draw_top_menu_overlay(&mut game, &mut state, layout.top);
        if state.command_palette.open {
            draw_command_palette(&mut game, &mut state, sw, sh);
        }
        draw_preferences_window(&mut game, &mut state, sw, sh);
        draw_script_close_prompt(&mut game, &mut state, sw, sh);
        draw_launcher_overlay(&mut game, &mut state, sw, sh);

        checkpoint_editor_session(&mut game, &mut state, false);

        next_frame().await;
    }
    finish_editor_session(&mut game, &mut state);
}

fn session_ui_state(state: &EditorState) -> SessionUiState {
    SessionUiState {
        show_console: state.show_console,
        show_grid: state.show_grid,
        show_hierarchy: state.show_hierarchy,
        show_inspector: state.show_inspector,
        active_bottom_panel: state.bottom_tab.label().to_string(),
        script_window_open: state.script_window_open,
        sprite_window_open: state.sprite_window_open,
        blueprint_picker_open: state.blueprint_picker_open,
    }
}

fn apply_recovered_ui(state: &mut EditorState, snapshot: &EditorSessionSnapshot) {
    state.show_console = snapshot.ui.show_console;
    state.show_grid = snapshot.ui.show_grid;
    state.show_hierarchy = snapshot.ui.show_hierarchy;
    state.show_inspector = snapshot.ui.show_inspector;
    state.bottom_tab = BottomTab::from_label(&snapshot.ui.active_bottom_panel);
    state.script_window_open = snapshot.ui.script_window_open;
    state.sprite_window_open = snapshot.ui.sprite_window_open;
    state.blueprint_picker_open = snapshot.ui.blueprint_picker_open;
}

fn configure_session_recovery(game: &mut Game, state: &mut EditorState) {
    let recovery =
        SessionRecoveryManager::new(&game.project_path, std::time::Duration::from_secs(10));
    match recovery.load_pending() {
        Ok(Some(snapshot)) => {
            let report = recovery.restore_script_editor(&snapshot, &mut game.script_editor);
            if !game.safe_mode.enabled {
                apply_recovered_ui(state, &snapshot);
            }
            game.console.log(
                format!(
                    "Sesión recuperada: {} documentos, {} buffers sin guardar",
                    report.restored_documents, report.restored_dirty_buffers
                ),
                "RECOVERY",
            );
            if snapshot.scene_dirty {
                game.console.warning(
                    format!(
                        "La sesión anterior tenía cambios de escena pendientes: {}. Revisa el autosave antes de descartarlos.",
                        snapshot.scene_dirty_reason
                    ),
                    "RECOVERY",
                );
            }
            for path in report.missing_documents.iter().take(8) {
                game.console.warning(
                    format!(
                        "Documento de recuperación no encontrado: {}",
                        path.display()
                    ),
                    "RECOVERY",
                );
            }
        }
        Ok(None) => {}
        Err(error) => game.console.error(
            format!("No se pudo leer la sesión de recuperación: {error}"),
            "RECOVERY",
        ),
    }
    state.session_recovery = Some(recovery);
    checkpoint_editor_session(game, state, true);
}

fn checkpoint_editor_session(game: &mut Game, state: &mut EditorState, force: bool) -> bool {
    let ui = session_ui_state(state);
    let current_scene = game.scene_manager.current_scene.clone();
    let scene_dirty_reason = game.scene_dirty_reason.clone();
    let Some(recovery) = state.session_recovery.as_mut() else {
        return false;
    };
    if !force && !recovery.should_checkpoint() {
        return false;
    }
    match recovery.checkpoint(
        &current_scene,
        game.scene_dirty,
        &scene_dirty_reason,
        &mut game.script_editor,
        ui,
    ) {
        Ok(report) => {
            if !report.omitted_buffers.is_empty() {
                game.console.warning(
                    format!(
                        "{} buffers excedieron el límite de recovery",
                        report.omitted_buffers.len()
                    ),
                    "RECOVERY",
                );
            }
            true
        }
        Err(error) => {
            game.console.error(
                format!("No se pudo guardar la sesión de recuperación: {error}"),
                "RECOVERY",
            );
            false
        }
    }
}

fn finish_editor_session(game: &mut Game, state: &mut EditorState) {
    let has_unsaved_changes = game.scene_dirty || game.script_editor.has_dirty_documents();
    if has_unsaved_changes {
        if checkpoint_editor_session(game, state, true) {
            game.console.log(
                "Cambios pendientes conservados en recovery de sesión",
                "RECOVERY",
            );
        }
        return;
    }
    if let Some(recovery) = state.session_recovery.as_mut()
        && let Err(error) = recovery.clear()
    {
        game.console.error(
            format!("No se pudo limpiar la sesión cerrada: {error}"),
            "RECOVERY",
        );
    }
}

fn draw_launcher_overlay(game: &mut Game, state: &mut EditorState, sw: f32, sh: f32) {
    if state.launcher_overlay.is_none() {
        return;
    }

    draw_rectangle(0.0, 0.0, sw, sh, Color::from_rgba(0, 0, 0, 178));

    let mut selected_path = None;
    let mut close_requested = false;
    if let Some(launcher_state) = state.launcher_overlay.as_mut() {
        if is_key_pressed(KeyCode::Escape) {
            close_requested = true;
        } else {
            handle_launcher_text_input(launcher_state);
            selected_path = draw_launcher_ui(launcher_state);
            close_requested = launcher_state.close_requested;
        }
    }

    if close_requested {
        state.launcher_overlay = None;
        return;
    }

    let Some(path) = selected_path else {
        return;
    };

    CrashReporter::install(CrashReporterConfig::for_project(&path, "MiniForge Editor"));
    let safe_mode = game.safe_mode.clone();
    match AssetTools::ensure_project_folders(&path)
        .and_then(|_| Game::from_project_with_safe_mode(&path, false, safe_mode))
    {
        Ok(mut loaded) => {
            loaded.refresh_assets().ok();
            loaded.console.log(
                format!("Proyecto abierto desde Launcher: {}", path.display()),
                "ENGINE",
            );
            finish_editor_session(game, state);
            *game = loaded;
            state.launcher_overlay = None;
            state.editor_preferences = load_editor_preferences(game);
            state.show_console = true;
            state.bottom_tab = BottomTab::Console;
            state.active_text_field = None;
            state.command_palette.close();
            configure_session_recovery(game, state);
        }
        Err(error) => {
            if let Some(launcher_state) = state.launcher_overlay.as_mut() {
                launcher_state.launcher.status =
                    format!("No se pudo abrir {}: {error}", path.display());
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Layout {
    top: RectSpec,
    left: RectSpec,
    right: RectSpec,
    scene: RectSpec,
    console: RectSpec,
    status: RectSpec,
}

fn layout(sw: f32, sh: f32, show_left: bool, show_right: bool, show_console: bool) -> Layout {
    let top_h = 82.0;
    let status_h = 26.0;
    let console_h = if show_console {
        (sh * 0.30).clamp(190.0, 340.0)
    } else {
        0.0
    };
    let left_w = if show_left {
        (sw * 0.18).clamp(220.0, 320.0)
    } else {
        0.0
    };
    let right_w = if show_right {
        (sw * 0.22).clamp(300.0, 420.0)
    } else {
        0.0
    };
    Layout {
        top: RectSpec {
            x: 0.0,
            y: 0.0,
            w: sw,
            h: top_h,
        },
        left: RectSpec {
            x: 0.0,
            y: top_h,
            w: left_w,
            h: sh - top_h - status_h - console_h,
        },
        right: RectSpec {
            x: sw - right_w,
            y: top_h,
            w: right_w,
            h: sh - top_h - status_h - console_h,
        },
        scene: RectSpec {
            x: left_w,
            y: top_h,
            w: sw - left_w - right_w,
            h: sh - top_h - status_h - console_h,
        },
        console: RectSpec {
            x: 0.0,
            y: sh - status_h - console_h,
            w: sw,
            h: console_h,
        },
        status: RectSpec {
            x: 0.0,
            y: sh - status_h,
            w: sw,
            h: status_h,
        },
    }
}

fn handle_text_edit_input(game: &mut Game, state: &mut EditorState) {
    if state.command_palette.open {
        handle_command_palette_input(game, state);
        return;
    }
    if state.content_search_active {
        handle_content_search_input(state);
        return;
    }
    if state.graph_node_search_active {
        handle_text_buffer_input(
            &mut state.graph_node_search,
            &mut state.graph_node_search_active,
        );
        return;
    }
    if state.graph_template_search_active {
        handle_text_buffer_input(
            &mut state.graph_template_search,
            &mut state.graph_template_search_active,
        );
        return;
    }
    if state.script_search_active {
        handle_text_buffer_input(&mut state.script_search, &mut state.script_search_active);
        return;
    }
    if state.code_editor_active {
        handle_code_editor_input(game, state);
        return;
    }
    let Some(edit) = state.active_text_field.as_mut() else {
        return;
    };

    while let Some(character) = get_char_pressed() {
        if !character.is_control() {
            edit.buffer.push(character);
        }
    }
    if is_key_pressed(KeyCode::Backspace) {
        edit.buffer.pop();
    }
    if is_key_pressed(KeyCode::Escape) {
        state.active_text_field = None;
        return;
    }
    if is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter) {
        let finished = state.active_text_field.take();
        if let Some(edit) = finished {
            match game.edit_inspector_value(
                edit.entity_id,
                &edit.target,
                &edit.key,
                json!(edit.buffer),
            ) {
                Ok(_) => game.console.log(
                    format!("Inspector editado: {}.{}", edit.target, edit.key),
                    "EDITOR",
                ),
                Err(error) => game.console.log(format!("Inspector: {error}"), "WARNING"),
            }
        }
    }
}

fn handle_command_palette_input(game: &mut Game, state: &mut EditorState) {
    while let Some(character) = get_char_pressed() {
        state.command_palette.push(character);
    }
    if is_key_pressed(KeyCode::Backspace) {
        state.command_palette.backspace();
    }
    if is_key_pressed(KeyCode::Down) {
        state.command_palette.move_selection(1);
    }
    if is_key_pressed(KeyCode::Up) {
        state.command_palette.move_selection(-1);
    }
    if (is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter))
        && let Some((_, command)) = filtered_palette_commands(&state.command_palette.query)
            .get(state.command_palette.selected_index)
            .copied()
    {
        run_palette_command(game, state, command);
        state.command_palette.record_execution(command);
        state.command_palette.close();
    }
}

fn handle_text_buffer_input(buffer: &mut String, active: &mut bool) {
    while let Some(character) = get_char_pressed() {
        if !character.is_control() {
            buffer.push(character);
        }
    }
    if is_key_pressed(KeyCode::Backspace) {
        buffer.pop();
    }
    if is_key_pressed(KeyCode::Escape)
        || is_key_pressed(KeyCode::Enter)
        || is_key_pressed(KeyCode::KpEnter)
    {
        *active = false;
    }
}

fn handle_content_search_input(state: &mut EditorState) {
    while let Some(character) = get_char_pressed() {
        if !character.is_control() {
            state.content_search.push(character);
        }
    }
    if is_key_pressed(KeyCode::Backspace) {
        state.content_search.pop();
    }
    if is_key_pressed(KeyCode::Escape) {
        state.content_search_active = false;
    }
    if is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter) {
        state.content_search_active = false;
    }
}

fn handle_code_editor_input(game: &mut Game, state: &mut EditorState) {
    if game.script_editor.document.path.is_none() {
        match game.create_luau_script_asset("NewGameplayScript") {
            Ok(path) => {
                set_open_file_editor_state(state, &path);
                game.console.log(
                    "Script creado automaticamente para poder escribir.",
                    "SCRIPT",
                );
            }
            Err(error) => {
                state.code_editor_active = false;
                game.console.log(
                    format!("No se pudo crear script editable: {error}"),
                    "ERROR",
                );
                return;
            }
        }
    }
    if command_modifier_down() && is_key_pressed(KeyCode::S) {
        if let Err(error) = game.save_open_file() {
            game.console
                .log(format!("Error guardando archivo abierto: {error}"), "ERROR");
        }
        return;
    }
    if command_modifier_down() && is_key_pressed(KeyCode::C) {
        if let Some(line) = game.script_editor.lines.get(state.code_cursor_line)
            && let Err(error) = EditorClipboard::copy_text(line.clone())
        {
            game.console
                .log(format!("No se pudo copiar: {error}"), "WARNING");
        }
        return;
    }
    if command_modifier_down() && is_key_pressed(KeyCode::X) {
        if let Some(line) = game
            .script_editor
            .lines
            .get(state.code_cursor_line)
            .cloned()
        {
            match EditorClipboard::copy_text(line) {
                Ok(()) => {
                    let (line, column) = game.script_editor.delete_line(state.code_cursor_line);
                    state.code_cursor_line = line;
                    state.code_cursor_column = column;
                }
                Err(error) => game
                    .console
                    .log(format!("No se pudo cortar: {error}"), "WARNING"),
            }
        }
        return;
    }
    if command_modifier_down() && is_key_pressed(KeyCode::V) {
        match EditorClipboard::paste_text() {
            Ok(text) => {
                let (line, column) = game.script_editor.insert_text(
                    state.code_cursor_line,
                    state.code_cursor_column,
                    &text.replace("\r\n", "\n"),
                );
                state.code_cursor_line = line;
                state.code_cursor_column = column;
                keep_code_cursor_visible(state, 12);
            }
            Err(error) => game
                .console
                .log(format!("No se pudo pegar: {error}"), "WARNING"),
        }
        return;
    }
    if command_modifier_down() && is_key_pressed(KeyCode::D) {
        let (line, column) = game.script_editor.duplicate_line(state.code_cursor_line);
        state.code_cursor_line = line;
        state.code_cursor_column = column;
        keep_code_cursor_visible(state, 12);
        return;
    }
    if command_modifier_down() && is_key_pressed(KeyCode::Backspace) {
        let (line, column) = game.script_editor.delete_line(state.code_cursor_line);
        state.code_cursor_line = line;
        state.code_cursor_column = column;
        keep_code_cursor_visible(state, 12);
        return;
    }
    if command_modifier_down() && (is_key_pressed(KeyCode::Slash) || is_key_pressed(KeyCode::Key7))
    {
        let (line, column) = game
            .script_editor
            .toggle_line_comment(state.code_cursor_line);
        state.code_cursor_line = line;
        state.code_cursor_column = column;
        return;
    }
    if is_key_pressed(KeyCode::Tab) {
        let (line, column) = if is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift)
        {
            game.script_editor.outdent_line(state.code_cursor_line)
        } else {
            game.script_editor.indent_line(state.code_cursor_line)
        };
        state.code_cursor_line = line;
        state.code_cursor_column = column;
        return;
    }
    if is_key_pressed(KeyCode::Escape) {
        state.code_editor_active = false;
        return;
    }
    while let Some(character) = get_char_pressed() {
        if !character.is_control() {
            let (line, column) = game.script_editor.insert_char(
                state.code_cursor_line,
                state.code_cursor_column,
                character,
            );
            state.code_cursor_line = line;
            state.code_cursor_column = column;
        }
    }
    if is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter) {
        let (line, column) = game
            .script_editor
            .split_line(state.code_cursor_line, state.code_cursor_column);
        state.code_cursor_line = line;
        state.code_cursor_column = column;
    }
    if is_key_pressed(KeyCode::Backspace) {
        let (line, column) = game
            .script_editor
            .backspace(state.code_cursor_line, state.code_cursor_column);
        state.code_cursor_line = line;
        state.code_cursor_column = column;
    }
    if is_key_pressed(KeyCode::Delete) {
        let (line, column) = game
            .script_editor
            .delete_forward(state.code_cursor_line, state.code_cursor_column);
        state.code_cursor_line = line;
        state.code_cursor_column = column;
    }
    if is_key_pressed(KeyCode::Up) {
        state.code_cursor_line = state.code_cursor_line.saturating_sub(1);
        clamp_code_cursor(game, state);
    }
    if is_key_pressed(KeyCode::Down) {
        state.code_cursor_line =
            (state.code_cursor_line + 1).min(game.script_editor.lines.len().saturating_sub(1));
        clamp_code_cursor(game, state);
    }
    if is_key_pressed(KeyCode::Left) {
        if state.code_cursor_column > 0 {
            if let Some(line) = game.script_editor.lines.get(state.code_cursor_line) {
                state.code_cursor_column =
                    previous_code_char_boundary(line, state.code_cursor_column);
            } else {
                state.code_cursor_column = state.code_cursor_column.saturating_sub(1);
            }
        } else if state.code_cursor_line > 0 {
            state.code_cursor_line -= 1;
            state.code_cursor_column = game
                .script_editor
                .lines
                .get(state.code_cursor_line)
                .map(String::len)
                .unwrap_or(0);
        }
    }
    if is_key_pressed(KeyCode::Right) {
        let line_len = game
            .script_editor
            .lines
            .get(state.code_cursor_line)
            .map(String::len)
            .unwrap_or(0);
        if state.code_cursor_column < line_len {
            if let Some(line) = game.script_editor.lines.get(state.code_cursor_line) {
                state.code_cursor_column = next_code_char_boundary(line, state.code_cursor_column);
            } else {
                state.code_cursor_column += 1;
            }
        } else if state.code_cursor_line + 1 < game.script_editor.lines.len() {
            state.code_cursor_line += 1;
            state.code_cursor_column = 0;
        }
    }
    if is_key_pressed(KeyCode::Home) {
        state.code_cursor_column = 0;
    }
    if is_key_pressed(KeyCode::End) {
        state.code_cursor_column = game
            .script_editor
            .lines
            .get(state.code_cursor_line)
            .map(String::len)
            .unwrap_or(0);
    }
    if is_key_pressed(KeyCode::PageUp) {
        state.code_cursor_line = state.code_cursor_line.saturating_sub(12);
        state.code_scroll_line = state.code_scroll_line.saturating_sub(12);
        clamp_code_cursor(game, state);
    }
    if is_key_pressed(KeyCode::PageDown) {
        state.code_cursor_line =
            (state.code_cursor_line + 12).min(game.script_editor.lines.len().saturating_sub(1));
        state.code_scroll_line = state.code_scroll_line.saturating_add(12);
        clamp_code_cursor(game, state);
    }
    keep_code_cursor_visible(state, 12);
}

fn clamp_code_cursor(game: &Game, state: &mut EditorState) {
    state.code_cursor_line = state
        .code_cursor_line
        .min(game.script_editor.lines.len().saturating_sub(1));
    let line_len = game
        .script_editor
        .lines
        .get(state.code_cursor_line)
        .map(String::len)
        .unwrap_or(0);
    state.code_cursor_column = state.code_cursor_column.min(line_len);
}

fn keep_code_cursor_visible(state: &mut EditorState, margin: usize) {
    let visible_lines = margin.max(1);
    if state.code_cursor_line < state.code_scroll_line {
        state.code_scroll_line = state.code_cursor_line;
    } else if state.code_cursor_line >= state.code_scroll_line + visible_lines {
        state.code_scroll_line = state
            .code_cursor_line
            .saturating_sub(visible_lines.saturating_sub(1));
    }
}

fn handle_shortcuts(game: &mut Game, state: &mut EditorState) -> bool {
    let command = command_modifier_down();
    let shift = is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift);

    if state.active_text_field.is_some()
        || state.code_editor_active
        || state.content_search_active
        || state.graph_node_search_active
        || state.graph_template_search_active
        || state.script_search_active
    {
        return false;
    }

    if state.menu_bar.open_menu.is_some() {
        if is_key_pressed(KeyCode::Escape) {
            state.menu_bar.close();
            return false;
        }
        if is_key_pressed(KeyCode::Down) {
            state.menu_bar.move_focus(1);
            return false;
        }
        if is_key_pressed(KeyCode::Up) {
            state.menu_bar.move_focus(-1);
            return false;
        }
        if is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter) {
            if let Some(command) = state.menu_bar.activate_focused() {
                run_palette_command(game, state, &command);
            }
            return false;
        }
    }

    if is_key_pressed(KeyCode::Escape) {
        if state.command_palette.open {
            state.command_palette.close();
            return false;
        }
        if state.script_window_open {
            request_close_script_tab(game, state, game.script_editor.document.path.clone());
            return false;
        }
        if state.play_window_open {
            state.play_window_open = false;
            game.console
                .log("Ventana Play cerrada; el editor sigue activo", "EDITOR");
            return false;
        }
        return true;
    }
    if command && is_key_pressed(KeyCode::P) {
        state.command_palette.toggle();
        return false;
    }
    if state.command_palette.open {
        return false;
    }

    if is_key_pressed(KeyCode::F1) {
        state.show_console = !state.show_console;
    }
    if is_key_pressed(KeyCode::F2) {
        state.show_hierarchy = !state.show_hierarchy;
    }
    if is_key_pressed(KeyCode::F3) {
        state.show_inspector = !state.show_inspector;
    }
    if is_key_pressed(KeyCode::F4) {
        state.show_grid = !state.show_grid;
    }
    if is_key_pressed(KeyCode::F5) {
        toggle_play_mode(game, state);
    }
    if is_key_pressed(KeyCode::F6) {
        game.cycle_workspace_mode();
    }
    if is_key_pressed(KeyCode::F11) {
        state.paused = !state.paused;
        game.console.log(
            if state.paused {
                "Play mode pausado"
            } else {
                "Play mode reanudado"
            },
            "ENGINE",
        );
    }
    if is_key_pressed(KeyCode::Key1) {
        set_tool(game, state, EditorTool::Select);
    }
    if is_key_pressed(KeyCode::Key2) {
        set_tool(game, state, EditorTool::Move);
    }
    if is_key_pressed(KeyCode::Key3) {
        set_tool(game, state, EditorTool::Rotate);
    }
    if is_key_pressed(KeyCode::Key4) {
        set_tool(game, state, EditorTool::Scale);
    }
    if is_key_pressed(KeyCode::Key5) {
        set_tool(game, state, EditorTool::Paint);
    }
    if is_key_pressed(KeyCode::Key6) {
        set_tool(game, state, EditorTool::Pivot);
    }
    if is_key_pressed(KeyCode::Key7) {
        set_tool(game, state, EditorTool::Collision);
    }
    if is_key_pressed(KeyCode::Tab) {
        state.show_console = true;
        state.bottom_tab = state.bottom_tab.next();
    }
    if is_key_pressed(KeyCode::L) && state.toolbar.active_tool() == EditorTool::Paint {
        game.cycle_tilemap_layer();
    }
    if is_key_pressed(KeyCode::B) && state.toolbar.active_tool() == EditorTool::Paint && !command {
        state.tile_brush_mode = state.tile_brush_mode.next();
        game.console.log(
            format!("Brush activo: {}", state.tile_brush_mode.label()),
            "TILEMAP",
        );
    }
    if is_key_pressed(KeyCode::G) && !command {
        state.snap_to_grid = !state.snap_to_grid;
        game.console.log(
            format!(
                "Snap to grid {}",
                if state.snap_to_grid { "ON" } else { "OFF" }
            ),
            "EDITOR",
        );
    }
    if is_key_pressed(KeyCode::F)
        && let Some((entity_x, entity_y)) = game
            .selected_units
            .first()
            .and_then(|id| game.get_entity_by_id(*id))
            .map(|entity| (entity.x, entity.y))
    {
        game.camera.x = entity_x * game.grid.tile_size as f64 - 280.0;
        game.camera.y = entity_y * game.grid.tile_size as f64 - 180.0;
        game.camera.clamp_to_bounds();
    }

    if command {
        if is_key_pressed(KeyCode::S) {
            save_scene(game);
        }
        if is_key_pressed(KeyCode::N) {
            create_new_scene(game);
        }
        if is_key_pressed(KeyCode::D) {
            duplicate_selected(game);
        }
        if is_key_pressed(KeyCode::B) {
            build_manifest(game);
        }
        if is_key_pressed(KeyCode::E) {
            export_runtime(game, ExportProfile::Debug);
        }
        if is_key_pressed(KeyCode::R) {
            refresh_assets(game);
        }
        if !shift
            && is_key_pressed(KeyCode::G)
            && let Ok(path) = game.create_program_asset("LogAndMove")
        {
            set_open_file_editor_state(state, &path);
            state.show_console = true;
            state.bottom_tab = BottomTab::Programming;
        }
        if is_key_pressed(KeyCode::I) {
            let (x, y) = spawn_position(game);
            if game.instantiate_first_prefab(x, y).ok().flatten().is_some() {
                state.show_console = true;
                state.bottom_tab = BottomTab::Prefabs;
            }
        }
        if shift && is_key_pressed(KeyCode::G) {
            group_selected_entities(game);
        }
        if shift && is_key_pressed(KeyCode::U) {
            ungroup_selected_entities(game);
        }
        if is_key_pressed(KeyCode::Z) {
            undo(game);
        }
        if is_key_pressed(KeyCode::Y)
            || (is_key_down(KeyCode::LeftShift) && is_key_pressed(KeyCode::Z))
            || (is_key_down(KeyCode::RightShift) && is_key_pressed(KeyCode::Z))
        {
            redo(game);
        }
    }

    if is_key_pressed(KeyCode::Delete) || is_key_pressed(KeyCode::Backspace) {
        delete_selected(game);
    }

    false
}

fn handle_camera_input(game: &mut Game, dt: f64, state: &mut EditorState) {
    if state.command_palette.open {
        return;
    }
    let speed = 520.0 * dt / game.camera.zoom.max(0.1);
    let mut dx = 0.0;
    let mut dy = 0.0;
    if is_key_down(KeyCode::A) || is_key_down(KeyCode::Left) {
        dx -= speed;
    }
    if (is_key_down(KeyCode::D) && !command_modifier_down()) || is_key_down(KeyCode::Right) {
        dx += speed;
    }
    if is_key_down(KeyCode::W) || is_key_down(KeyCode::Up) {
        dy -= speed;
    }
    if (is_key_down(KeyCode::S) && !command_modifier_down()) || is_key_down(KeyCode::Down) {
        dy += speed;
    }
    if dx != 0.0 || dy != 0.0 {
        game.camera.move_by(dx, dy);
    }
    if is_key_down(KeyCode::Q) {
        state.zoom_target -= 1.4 * dt;
    }
    if is_key_down(KeyCode::E) {
        state.zoom_target += 1.4 * dt;
    }
    state.zoom_target = state.zoom_target.clamp(0.1, 8.0);
    let blend = 1.0 - (-14.0 * dt).exp();
    game.camera
        .set_zoom(game.camera.zoom + (state.zoom_target - game.camera.zoom) * blend);
}

fn handle_character_input(game: &mut Game) {
    if game.mode != "PLAY" {
        return;
    }
    let left = is_key_down(KeyCode::A) || is_key_down(KeyCode::Left);
    let right = is_key_down(KeyCode::D) || is_key_down(KeyCode::Right);
    let up = is_key_down(KeyCode::W) || is_key_down(KeyCode::Up);
    let down = is_key_down(KeyCode::S) || is_key_down(KeyCode::Down);
    let input_x = right as i32 as f64 - left as i32 as f64;
    let input_y = down as i32 as f64 - up as i32 as f64;
    let jump_pressed = is_key_pressed(KeyCode::Space) || is_key_pressed(KeyCode::Up);
    let jump_held = is_key_down(KeyCode::Space) || is_key_down(KeyCode::Up);
    let run_pressed = is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift);
    let dash_pressed = is_key_pressed(KeyCode::X)
        || is_key_pressed(KeyCode::LeftControl)
        || is_key_pressed(KeyCode::RightControl);

    game.set_character_input_for_tag(
        "Player",
        (input_x, input_y),
        jump_pressed,
        jump_held,
        run_pressed,
        dash_pressed,
    );
    for (key, pressed) in [
        ("A", left),
        ("D", right),
        ("W", up),
        ("S", down),
        ("Space", jump_held),
        ("Shift", run_pressed),
        ("X", dash_pressed),
        ("E", is_key_down(KeyCode::E) || is_key_down(KeyCode::Enter)),
        ("1", is_key_down(KeyCode::Key1)),
        ("2", is_key_down(KeyCode::Key2)),
    ] {
        game.set_script_input_pressed(key, pressed);
    }
    for (key_code, key) in [
        (KeyCode::Space, "Space"),
        (KeyCode::E, "E"),
        (KeyCode::Enter, "Enter"),
        (KeyCode::Key1, "1"),
        (KeyCode::Key2, "2"),
        (KeyCode::X, "X"),
    ] {
        if is_key_pressed(key_code) {
            game.dispatch_script_key_down(key);
        }
    }
    if is_key_pressed(KeyCode::E) || is_key_pressed(KeyCode::Enter) {
        let _ = game.interact();
    }
    if is_key_pressed(KeyCode::Key1) {
        let _ = game.choose_dialogue(0);
    }
    if is_key_pressed(KeyCode::Key2) {
        let _ = game.choose_dialogue(1);
    }
}

fn handle_scene_mouse(game: &mut Game, state: &mut EditorState, viewport: Viewport) {
    let (mx, my) = mouse_position();
    if pointer_over_floating_windows(state, mx, my) {
        if is_mouse_button_released(MouseButton::Left) {
            finish_ui_canvas_drag(game, state);
            finish_gizmo_drag(game, state);
            finish_collision_vertex_drag(game, state);
            state.drag_payload = None;
        }
        return;
    }
    if let Some(payload) = state.drag_payload.clone() {
        if is_mouse_button_released(MouseButton::Left) {
            if contains(viewport.rect, mx, my) {
                let world = screen_to_world(viewport, mx, my);
                let target =
                    find_ui_entity_at(game, mx, my).or_else(|| find_entity_at(game, world));
                match game.drop_asset_to_target(&payload, world.0 as f64, world.1 as f64, target) {
                    Ok(outcome) => log_drop_outcome(game, &payload.name, outcome),
                    Err(error) => game
                        .console
                        .log(format!("Drop asset falló: {error}"), "ERROR"),
                }
            }
            state.drag_payload = None;
        }
        return;
    }

    if !contains(viewport.rect, mx, my) || state.command_palette.open {
        if is_mouse_button_released(MouseButton::Left) {
            finish_ui_canvas_drag(game, state);
            finish_gizmo_drag(game, state);
            finish_collision_vertex_drag(game, state);
        }
        if !is_mouse_button_down(MouseButton::Right) && !is_mouse_button_down(MouseButton::Middle) {
            state.scene_panning = false;
        }
        return;
    }

    if update_scene_view_pan(game, state, viewport) {
        return;
    }

    let world = screen_to_world(viewport, mx, my);
    if state.toolbar.active_tool() != EditorTool::Paint
        && handle_ui_canvas_scene_mouse(game, state, viewport)
    {
        return;
    }
    match state.toolbar.active_tool() {
        EditorTool::Paint => {
            if is_mouse_button_pressed(MouseButton::Left) {
                state.paint_start = world_to_cell(game, world);
                state.last_painted_cell = None;
            }
            if is_mouse_button_down(MouseButton::Left) {
                paint_at(game, state, world);
            }
            if is_mouse_button_released(MouseButton::Left) {
                if matches!(
                    state.tile_brush_mode,
                    TileBrushMode::Rectangle | TileBrushMode::Line
                ) && let (Some(start), Some(end)) =
                    (state.paint_start, world_to_cell(game, world))
                    && game.paint_tile_brush(state.tile_brush_mode, start, end, state.tile_brush)
                {
                    game.console.log(
                        format!("{} brush aplicado", state.tile_brush_mode.label()),
                        "TILEMAP",
                    );
                }
                state.paint_start = None;
                state.last_painted_cell = None;
            }
        }
        EditorTool::Move | EditorTool::Rotate | EditorTool::Scale | EditorTool::Pivot => {
            if is_mouse_button_pressed(MouseButton::Left) {
                state.drag_entity = if state.toolbar.active_tool() == EditorTool::Move {
                    find_ui_entity_at(game, mx, my).or_else(|| find_entity_at(game, world))
                } else {
                    find_entity_at(game, world)
                };
                if let Some(id) = state.drag_entity {
                    clear_ui_canvas_selection(state);
                    if !game.selected_units.contains(&id) {
                        let additive =
                            is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift);
                        if additive {
                            game.select_entity_additive(id);
                        } else {
                            game.select_entity(id);
                        }
                    }
                    state.drag_ui_offset = ui_drag_offset(game, id, mx, my);
                    state.drag_entity_before = Some(game.capture_editor_snapshot());
                    state.drag_world_last = world;
                    game.console.log(format!("Moviendo entity #{id}"), "EDITOR");
                }
            }
            if is_mouse_button_down(MouseButton::Left) {
                apply_gizmo_drag(game, state, world);
            } else {
                finish_gizmo_drag(game, state);
            }
        }
        EditorTool::Collision => {
            if is_mouse_button_pressed(MouseButton::Left) {
                begin_collision_vertex_drag(game, state, world);
            }
            if is_mouse_button_down(MouseButton::Left) {
                apply_collision_vertex_drag(game, state, world);
            }
            if is_mouse_button_released(MouseButton::Left) {
                finish_collision_vertex_drag(game, state);
            }
        }
        EditorTool::Select => {
            if is_mouse_button_pressed(MouseButton::Left) {
                select_at(game, state, world, (mx, my));
            }
            if is_mouse_button_pressed(MouseButton::Right) {
                command_selected_move(game, world);
            }
        }
    }
}

fn update_scene_view_pan(game: &mut Game, state: &mut EditorState, viewport: Viewport) -> bool {
    let mouse = mouse_position();
    let pan_button_down = is_mouse_button_down(MouseButton::Middle)
        || is_mouse_button_down(MouseButton::Right)
        || (is_key_down(KeyCode::Space) && is_mouse_button_down(MouseButton::Left));
    let pan_button_pressed = is_mouse_button_pressed(MouseButton::Middle)
        || is_mouse_button_pressed(MouseButton::Right)
        || (is_key_down(KeyCode::Space) && is_mouse_button_pressed(MouseButton::Left));
    let pan_button_released = is_mouse_button_released(MouseButton::Middle)
        || is_mouse_button_released(MouseButton::Right)
        || is_mouse_button_released(MouseButton::Left);

    if pan_button_down && (contains(viewport.rect, mouse.0, mouse.1) || state.scene_panning) {
        if pan_button_pressed || !state.scene_panning {
            state.scene_pan_last = mouse;
            state.scene_pan_moved = false;
            state.scene_panning = true;
            return true;
        }
        let dx = mouse.0 - state.scene_pan_last.0;
        let dy = mouse.1 - state.scene_pan_last.1;
        if dx.abs() > 0.01 || dy.abs() > 0.01 {
            let zoom = game.camera.zoom.max(0.1);
            game.camera
                .move_by(-(dx as f64) / zoom, -(dy as f64) / zoom);
            state.scene_pan_moved = true;
        }
        state.scene_pan_last = mouse;
        return true;
    }

    if state.scene_panning {
        let consumed = state.scene_pan_moved || pan_button_released;
        state.scene_panning = false;
        state.scene_pan_moved = false;
        if consumed {
            return true;
        }
    }

    let (wheel_x, wheel_y) = mouse_wheel();
    if wheel_x.abs() > f32::EPSILON || wheel_y.abs() > f32::EPSILON {
        if command_modifier_down() {
            state.zoom_target =
                (state.zoom_target * (1.0 + (wheel_y as f64) * 0.12)).clamp(0.1, 8.0);
        } else {
            let zoom = game.camera.zoom.max(0.1);
            let shift_pan = is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift);
            let horizontal = if shift_pan { wheel_y } else { wheel_x };
            let vertical = if shift_pan { 0.0 } else { wheel_y };
            game.camera.move_by(
                -(horizontal as f64) * 64.0 / zoom,
                -(vertical as f64) * 64.0 / zoom,
            );
        }
        return true;
    }

    false
}

fn paint_at(game: &mut Game, state: &mut EditorState, world: (f32, f32)) {
    let Some(cell) = world_to_cell(game, world) else {
        return;
    };
    if state.tile_brush_mode == TileBrushMode::Fill {
        if is_mouse_button_pressed(MouseButton::Left)
            && game.paint_tile_brush(TileBrushMode::Fill, cell, cell, state.tile_brush)
        {
            game.console.log("Fill brush aplicado", "TILEMAP");
        }
        return;
    }
    if matches!(
        state.tile_brush_mode,
        TileBrushMode::Rectangle | TileBrushMode::Line
    ) {
        return;
    }
    if state.last_painted_cell == Some(cell) && !is_mouse_button_pressed(MouseButton::Left) {
        return;
    }
    state.last_painted_cell = Some(cell);

    let origin_x = world.0.floor() as isize;
    let origin_y = world.1.floor() as isize;
    let brush = game.brush_size.max(1) as isize;
    let half = brush / 2;
    let mut painted = 0;
    for y in origin_y - half..=origin_y + half {
        for x in origin_x - half..=origin_x + half {
            if x < 0 || y < 0 {
                continue;
            }
            if game.paint_tile_brush(
                state.tile_brush_mode,
                (x as usize, y as usize),
                (x as usize, y as usize),
                state.tile_brush,
            ) {
                painted += 1;
            }
        }
    }
    if painted > 0 && is_mouse_button_pressed(MouseButton::Left) {
        let layer = &game.tilemap_layers.layers[game.tilemap_layers.active_layer].name;
        game.console
            .log(format!("Pintados {painted} tiles en {layer}"), "TILEMAP");
    }
}

fn world_to_cell(game: &Game, world: (f32, f32)) -> Option<(usize, usize)> {
    if world.0 < 0.0 || world.1 < 0.0 {
        return None;
    }
    let x = world.0.floor() as usize;
    let y = world.1.floor() as usize;
    (x < game.tilemap_layers.width && y < game.tilemap_layers.height).then_some((x, y))
}

fn handle_ui_canvas_scene_mouse(
    game: &mut Game,
    state: &mut EditorState,
    viewport: Viewport,
) -> bool {
    let (mx, my) = mouse_position();
    let local = (mx - viewport.rect.x, my - viewport.rect.y);

    if is_mouse_button_released(MouseButton::Left)
        && (state.drag_ui_canvas_element || state.drag_ui_canvas_handle.is_some())
    {
        finish_ui_canvas_drag(game, state);
        return true;
    }

    if is_mouse_button_down(MouseButton::Left)
        && (state.drag_ui_canvas_element || state.drag_ui_canvas_handle.is_some())
    {
        apply_ui_canvas_drag(game, state, viewport, (mx, my));
        return true;
    }

    if !is_mouse_button_pressed(MouseButton::Left) {
        return false;
    }

    let roots = ui_canvases_from_value(&game.ui_canvases);
    if roots.is_empty() {
        return false;
    }

    if let Some((canvas_id, element_id, handle)) = hit_ui_canvas_handle(&roots, viewport, local) {
        game.clear_selection();
        state.selected_ui_canvas_id = Some(canvas_id);
        state.selected_ui_element_id = Some(element_id);
        state.drag_ui_canvas_handle = Some(handle);
        state.drag_ui_canvas_element = false;
        state.drag_ui_canvas_before = Some(game.capture_editor_snapshot());
        state.drag_ui_canvas_last = (mx, my);
        return true;
    }

    if let Some((canvas_id, element_id)) = hit_ui_canvas_element(&roots, viewport, local) {
        game.clear_selection();
        state.selected_ui_canvas_id = Some(canvas_id);
        state.selected_ui_element_id = Some(element_id.clone());
        state.drag_ui_canvas_handle = None;
        state.drag_ui_canvas_element = state.toolbar.active_tool() == EditorTool::Move;
        state.drag_ui_canvas_before = state
            .drag_ui_canvas_element
            .then(|| game.capture_editor_snapshot());
        state.drag_ui_canvas_last = (mx, my);
        game.console
            .log(format!("UI Canvas seleccionado: {element_id}"), "UI");
        return true;
    }

    false
}

fn apply_ui_canvas_drag(
    game: &mut Game,
    state: &mut EditorState,
    viewport: Viewport,
    mouse: (f32, f32),
) {
    let Some(canvas_id) = state.selected_ui_canvas_id.clone() else {
        return;
    };
    let Some(element_id) = state.selected_ui_element_id.clone() else {
        return;
    };
    let screen_dx = mouse.0 - state.drag_ui_canvas_last.0;
    let screen_dy = mouse.1 - state.drag_ui_canvas_last.1;
    if screen_dx.abs() < f32::EPSILON && screen_dy.abs() < f32::EPSILON {
        return;
    }

    let mut roots = ui_canvases_from_value(&game.ui_canvases);
    let Some(root) = roots.iter_mut().find(|root| root.id == canvas_id) else {
        return;
    };
    let tools = SceneViewTools {
        grid_snapping: state.snap_to_grid,
        snap_size: 8.0,
        tile_size: game.grid.tile_size as f64,
        camera_zoom: game.camera.zoom,
    };
    let report = if let Some(handle) = state.drag_ui_canvas_handle {
        tools.resize_ui_element_from_handle(
            root,
            &element_id,
            handle,
            viewport.rect.w,
            viewport.rect.h,
            screen_dx,
            screen_dy,
        )
    } else if state.drag_ui_canvas_element {
        tools.drag_ui_element(
            root,
            &element_id,
            viewport.rect.w,
            viewport.rect.h,
            screen_dx,
            screen_dy,
        )
    } else {
        return;
    };

    if report.changed {
        write_ui_canvas_roots(game, roots, "Edit UI Canvas");
        state.drag_ui_canvas_last = mouse;
    }
}

fn finish_ui_canvas_drag(game: &mut Game, state: &mut EditorState) {
    let before = state.drag_ui_canvas_before.take();
    let was_dragging = state.drag_ui_canvas_element || state.drag_ui_canvas_handle.is_some();
    state.drag_ui_canvas_element = false;
    state.drag_ui_canvas_handle = None;
    if !was_dragging {
        return;
    }
    if let Some(before) = before
        && before.ui_canvases != game.ui_canvases
    {
        game.push_editor_command(
            "Edit UI Canvas",
            EditorCommandKind::SceneOperation {
                name: "UI Canvas Gizmo".to_string(),
            },
            before,
        );
        game.console.log("UI Canvas actualizado", "UI");
    }
}

fn hit_ui_canvas_handle(
    roots: &[UiCanvasRoot],
    viewport: Viewport,
    local: (f32, f32),
) -> Option<(String, String, UiCanvasGizmoHandleKind)> {
    roots.iter().rev().find_map(|root| {
        root.hit_test_gizmo_handle(viewport.rect.w, viewport.rect.h, local)
            .map(|handle| (root.id.clone(), handle.element_id, handle.kind))
    })
}

fn hit_ui_canvas_element(
    roots: &[UiCanvasRoot],
    viewport: Viewport,
    local: (f32, f32),
) -> Option<(String, String)> {
    roots.iter().rev().find_map(|root| {
        root.hit_test_element(viewport.rect.w, viewport.rect.h, local)
            .map(|element| (root.id.clone(), element.id().to_string()))
    })
}

fn write_ui_canvas_roots(game: &mut Game, roots: Vec<UiCanvasRoot>, reason: &str) {
    game.ui_canvases = serde_json::to_value(&roots).unwrap_or_else(|_| json!([]));
    game.mark_scene_dirty(reason);
}

fn clear_ui_canvas_selection(state: &mut EditorState) {
    state.selected_ui_canvas_id = None;
    state.selected_ui_element_id = None;
    state.drag_ui_canvas_element = false;
    state.drag_ui_canvas_handle = None;
    state.drag_ui_canvas_before = None;
}

fn apply_gizmo_drag(game: &mut Game, state: &mut EditorState, world: (f32, f32)) {
    let Some(id) = state.drag_entity else {
        return;
    };
    let snap = state.snap_to_grid;
    let camera_zoom = game.camera.zoom;
    let (mx, my) = mouse_position();
    let Some(primary) = game.get_entity_by_id(id).cloned() else {
        return;
    };
    match state.toolbar.active_tool() {
        EditorTool::Move => {
            if primary.get_component("UIElement").is_some() {
                if let Some(entity) = game.get_entity_by_id_mut(id)
                    && let Some(ui) = entity.get_component_mut("UIElement")
                {
                    let mut next_x = mx - state.drag_ui_offset.0;
                    let mut next_y = my - state.drag_ui_offset.1;
                    if snap {
                        next_x = (next_x / 4.0).round() * 4.0;
                        next_y = (next_y / 4.0).round() * 4.0;
                    }
                    ui.set_f64("x", next_x as f64);
                    ui.set_f64("y", next_y as f64);
                }
            } else {
                let settings = SmartSnapSettings2D {
                    enabled: state.smart_snap,
                    grid_enabled: snap,
                    grid_step: 0.25,
                    ..SmartSnapSettings2D::default()
                };
                let result = EditorSpatialTools2D::smart_snap(
                    &primary,
                    (world.0 as f64, world.1 as f64),
                    &game.runtime_world.units,
                    camera_zoom,
                    &settings,
                );
                state.snap_guides = result.guides;
                let delta = (result.point.0 - primary.x, result.point.1 - primary.y);
                let selected = game.selected_units.clone();
                for selected_id in selected {
                    if let Some(entity) = game.get_entity_by_id_mut(selected_id)
                        && !entity.locked
                    {
                        entity.x += delta.0;
                        entity.y += delta.1;
                        entity.path.clear();
                        entity.sync_to_components();
                    }
                }
            }
        }
        EditorTool::Rotate => {
            if let Some(entity) = game.get_entity_by_id_mut(id) {
                let dx = world.0 as f64 - entity.x;
                let dy = world.1 as f64 - entity.y;
                entity.rotation = dy.atan2(dx).to_degrees();
                entity.sync_to_components();
            }
        }
        EditorTool::Scale => {
            if let Some(entity) = game.get_entity_by_id_mut(id) {
                let dx = world.0 as f64 - entity.x;
                let dy = world.1 as f64 - entity.y;
                let scale = (dx.hypot(dy) * 0.5).clamp(0.1, 12.0);
                entity.scale_x = scale;
                entity.scale_y = scale;
                entity.sync_to_components();
            }
        }
        EditorTool::Pivot => {
            if let Some(entity) = game.get_entity_by_id_mut(id) {
                let local = world_to_entity_local(entity, (world.0 as f64, world.1 as f64));
                let normalized = (
                    local.0 / entity.width.max(0.0001) + 0.5,
                    local.1 / entity.height.max(0.0001) + 0.5,
                );
                EditorSpatialTools2D::set_pivot(entity, normalized, false);
            }
        }
        EditorTool::Select | EditorTool::Collision | EditorTool::Paint => {}
    }
    state.drag_world_last = world;
    game.mark_scene_dirty(state.toolbar.active_tool().label());
}

fn finish_gizmo_drag(game: &mut Game, state: &mut EditorState) {
    let Some(id) = state.drag_entity.take() else {
        state.drag_entity_before = None;
        return;
    };
    if let Some(before) = state.drag_entity_before.take() {
        game.sync_world();
        game.push_editor_command(
            format!("{} Entity", state.toolbar.active_tool().label()),
            EditorCommandKind::MoveEntity { entity_id: id },
            before,
        );
    }
    state.snap_guides.clear();
}

fn begin_collision_vertex_drag(game: &mut Game, state: &mut EditorState, world: (f32, f32)) {
    let Some(id) = find_entity_at(game, world) else {
        return;
    };
    if game
        .get_entity_by_id(id)
        .is_some_and(|entity| entity.locked)
    {
        return;
    }
    game.select_entity(id);
    let before = game.capture_editor_snapshot();
    let local = game
        .get_entity_by_id(id)
        .map(|entity| world_to_entity_local(entity, (world.0 as f64, world.1 as f64)))
        .unwrap_or_default();
    if is_key_down(KeyCode::LeftAlt) || is_key_down(KeyCode::RightAlt) {
        if let Some(entity) = game.get_entity_by_id_mut(id)
            && EditorSpatialTools2D::add_collision_vertex(entity, local)
        {
            let index = EditorSpatialTools2D::collision_points(entity)
                .len()
                .saturating_sub(1);
            state.drag_collision_vertex = Some((id, index));
            state.drag_entity_before = Some(before);
        }
        return;
    }
    let Some(entity) = game.get_entity_by_id(id) else {
        return;
    };
    let points = EditorSpatialTools2D::collision_points(entity);
    let nearest = points
        .iter()
        .enumerate()
        .min_by(|(_, left), (_, right)| {
            (left.0 - local.0)
                .hypot(left.1 - local.1)
                .partial_cmp(&(right.0 - local.0).hypot(right.1 - local.1))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .filter(|(_, point)| (point.0 - local.0).hypot(point.1 - local.1) <= 0.4)
        .map(|(index, _)| index);
    if let Some(index) = nearest {
        state.drag_collision_vertex = Some((id, index));
        state.drag_entity_before = Some(before);
    }
}

fn apply_collision_vertex_drag(game: &mut Game, state: &mut EditorState, world: (f32, f32)) {
    let Some((id, index)) = state.drag_collision_vertex else {
        return;
    };
    let Some(entity) = game.get_entity_by_id_mut(id) else {
        return;
    };
    let local = world_to_entity_local(entity, (world.0 as f64, world.1 as f64));
    if EditorSpatialTools2D::move_collision_vertex(
        entity,
        index,
        local,
        state.snap_to_grid.then_some(0.125),
    ) {
        game.mark_scene_dirty("Edit Collision Shape");
    }
}

fn finish_collision_vertex_drag(game: &mut Game, state: &mut EditorState) {
    let Some((id, _)) = state.drag_collision_vertex.take() else {
        return;
    };
    if let Some(before) = state.drag_entity_before.take() {
        game.sync_world();
        game.push_editor_command(
            "Edit Collision Shape",
            EditorCommandKind::SceneOperation {
                name: format!("Collider2D #{id}"),
            },
            before,
        );
    }
}

fn world_to_entity_local(entity: &GameObject, world: (f64, f64)) -> (f64, f64) {
    let translated = (world.0 - entity.x, world.1 - entity.y);
    let radians = -entity.rotation.to_radians();
    (
        (translated.0 * radians.cos() - translated.1 * radians.sin())
            / entity.scale_x.abs().max(0.0001),
        (translated.0 * radians.sin() + translated.1 * radians.cos())
            / entity.scale_y.abs().max(0.0001),
    )
}

fn log_drop_outcome(game: &mut Game, asset_name: &str, outcome: DropOutcome) {
    match outcome {
        DropOutcome::SpawnedEntity(id) => game
            .console
            .log(format!("{asset_name} instanciado como #{id}"), "ASSETS"),
        DropOutcome::AppliedToEntity(id) => game
            .console
            .log(format!("{asset_name} aplicado a #{id}"), "ASSETS"),
        DropOutcome::OpenScene(scene_name) => game
            .console
            .log(format!("{asset_name} abrio escena {scene_name}"), "ASSETS"),
        DropOutcome::Unsupported(reason) => game
            .console
            .log(format!("{asset_name}: {reason}"), "WARNING"),
    }
}

fn select_at(game: &mut Game, state: &mut EditorState, world: (f32, f32), screen: (f32, f32)) {
    clear_ui_canvas_selection(state);
    let additive = is_key_down(KeyCode::LeftShift)
        || is_key_down(KeyCode::RightShift)
        || command_modifier_down();
    if let Some(id) =
        find_ui_entity_at(game, screen.0, screen.1).or_else(|| find_entity_at(game, world))
    {
        let group = game
            .get_entity_by_id(id)
            .and_then(|entity| entity.editor_group.clone());
        if let Some(group) = group {
            game.select_editor_group(&group, additive);
        } else if additive {
            game.toggle_entity_selection(id);
        } else {
            game.select_entity(id);
        }
        game.console.log(
            format!("{} entidades seleccionadas", game.selected_units.len()),
            "EDITOR",
        );
    } else if !additive {
        game.clear_selection();
    }
}

fn align_selected_entities(game: &mut Game, mode: AlignMode2D) {
    let before = game.capture_editor_snapshot();
    let changed = EditorSpatialTools2D::align(
        &mut game.runtime_world.units,
        &game.selected_units.clone(),
        mode,
    );
    if changed.is_empty() {
        game.console
            .log("Selecciona al menos dos objetos desbloqueados", "EDITOR");
        return;
    }
    game.sync_world();
    game.mark_scene_dirty("Align Selection");
    game.push_editor_command(
        format!("Align {mode:?}"),
        EditorCommandKind::SceneOperation {
            name: format!("Align {mode:?}"),
        },
        before,
    );
}

fn group_selected_entities(game: &mut Game) {
    if game.selected_units.len() < 2 {
        game.console
            .log("Selecciona al menos dos objetos para agrupar", "EDITOR");
        return;
    }
    let before = game.capture_editor_snapshot();
    let group_id = format!("group_{}", game.selected_units[0]);
    for id in game.selected_units.clone() {
        if let Some(entity) = game.get_entity_by_id_mut(id) {
            entity.editor_group = Some(group_id.clone());
        }
    }
    game.mark_scene_dirty("Group Selection");
    game.push_editor_command(
        "Group Selection",
        EditorCommandKind::SceneOperation {
            name: group_id.clone(),
        },
        before,
    );
    game.console
        .log(format!("Grupo creado: {group_id}"), "EDITOR");
}

fn ungroup_selected_entities(game: &mut Game) {
    let before = game.capture_editor_snapshot();
    let mut changed = false;
    for id in game.selected_units.clone() {
        if let Some(entity) = game.get_entity_by_id_mut(id) {
            changed |= entity.editor_group.take().is_some();
        }
    }
    if !changed {
        return;
    }
    game.mark_scene_dirty("Ungroup Selection");
    game.push_editor_command(
        "Ungroup Selection",
        EditorCommandKind::SceneOperation {
            name: "Ungroup".to_string(),
        },
        before,
    );
}

fn toggle_selected_layer_lock(game: &mut Game) {
    let Some(layer) = game
        .selected_units
        .first()
        .and_then(|id| game.get_entity_by_id(*id))
        .map(|entity| entity.layer.clone())
    else {
        return;
    };
    let before = game.capture_editor_snapshot();
    let should_lock = game
        .units
        .iter()
        .filter(|entity| entity.layer == layer)
        .any(|entity| !entity.locked);
    for entity in game
        .runtime_world
        .units
        .iter_mut()
        .filter(|entity| entity.layer == layer)
    {
        entity.locked = should_lock;
    }
    if should_lock {
        game.clear_selection();
    }
    game.mark_scene_dirty("Toggle Layer Lock");
    game.push_editor_command(
        format!(
            "{} layer {layer}",
            if should_lock { "Lock" } else { "Unlock" }
        ),
        EditorCommandKind::SceneOperation {
            name: format!("Layer Lock {layer}"),
        },
        before,
    );
}

fn toggle_selected_layer_visibility(game: &mut Game) {
    let Some(layer) = game
        .selected_units
        .first()
        .and_then(|id| game.get_entity_by_id(*id))
        .map(|entity| entity.layer.clone())
    else {
        return;
    };
    let before = game.capture_editor_snapshot();
    let should_show = game
        .units
        .iter()
        .filter(|entity| entity.layer == layer)
        .any(|entity| !entity.visible);
    for entity in game
        .runtime_world
        .units
        .iter_mut()
        .filter(|entity| entity.layer == layer)
    {
        entity.visible = should_show;
    }
    if !should_show {
        game.clear_selection();
    }
    game.mark_scene_dirty("Toggle Layer Visibility");
    game.push_editor_command(
        format!(
            "{} layer {layer}",
            if should_show { "Show" } else { "Hide" }
        ),
        EditorCommandKind::SceneOperation {
            name: format!("Layer Visibility {layer}"),
        },
        before,
    );
}

fn move_selection_to_next_layer(game: &mut Game) {
    if game.selected_units.is_empty() || game.tags_layers_manager.layers.is_empty() {
        return;
    }
    let before = game.capture_editor_snapshot();
    let layers = game.tags_layers_manager.layers.clone();
    for id in game.selected_units.clone() {
        if let Some(entity) = game.get_entity_by_id_mut(id) {
            let current = layers
                .iter()
                .position(|layer| layer == &entity.layer)
                .unwrap_or(0);
            entity.layer = layers[(current + 1) % layers.len()].clone();
        }
    }
    game.mark_scene_dirty("Move Selection Layer");
    game.push_editor_command(
        "Move Selection Layer",
        EditorCommandKind::SceneOperation {
            name: "Cycle Layer".to_string(),
        },
        before,
    );
}

fn run_python_tool(game: &mut Game, state: &mut EditorState, tool_id: &str) {
    let host = PythonAutomationHost::new(&game.project_path);
    if let Err(error) = host.install_builtin_tools() {
        game.console.log(format!("Python tools: {error}"), "ERROR");
        return;
    }
    let manifest = match host
        .discover()
        .ok()
        .and_then(|tools| tools.into_iter().find(|tool| tool.id == tool_id))
    {
        Some(manifest) => manifest,
        None => {
            game.console
                .log(format!("Python tool no encontrado: {tool_id}"), "ERROR");
            return;
        }
    };
    let context = PythonEditorContext {
        project_root: game.project_path.to_string_lossy().to_string(),
        active_scene: Some(game.scene_manager.current_scene.clone()),
        selected_entity_ids: game.selected_units.clone(),
        assets: game.asset_database.assets.keys().cloned().collect(),
        parameters: json!({
            "engine_version": crate::DEVELOPMENT_VERSION,
            "tool_id": tool_id,
        }),
    };
    match host.run(&manifest, context) {
        Ok(result) => {
            let level = if result.success { "PYTHON" } else { "ERROR" };
            game.console.log(result.message, level);
            for operation in result.operations {
                apply_python_editor_operation(game, state, operation);
            }
            state.show_console = true;
            state.bottom_tab = BottomTab::Console;
        }
        Err(error) => game
            .console
            .log(format!("Python automation: {error}"), "ERROR"),
    }
}

fn apply_python_editor_operation(
    game: &mut Game,
    state: &mut EditorState,
    operation: PythonEditorOperation,
) {
    match operation.operation.as_str() {
        "log" => game
            .console
            .log(operation.value.as_str().unwrap_or_default(), "PYTHON"),
        "select_entities" => {
            let ids = operation
                .value
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_u64)
                .filter(|id| game.get_entity_by_id(*id).is_some())
                .collect::<Vec<_>>();
            game.selected_units = ids;
        }
        "set_editor_property" => apply_python_bulk_properties(game, &operation),
        "open_document" => {
            let path = game.project_path.join(&operation.target);
            if path.exists() {
                open_project_file_in_editor(game, state, path);
            }
        }
        "request_reimport" | "refresh_assets" => {
            refresh_assets(game);
        }
        "create_asset_descriptor" => {
            let path = game.project_path.join(&operation.target);
            if let Err(error) = AssetTools::write_json(path, &operation.value) {
                game.console
                    .log(format!("Python asset descriptor: {error}"), "ERROR");
            } else {
                refresh_assets(game);
            }
        }
        "batch_import_assets" => {
            let destination = operation
                .value
                .get("destination")
                .and_then(Value::as_str)
                .unwrap_or("assets/imported");
            match batch_import_assets(&game.project_path, &operation.target, destination) {
                Ok(report) => finish_python_batch(game, "Import", report),
                Err(error) => game.console.log(format!("Import batch: {error}"), "ERROR"),
            }
            refresh_assets(game);
        }
        "convert_sprites" => {
            let destination = operation
                .value
                .get("destination")
                .and_then(Value::as_str)
                .unwrap_or("assets/sprites/converted");
            match batch_convert_sprites(&game.project_path, &operation.target, destination) {
                Ok(report) => finish_python_batch(game, "Sprite conversion", report),
                Err(error) => game
                    .console
                    .log(format!("Sprite conversion: {error}"), "ERROR"),
            }
            refresh_assets(game);
        }
        "generate_atlas" => {
            let destination = operation
                .value
                .get("destination")
                .and_then(Value::as_str)
                .unwrap_or("assets/atlases");
            let size = operation
                .value
                .get("size")
                .and_then(Value::as_u64)
                .unwrap_or(4096) as u32;
            let extrude = operation
                .value
                .get("extrude")
                .and_then(Value::as_u64)
                .unwrap_or(1) as u32;
            match generate_paged_sprite_atlases(
                &game.project_path,
                &operation.target,
                destination,
                size,
                extrude,
            ) {
                Ok(report) => finish_python_batch(game, "Atlas", report),
                Err(error) => game.console.log(format!("Atlas: {error}"), "ERROR"),
            }
            refresh_assets(game);
        }
        "create_procedural_level" => create_python_procedural_level(game, &operation),
        "export_project_data" => export_python_project_data(game, &operation),
        "automate_build" => automate_python_build(game),
        "process_animations" => process_python_animations(game, &operation),
        "generate_documentation" => generate_python_documentation(game, &operation),
        other => game
            .console
            .log(format!("Python operation ignorada: {other}"), "WARNING"),
    }
}

fn finish_python_batch(game: &mut Game, label: &str, report: PythonBatchReport) {
    game.console.log(
        format!(
            "{label}: {} procesados, {} omitidos, {} salidas",
            report.processed,
            report.skipped,
            report.output_files.len()
        ),
        "PYTHON",
    );
    for warning in report.warnings.into_iter().take(12) {
        game.console.log(warning, "WARNING");
    }
}

fn apply_python_bulk_properties(game: &mut Game, operation: &PythonEditorOperation) {
    if operation.target != "selection" || game.selected_units.is_empty() {
        game.console
            .log("Bulk properties: no hay selección", "WARNING");
        return;
    }
    let before = game.capture_editor_snapshot();
    let ids = game.selected_units.clone();
    let mut changed = 0usize;
    for id in ids {
        let Some(entity) = game.get_entity_by_id_mut(id) else {
            continue;
        };
        if let Some(value) = operation.value.get("visible").and_then(Value::as_bool) {
            entity.visible = value;
        }
        if let Some(value) = operation.value.get("locked").and_then(Value::as_bool) {
            entity.locked = value;
        }
        if let Some(value) = operation.value.get("enabled").and_then(Value::as_bool) {
            entity.enabled = value;
        }
        if let Some(value) = operation.value.get("tag").and_then(Value::as_str) {
            entity.tag = value.to_string();
        }
        if let Some(value) = operation.value.get("layer").and_then(Value::as_str) {
            entity.layer = value.to_string();
        }
        changed += 1;
    }
    game.sync_world();
    game.mark_scene_dirty("Python Bulk Properties");
    game.push_editor_command(
        "Python Bulk Properties",
        EditorCommandKind::SceneOperation {
            name: "Python Bulk Properties".to_string(),
        },
        before,
    );
    game.console.log(
        format!("Propiedades aplicadas a {changed} entidades"),
        "PYTHON",
    );
}

fn create_python_procedural_level(game: &mut Game, operation: &PythonEditorOperation) {
    let width = operation
        .value
        .get("width")
        .and_then(Value::as_u64)
        .unwrap_or(24)
        .clamp(4, 128) as usize;
    let height = operation
        .value
        .get("height")
        .and_then(Value::as_u64)
        .unwrap_or(16)
        .clamp(4, 128) as usize;
    let spacing = operation
        .value
        .get("spacing")
        .and_then(Value::as_f64)
        .unwrap_or(2.0)
        .clamp(0.25, 16.0);
    let name = if operation.target.trim().is_empty() {
        "PythonProcedural"
    } else {
        operation.target.as_str()
    };
    if let Err(error) = game.create_empty_scene(name) {
        game.console
            .log(format!("Nivel procedural: {error}"), "ERROR");
        return;
    }
    for x in 0..width {
        game.spawn_game_object("Boundary", x as f64 * spacing, 0.0);
        game.spawn_game_object(
            "Boundary",
            x as f64 * spacing,
            (height - 1) as f64 * spacing,
        );
    }
    for y in 1..height.saturating_sub(1) {
        game.spawn_game_object("Boundary", 0.0, y as f64 * spacing);
        game.spawn_game_object("Boundary", (width - 1) as f64 * spacing, y as f64 * spacing);
    }
    let center = (width as f64 * spacing * 0.5, height as f64 * spacing * 0.5);
    for (name, component, offset) in [
        ("KeyLight2D", "Light2D", (-4.0, -2.0)),
        ("WaterArea2D", "Water2D", (3.0, 1.0)),
        ("FogVolume2D", "Fog2D", (0.0, 4.0)),
        ("FireEmitter2D", "Fire2D", (5.0, -3.0)),
    ] {
        let id = game.spawn_game_object(name, center.0 + offset.0, center.1 + offset.1);
        if let Some(component) = default_component(component)
            && let Some(entity) = game.get_entity_by_id_mut(id)
        {
            entity.add_component(component);
        }
    }
    game.sync_world();
    if let Err(error) = game.save_scene() {
        game.console
            .log(format!("Guardar nivel procedural: {error}"), "ERROR");
    } else {
        game.console.log(
            format!("Nivel procedural {name}: {width}x{height}"),
            "PYTHON",
        );
    }
}

fn export_python_project_data(game: &mut Game, operation: &PythonEditorOperation) {
    let target = if operation.target.trim().is_empty() {
        ".miniforge/generated/exports"
    } else {
        operation.target.as_str()
    };
    let output = game.project_path.join(target);
    let result: io::Result<()> = (|| {
        fs::create_dir_all(&output)?;
        let manifest = game.build_manifest()?;
        AssetTools::write_json(output.join("manifest.json"), &manifest)?;
        AssetTools::write_json(
            output.join("scene.json"),
            &json!({
                "scene": game.scene_manager.current_scene,
                "entities": game.runtime_world.units,
                "ui_canvases": game.ui_canvases,
            }),
        )?;
        let assets = game
            .asset_database
            .assets
            .values()
            .map(|asset| {
                json!({
                    "name": asset.name,
                    "type": asset.asset_type,
                    "path": asset.relative_path,
                })
            })
            .collect::<Vec<_>>();
        AssetTools::write_json(output.join("assets.json"), &json!(assets))?;
        Ok(())
    })();
    match result {
        Ok(()) => game
            .console
            .log(format!("Datos exportados: {}", output.display()), "PYTHON"),
        Err(error) => game
            .console
            .log(format!("Exportar datos: {error}"), "ERROR"),
    }
}

fn automate_python_build(game: &mut Game) {
    if !game.validate_project() {
        game.console
            .log("Build detenido: valida los errores del proyecto", "ERROR");
        return;
    }
    if let Err(error) = game
        .build_manifest()
        .and_then(|_| game.export_runtime(ExportProfile::Debug).map(|_| ()))
    {
        game.console
            .log(format!("Build automático: {error}"), "ERROR");
    } else {
        game.console.log("Build debug automático listo", "PYTHON");
    }
}

fn process_python_animations(game: &mut Game, operation: &PythonEditorOperation) {
    let animations = game
        .asset_database
        .assets
        .values()
        .filter(|asset| {
            asset.relative_path.contains("animations/")
                || asset.asset_type.to_ascii_lowercase().contains("animation")
                || asset.relative_path.ends_with(".spriteframes")
        })
        .map(|asset| {
            json!({
                "name": asset.name,
                "type": asset.asset_type,
                "path": asset.relative_path,
                "status": "validated",
            })
        })
        .collect::<Vec<_>>();
    let output = game
        .project_path
        .join(".miniforge/generated/animations/animation_report.json");
    let value = json!({
        "requested": operation.value,
        "count": animations.len(),
        "animations": animations,
    });
    match AssetTools::write_json(&output, &value) {
        Ok(()) => game.console.log(
            format!("Animaciones procesadas: {}", value["count"]),
            "PYTHON",
        ),
        Err(error) => game
            .console
            .log(format!("Procesar animaciones: {error}"), "ERROR"),
    }
}

fn generate_python_documentation(game: &mut Game, operation: &PythonEditorOperation) {
    let target = if operation.target.trim().is_empty() {
        ".miniforge/generated/docs"
    } else {
        operation.target.as_str()
    };
    let output = game.project_path.join(target).join("PROJECT_AUTOMATION.md");
    let mut asset_types = std::collections::BTreeMap::<String, usize>::new();
    for asset in game.asset_database.assets.values() {
        *asset_types.entry(asset.asset_type.clone()).or_default() += 1;
    }
    let tools = PythonAutomationHost::new(&game.project_path)
        .discover()
        .unwrap_or_default();
    let mut markdown = format!(
        "# MiniForge Project Automation\n\n- Scene: `{}`\n- Entities: {}\n- Assets: {}\n- Python tools: {}\n\n## Asset types\n\n",
        game.scene_manager.current_scene,
        game.runtime_world.units.len(),
        game.asset_database.assets.len(),
        tools.len(),
    );
    for (kind, count) in asset_types {
        markdown.push_str(&format!("- {kind}: {count}\n"));
    }
    markdown.push_str("\n## Python editor tools\n\n");
    for tool in tools {
        markdown.push_str(&format!(
            "- **{}** (`{}`): {}\n",
            tool.label, tool.id, tool.description
        ));
    }
    markdown.push_str("\n## 2D rendering effects\n\n");
    for preset in crate::engine::render_2d::production_effect_presets_2d() {
        markdown.push_str(&format!(
            "- **{}**: component `{}`, shader `{}`\n",
            preset.label, preset.component, preset.shader
        ));
    }
    if let Some(parent) = output.parent() {
        let _ = fs::create_dir_all(parent);
    }
    match fs::write(&output, markdown) {
        Ok(()) => game.console.log(
            format!("Documentación generada: {}", output.display()),
            "PYTHON",
        ),
        Err(error) => game
            .console
            .log(format!("Generar documentación: {error}"), "ERROR"),
    }
}

fn find_ui_entity_at(game: &Game, mx: f32, my: f32) -> Option<u64> {
    game.runtime_world
        .units
        .iter()
        .rev()
        .find(|entity| {
            if !entity.enabled || !entity.visible || entity.locked {
                return false;
            }
            let Some(ui) = entity.get_component("UIElement") else {
                return false;
            };
            let rect = RectSpec {
                x: ui.get_f64("x", 0.0) as f32,
                y: ui.get_f64("y", 0.0) as f32,
                w: ui.get_f64("width", 160.0) as f32,
                h: ui.get_f64("height", 36.0) as f32,
            };
            contains(rect, mx, my)
        })
        .map(|entity| entity.id)
}

fn ui_drag_offset(game: &Game, entity_id: u64, mx: f32, my: f32) -> (f32, f32) {
    game.get_entity_by_id(entity_id)
        .and_then(|entity| entity.get_component("UIElement"))
        .map(|ui| {
            (
                mx - ui.get_f64("x", 0.0) as f32,
                my - ui.get_f64("y", 0.0) as f32,
            )
        })
        .unwrap_or((0.0, 0.0))
}

fn find_entity_at(game: &Game, world: (f32, f32)) -> Option<u64> {
    game.runtime_world
        .units
        .iter()
        .rev()
        .find(|entity| {
            if !entity.enabled || !entity.visible || entity.locked {
                return false;
            }
            let half_w = (entity.width.max(entity.radius * 2.0) * 0.5) as f32;
            let half_h = (entity.height.max(entity.radius * 2.0) * 0.5) as f32;
            world.0 >= entity.x as f32 - half_w
                && world.0 <= entity.x as f32 + half_w
                && world.1 >= entity.y as f32 - half_h
                && world.1 <= entity.y as f32 + half_h
        })
        .map(|entity| entity.id)
}

fn command_selected_move(game: &mut Game, world: (f32, f32)) {
    let target = (world.0.round() as i32, world.1.round() as i32);
    let selected_ids = game.selected_units.clone();
    let grid = game.grid.clone();
    let team_id = selected_ids
        .first()
        .map(|id| game.team_id_of(*id))
        .unwrap_or(1);
    let threats = game.rts_threat_sources_for_team(team_id);
    let positions = CommandSystem::formation_targets(
        Some(&grid),
        selected_ids.len(),
        (target.0 as f64, target.1 as f64),
        "square",
        1.0,
    );
    for (id, position) in selected_ids.into_iter().zip(positions) {
        if let Some(entity) = game.get_entity_by_id_mut(id) {
            if threats.is_empty() {
                CommandSystem::move_unit_to_grid(
                    &grid,
                    entity,
                    (position.0 as i32, position.1 as i32),
                );
            } else {
                CommandSystem::threat_aware_move_units(
                    &grid,
                    std::slice::from_mut(entity),
                    (position.0 as i32, position.1 as i32),
                    &threats,
                );
            }
            game.mark_scene_dirty("Move Command");
        }
    }
    game.sync_world();
}

fn toggle_play_mode(game: &mut Game, state: &mut EditorState) {
    if state.external_play_child.is_some() {
        stop_external_play_window(game, state);
    } else if game.mode == "PLAY" {
        game.exit_play_mode("floating play stop");
        state.play_window_open = false;
    } else {
        launch_play_window(game, state);
    }
}

fn launch_play_window(game: &mut Game, state: &mut EditorState) {
    match game.export_runtime(ExportProfile::Debug) {
        Ok(report) => {
            state.external_play_path = Some(report.output_path.clone());
            if let Some(runtime_exe) = sibling_runtime_binary() {
                match Command::new(runtime_exe)
                    .arg("--build")
                    .arg(&report.output_path)
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()
                {
                    Ok(child) => {
                        state.external_play_child = Some(child);
                        state.play_window_open = true;
                        game.docking_workspace
                            .set_floating_visibility("play_window", true);
                        game.console.log(
                            format!("Play Window externo abierto: {}", report.output_path.display()),
                            "ENGINE",
                        );
                        return;
                    }
                    Err(error) => game.console.log(
                        format!(
                            "No se pudo lanzar runtime externo ({error}); usando Play flotante interno."
                        ),
                        "WARNING",
                    ),
                }
            } else {
                game.console.log(
                    "Runtime externo no encontrado junto al editor; usando Play flotante interno.",
                    "WARNING",
                );
            }
        }
        Err(error) => game.console.log(
            format!("No se pudo preparar build de Play externo: {error}"),
            "ERROR",
        ),
    }

    game.enter_play_mode();
    state.play_window_open = true;
    game.docking_workspace
        .set_floating_visibility("play_window", true);
}

fn stop_external_play_window(game: &mut Game, state: &mut EditorState) {
    if let Some(mut child) = state.external_play_child.take() {
        if let Err(error) = child.kill() {
            game.console.log(
                format!("No se pudo detener runtime externo: {error}"),
                "WARNING",
            );
        }
        let _ = child.wait();
        game.console.log("Play Window externo detenido", "ENGINE");
    }
    state.external_play_path = None;
    state.play_window_open = false;
}

fn poll_external_play_window(game: &mut Game, state: &mut EditorState) {
    let Some(child) = state.external_play_child.as_mut() else {
        return;
    };
    match child.try_wait() {
        Ok(Some(status)) => {
            state.external_play_child = None;
            state.external_play_path = None;
            state.play_window_open = false;
            game.console
                .log(format!("Play Window externo cerró: {status}"), "ENGINE");
        }
        Ok(None) => {}
        Err(error) => {
            state.external_play_child = None;
            state.external_play_path = None;
            state.play_window_open = false;
            game.console.log(
                format!("Play Window externo no responde: {error}"),
                "WARNING",
            );
        }
    }
}

fn poll_editor_file_changes(game: &mut Game, state: &mut EditorState) {
    let changes = state
        .file_watcher
        .as_ref()
        .map(EditorFileWatcher::drain)
        .unwrap_or_default();
    if changes.is_empty() {
        return;
    }

    let changed_paths = changes
        .iter()
        .flat_map(|change| change.paths.iter())
        .collect::<Vec<_>>();
    let active_path = game.script_editor.document.path.clone();
    if !game.script_editor.document.dirty
        && active_path
            .as_ref()
            .is_some_and(|active| changed_paths.contains(&active))
    {
        if let Err(error) = game.reload_open_file() {
            game.console
                .log(format!("Hot reload de editor falló: {error}"), "WARNING");
        } else {
            game.console
                .log("Script recargado por cambio externo", "SCRIPT");
        }
    }

    if changed_paths.iter().any(|path| {
        let text = path.to_string_lossy();
        text.contains("/assets/") || text.contains("/scripts/") || text.contains("/saves/scenes/")
    }) && let Err(error) = game.refresh_assets()
    {
        game.console.log(
            format!("Actualización automática de assets falló: {error}"),
            "WARNING",
        );
    }
}

fn sibling_runtime_binary() -> Option<PathBuf> {
    let current = env::current_exe().ok()?;
    let runtime_name = if cfg!(windows) {
        "miniforge_runtime.exe"
    } else {
        "miniforge_runtime"
    };
    let candidate = current.with_file_name(runtime_name);
    candidate.exists().then_some(candidate)
}

fn set_tool(game: &mut Game, state: &mut EditorState, tool: EditorTool) {
    state.toolbar.activate(tool);
    game.active_tool = tool.label().to_string();
    game.console
        .log(format!("Herramienta activa: {}", tool.label()), "EDITOR");
}

fn draw_top_bar(game: &mut Game, state: &mut EditorState, rect: RectSpec) {
    sync_menu_state(game, state);
    draw_gradient_rect(
        rect,
        Color::from_rgba(20, 25, 34, 255),
        Color::from_rgba(13, 16, 22, 255),
    );
    draw_macos_chrome(rect);
    draw_rectangle(
        rect.x,
        rect.y + 44.0,
        rect.w,
        1.0,
        Color::from_rgba(54, 66, 86, 150),
    );
    draw_rectangle(rect.x, rect.y + rect.h - 1.0, rect.w, 1.0, ui_line_soft());
    draw_text("MiniForge", rect.x + 82.0, rect.y + 25.0, 24.0, ui_text());
    draw_top_menus(state, rect);
    draw_text(
        &ellipsize(
            &format!(
                "{} | {} | {}",
                crate::version_label(),
                game.mode,
                game.scene_summary()
            ),
            92,
        ),
        rect.x + 472.0,
        rect.y + 23.0,
        15.0,
        ui_text_muted(),
    );

    let mut x = rect.x + 16.0;
    let y = rect.y + 55.0;
    let play_active = game.mode == "PLAY" || state.external_play_child.is_some();
    let play_label = if play_active { "Stop" } else { "Play" };
    if icon_button(
        state.phosphor_font.as_ref(),
        x,
        y,
        58.0,
        24.0,
        if play_active {
            EditorIcon::Stop
        } else {
            EditorIcon::Play
        },
        play_label,
        play_active,
    ) {
        toggle_play_mode(game, state);
    }
    x += 66.0;
    if icon_button(
        state.phosphor_font.as_ref(),
        x,
        y,
        58.0,
        24.0,
        EditorIcon::Save,
        "Save",
        false,
    ) {
        save_project(game);
    }
    x += 66.0;
    if icon_button(
        state.phosphor_font.as_ref(),
        x,
        y,
        74.0,
        24.0,
        EditorIcon::NewEntity,
        "Create",
        false,
    ) {
        state.menu_bar.open(TopMenu::Create.id());
    }
    x += 82.0;
    if button(x, y, 98.0, 24.0, "Foundations", false) {
        game.prepare_complex_game_foundations();
        state.show_console = true;
        state.bottom_tab = BottomTab::Profiler;
    }
    x += 106.0;
    if icon_button(
        state.phosphor_font.as_ref(),
        x,
        y,
        70.0,
        24.0,
        EditorIcon::Folder,
        "Assets",
        false,
    ) {
        refresh_assets(game);
        state.show_console = true;
        state.bottom_tab = BottomTab::Assets;
    }
    x += 78.0;
    if icon_button(
        state.phosphor_font.as_ref(),
        x,
        y,
        82.0,
        24.0,
        EditorIcon::Scene,
        "Launcher",
        false,
    ) {
        state.launcher_overlay = Some(new_launcher_state());
    }
    x += 96.0;

    let tools = state.toolbar.descriptors().to_vec();
    for descriptor in tools {
        let width = f32::from(descriptor.width_hint);
        if button(
            x,
            y,
            width,
            24.0,
            &descriptor.label,
            state.toolbar.active_tool() == descriptor.tool,
        ) {
            set_tool(game, state, descriptor.tool);
        }
        x += width + 5.0;
    }
    if button(x, y, 54.0, 24.0, "Snap", state.snap_to_grid) {
        state.snap_to_grid = !state.snap_to_grid;
    }
    x += 62.0;
    if button(
        x,
        y,
        74.0,
        24.0,
        state.tile_brush_mode.label(),
        state.toolbar.active_tool() == EditorTool::Paint,
    ) {
        state.tile_brush_mode = state.tile_brush_mode.next();
    }
    x += 80.0;
    if button(x, y, 54.0, 24.0, "Smart", state.smart_snap) {
        state.smart_snap = !state.smart_snap;
    }
    x += 59.0;
    if button(x, y, 46.0, 24.0, "Coll", state.show_collisions) {
        state.show_collisions = !state.show_collisions;
    }
    x += 51.0;
    if icon_button(
        state.phosphor_font.as_ref(),
        x,
        y,
        46.0,
        24.0,
        EditorIcon::Camera,
        "Cam",
        state.show_camera_frame,
    ) {
        state.show_camera_frame = !state.show_camera_frame;
    }

    let modes = [
        WorkspaceMode::WorldBuilding,
        WorkspaceMode::Scripting,
        WorkspaceMode::PrefabEditing,
        WorkspaceMode::Profiling,
        WorkspaceMode::Shipping,
    ];
    let mut wx = rect.x + rect.w - 380.0;
    for mode in modes {
        if button(
            wx,
            rect.y + 8.0,
            68.0,
            22.0,
            mode.label(),
            game.editor_workspace.active_mode == mode,
        ) {
            game.set_workspace_mode(mode);
            match mode {
                WorkspaceMode::Scripting => {
                    state.show_console = true;
                    state.bottom_tab = BottomTab::Programming;
                }
                WorkspaceMode::PrefabEditing => {
                    state.show_console = true;
                    state.bottom_tab = BottomTab::Prefabs;
                }
                WorkspaceMode::Profiling => {
                    state.show_console = true;
                    state.bottom_tab = BottomTab::Profiler;
                }
                WorkspaceMode::Shipping => {
                    state.show_console = true;
                    state.bottom_tab = BottomTab::Assets;
                }
                WorkspaceMode::WorldBuilding => {}
            }
        }
        wx += 74.0;
    }
    draw_text(
        &ellipsize(&game.editor_workspace.workflow_summary(), 68),
        rect.x + rect.w - 430.0,
        rect.y + 35.0,
        14.0,
        ui_text_muted(),
    );
}

fn sync_menu_state(game: &Game, state: &mut EditorState) {
    state.menu_bar.set_item_checked(
        "toggle_browser",
        state.show_console && state.bottom_tab == BottomTab::Assets,
    );
    state
        .menu_bar
        .set_item_checked("toggle_hierarchy", state.show_hierarchy);
    state
        .menu_bar
        .set_item_checked("toggle_inspector", state.show_inspector);
    state
        .menu_bar
        .set_item_checked("toggle_smart_snap", state.smart_snap);
    state
        .menu_bar
        .set_item_checked("toggle_collision_overlay", state.show_collisions);
    state
        .menu_bar
        .set_item_checked("toggle_camera_frame", state.show_camera_frame);
    let can_build = game.mode != "PLAY" && state.external_play_child.is_none();
    for command in [
        "export_debug",
        "export_release",
        "package_debug",
        "package_release",
    ] {
        state.menu_bar.set_item_enabled(command, can_build);
    }
}

fn draw_top_menus(state: &mut EditorState, rect: RectSpec) {
    let mut x = rect.x + 190.0;
    for menu in [
        TopMenu::File,
        TopMenu::Create,
        TopMenu::View,
        TopMenu::Project,
        TopMenu::Rts,
    ] {
        let width = match menu {
            TopMenu::Project => 62.0,
            TopMenu::Create => 58.0,
            _ => 48.0,
        };
        if button(
            x,
            rect.y + 7.0,
            width,
            22.0,
            menu.label(),
            state.menu_bar.is_open(menu.id()),
        ) {
            state.menu_bar.toggle(menu.id());
        }
        x += width + 5.0;
    }
}

fn draw_top_menu_overlay(game: &mut Game, state: &mut EditorState, rect: RectSpec) {
    let Some(menu) = state
        .menu_bar
        .open_menu
        .as_deref()
        .and_then(TopMenu::from_id)
    else {
        return;
    };
    let (x, width) = top_menu_anchor(rect, menu);
    let y = rect.y + 32.0;
    let bounds = menu_popover_rect(state.menu_bar.items(menu.id()), x, y, width);
    if is_mouse_button_pressed(MouseButton::Left)
        && !contains_mouse(bounds)
        && mouse_position().1 > rect.y + 34.0
    {
        state.menu_bar.close();
        return;
    }
    draw_menu_popover(game, state, menu, x, y, width);
}

fn top_menu_anchor(rect: RectSpec, target: TopMenu) -> (f32, f32) {
    let mut x = rect.x + 190.0;
    for menu in [
        TopMenu::File,
        TopMenu::Create,
        TopMenu::View,
        TopMenu::Project,
        TopMenu::Rts,
    ] {
        let button_width = match menu {
            TopMenu::Project => 62.0,
            TopMenu::Create => 58.0,
            _ => 48.0,
        };
        if menu == target {
            return (x, menu_popover_width(menu).max(button_width));
        }
        x += button_width + 5.0;
    }
    (rect.x + 190.0, menu_popover_width(target))
}

fn menu_popover_width(menu: TopMenu) -> f32 {
    match menu {
        TopMenu::Project | TopMenu::Rts => 218.0,
        TopMenu::View => 196.0,
        _ => 184.0,
    }
}

fn menu_popover_rect(
    items: &[crate::engine::ui::MenuItem],
    x: f32,
    y: f32,
    width: f32,
) -> RectSpec {
    RectSpec {
        x,
        y,
        w: width,
        h: menu_popover_height(items),
    }
}

fn menu_popover_height(items: &[crate::engine::ui::MenuItem]) -> f32 {
    items.len() as f32 * 24.0
        + items.iter().filter(|item| item.separator_before).count() as f32 * 5.0
        + 12.0
}

fn draw_menu_popover(
    game: &mut Game,
    state: &mut EditorState,
    menu: TopMenu,
    x: f32,
    y: f32,
    width: f32,
) {
    let items = state.menu_bar.items(menu.id()).to_vec();
    let height = menu_popover_height(&items);
    draw_surface(
        RectSpec {
            x,
            y,
            w: width,
            h: height,
        },
        true,
    );
    let mut row_y = y + 26.0;
    for (index, item) in items.iter().enumerate() {
        if item.separator_before {
            draw_rectangle(x + 10.0, row_y - 20.0, width - 20.0, 1.0, ui_line_soft());
            row_y += 5.0;
        }
        let row = RectSpec {
            x: x + 6.0,
            y: row_y - 18.0,
            w: width - 12.0,
            h: 22.0,
        };
        let hovered = item.enabled && contains_mouse(row);
        let focused = state.menu_bar.focused_item == Some(index);
        if hovered {
            state.menu_bar.focused_item = Some(index);
        }
        draw_rect(
            row,
            if hovered || focused {
                Color::from_rgba(47, 63, 86, 255)
            } else {
                Color::from_rgba(24, 31, 44, 255)
            },
        );
        draw_rectangle(
            row.x,
            row.y,
            2.0,
            row.h,
            if hovered || focused {
                ui_accent()
            } else {
                ui_line_soft()
            },
        );
        draw_text_fit(
            &format!("{}{}", if item.checked { "✓ " } else { "" }, item.label),
            RectSpec {
                x: row.x + 8.0,
                y: row.y + 1.0,
                w: row.w - 64.0,
                h: row.h - 2.0,
            },
            14,
            if item.enabled {
                ui_text()
            } else {
                ui_text_muted()
            },
            TextAlign::Left,
        );
        if let Some(shortcut) = &item.shortcut {
            draw_text_fit(
                shortcut,
                RectSpec {
                    x: row.x + row.w - 58.0,
                    y: row.y + 1.0,
                    w: 50.0,
                    h: row.h - 2.0,
                },
                11,
                ui_text_muted(),
                TextAlign::Right,
            );
        }
        if hovered
            && is_mouse_button_pressed(MouseButton::Left)
            && let Some(command) = state.menu_bar.activate(menu.id(), index)
        {
            run_palette_command(game, state, &command);
        }
        row_y += 24.0;
    }
}

fn draw_hierarchy(game: &mut Game, state: &mut EditorState, rect: RectSpec) {
    if rect.w <= 0.0 {
        return;
    }
    draw_surface(rect, false);
    draw_panel_header(rect, "Hierarchy", "scene graph");
    draw_text(
        &ellipsize(
            &format!(
                "{} | {}",
                game.scene_manager.current_scene,
                game.editor_workspace.active_mode.label()
            ),
            34,
        ),
        rect.x + 14.0,
        rect.y + 49.0,
        14.0,
        ui_text_muted(),
    );
    draw_text(
        "F5 Play snapshot | F11 pausa sim | F2/F3 Hierarchy/Inspector",
        rect.x + 14.0,
        rect.y + 66.0,
        12.0,
        ui_text_muted(),
    );
    let visible_count = game
        .runtime_world
        .units
        .iter()
        .filter(|entity| entity.visible)
        .count();
    let locked_count = game
        .runtime_world
        .units
        .iter()
        .filter(|entity| entity.locked)
        .count();
    let prefab_count = game
        .units
        .iter()
        .filter(|entity| entity.is_prefab_instance)
        .count();
    draw_text(
        &format!(
            "{} objs | {} visibles | {} locked | {} prefabs",
            game.runtime_world.units.len(),
            visible_count,
            locked_count,
            prefab_count
        ),
        rect.x + 14.0,
        rect.y + 88.0,
        12.0,
        ui_text_muted(),
    );
    let quick_y = rect.y + 96.0;
    if button(rect.x + 14.0, quick_y, 66.0, 21.0, "Show All", false) {
        for entity in &mut game.runtime_world.units {
            entity.visible = true;
        }
        game.sync_world();
        game.mark_scene_dirty("Show All");
    }
    if button(rect.x + 86.0, quick_y, 68.0, 21.0, "Unlock", false) {
        for entity in &mut game.runtime_world.units {
            entity.locked = false;
        }
        game.mark_scene_dirty("Unlock All");
    }
    if button(rect.x + 160.0, quick_y, 58.0, 21.0, "Focus", false)
        && let Some(id) = selected_id(game)
        && let Some((entity_x, entity_y)) =
            game.get_entity_by_id(id).map(|entity| (entity.x, entity.y))
    {
        game.camera.x = entity_x * game.grid.tile_size as f64 - 280.0;
        game.camera.y = entity_y * game.grid.tile_size as f64 - 180.0;
        game.camera.clamp_to_bounds();
    }
    if button(
        rect.x + rect.w - 72.0,
        rect.y + 9.0,
        56.0,
        22.0,
        "+Scene",
        false,
    ) {
        create_new_scene(game);
    }

    let list_rect = RectSpec {
        x: rect.x + 8.0,
        y: rect.y + 126.0,
        w: rect.w - 16.0,
        h: (rect.h - 138.0).max(0.0),
    };
    let row_h = 27.0;
    let content_h = game.runtime_world.units.len() as f32 * row_h;
    let max_scroll = (content_h - list_rect.h).max(0.0);
    if contains_mouse(list_rect) {
        let (_, wheel_y) = mouse_wheel();
        if wheel_y.abs() > f32::EPSILON {
            state.hierarchy_scroll =
                (state.hierarchy_scroll - wheel_y * row_h * 1.2).clamp(0.0, max_scroll);
        }
    }
    state.hierarchy_scroll = state.hierarchy_scroll.clamp(0.0, max_scroll);
    let first_row = (state.hierarchy_scroll / row_h).floor().max(0.0) as usize;
    let visible_rows = (list_rect.h / row_h).ceil().max(1.0) as usize + 2;
    struct HierarchyRow {
        id: u64,
        name: String,
        tag: String,
        selected: bool,
        visible: bool,
        locked: bool,
        component_count: usize,
        depth: usize,
    }

    let rows: Vec<HierarchyRow> = game
        .units
        .iter()
        .skip(first_row)
        .take(visible_rows)
        .map(|entity| HierarchyRow {
            id: entity.id,
            name: entity.name.clone(),
            tag: entity.tag.clone(),
            selected: game.selected_units.contains(&entity.id),
            visible: entity.visible,
            locked: entity.locked,
            component_count: entity.components.len(),
            depth: entity_depth_in_units(&game.runtime_world.units, entity.id),
        })
        .collect();

    let mut y = list_rect.y + 19.0 - (state.hierarchy_scroll % row_h);
    for HierarchyRow {
        id,
        name,
        tag,
        selected,
        visible,
        locked,
        component_count,
        depth,
    } in rows
    {
        let row = RectSpec {
            x: list_rect.x,
            y: y - 17.0,
            w: list_rect.w,
            h: 24.0,
        };
        if row.y + row.h < list_rect.y || row.y > list_rect.y + list_rect.h {
            y += row_h;
            continue;
        }
        let hovered = contains_mouse(row);
        let color = if selected {
            Color::from_rgba(42, 86, 128, 255)
        } else if hovered {
            Color::from_rgba(36, 48, 66, 255)
        } else {
            Color::from_rgba(22, 28, 40, 255)
        };
        draw_rect(row, color);
        draw_rectangle(row.x, row.y, 3.0, row.h, entity_type_accent(&tag));
        let indent = (depth as f32 * 12.0).min(54.0);
        if depth > 0 {
            draw_line(
                row.x + 10.0 + indent,
                row.y + 5.0,
                row.x + 10.0 + indent,
                row.y + row.h - 5.0,
                1.0,
                ui_line_soft(),
            );
        }
        draw_text(
            &ellipsize(&format!("{name}  #{id}"), 25),
            rect.x + 16.0 + indent,
            y,
            16.0,
            if visible { ui_text() } else { ui_text_muted() },
        );
        draw_text(
            &ellipsize(&format!("{tag} [{component_count}]"), 14),
            rect.x + rect.w - 112.0,
            y,
            13.0,
            ui_text_muted(),
        );
        if button(
            row.x + row.w - 54.0,
            row.y + 2.0,
            23.0,
            19.0,
            if visible { "V" } else { "H" },
            visible,
        ) && let Some(entity) = game.get_entity_by_id_mut(id)
        {
            entity.visible = !entity.visible;
            game.mark_scene_dirty("Toggle Visibility");
        }
        if button(
            row.x + row.w - 27.0,
            row.y + 2.0,
            23.0,
            19.0,
            if locked { "L" } else { "-" },
            locked,
        ) && let Some(entity) = game.get_entity_by_id_mut(id)
        {
            entity.locked = !entity.locked;
            game.mark_scene_dirty("Toggle Lock");
        }
        if hovered && is_mouse_button_pressed(MouseButton::Left) {
            game.select_entity(id);
            game.console
                .log(format!("Seleccionado desde Hierarchy #{id}"), "EDITOR");
        }
        if hovered && is_mouse_button_pressed(MouseButton::Right) {
            state.hierarchy_context_entity = Some(id);
            state.hierarchy_context_pos = mouse_position();
        }
        y += row_h;
    }
    draw_scrollbar(
        RectSpec {
            x: list_rect.x + list_rect.w - 7.0,
            y: list_rect.y,
            w: 6.0,
            h: list_rect.h,
        },
        state.hierarchy_scroll,
        max_scroll,
        content_h.max(list_rect.h),
    );
    draw_hierarchy_context_menu(game, state);

    draw_rectangle_lines(
        rect.x + rect.w - 1.0,
        rect.y,
        1.0,
        rect.h,
        1.0,
        ui_line_soft(),
    );
}

fn draw_hierarchy_context_menu(game: &mut Game, state: &mut EditorState) {
    let Some(entity_id) = state.hierarchy_context_entity else {
        return;
    };
    if game.get_entity_by_id(entity_id).is_none() {
        state.hierarchy_context_entity = None;
        return;
    }
    let panel = RectSpec {
        x: state
            .hierarchy_context_pos
            .0
            .clamp(8.0, (screen_width() - 154.0).max(8.0)),
        y: state
            .hierarchy_context_pos
            .1
            .clamp(8.0, (screen_height() - 236.0).max(8.0)),
        w: 146.0,
        h: 228.0,
    };
    draw_rect(panel, Color::from_rgba(26, 31, 42, 252));
    draw_rectangle_lines(
        panel.x,
        panel.y,
        panel.w,
        panel.h,
        1.0,
        Color::from_rgba(92, 112, 145, 255),
    );
    draw_text(
        &format!("Entity #{entity_id}"),
        panel.x + 10.0,
        panel.y + 21.0,
        13.0,
        Color::from_rgba(220, 230, 244, 255),
    );
    let mut y = panel.y + 32.0;
    for (label, action) in [
        ("Select", "select"),
        ("Focus", "focus"),
        ("Hide/Show", "toggle_visible"),
        ("Lock/Unlock", "toggle_lock"),
        ("Move Up", "move_up"),
        ("Move Down", "move_down"),
        ("Parent Sel", "parent_selected"),
        ("Clear Parent", "clear_parent"),
        ("Delete", "delete"),
    ] {
        if button(panel.x + 8.0, y, panel.w - 16.0, 20.0, label, false) {
            match action {
                "select" => {
                    game.select_entity(entity_id);
                }
                "focus" => {
                    if let Some((entity_x, entity_y)) = game
                        .get_entity_by_id(entity_id)
                        .map(|entity| (entity.x, entity.y))
                    {
                        game.camera.x = entity_x * game.grid.tile_size as f64 - 280.0;
                        game.camera.y = entity_y * game.grid.tile_size as f64 - 180.0;
                        game.camera.clamp_to_bounds();
                    }
                }
                "toggle_visible" => {
                    if let Some(entity) = game.get_entity_by_id_mut(entity_id) {
                        entity.visible = !entity.visible;
                        game.mark_scene_dirty("Toggle Visibility");
                    }
                }
                "toggle_lock" => {
                    if let Some(entity) = game.get_entity_by_id_mut(entity_id) {
                        entity.locked = !entity.locked;
                        game.mark_scene_dirty("Toggle Lock");
                    }
                }
                "move_up" => {
                    game.move_entity_in_hierarchy(entity_id, -1);
                }
                "move_down" => {
                    game.move_entity_in_hierarchy(entity_id, 1);
                }
                "parent_selected" => {
                    if let Some(child_id) = selected_id(game)
                        && child_id != entity_id
                    {
                        game.set_entity_parent(child_id, entity_id);
                    }
                }
                "clear_parent" => {
                    game.clear_entity_parent(entity_id);
                }
                "delete" => {
                    game.select_entity(entity_id);
                    delete_selected(game);
                }
                _ => {}
            }
            state.hierarchy_context_entity = None;
            return;
        }
        y += 22.0;
    }
    if (is_mouse_button_pressed(MouseButton::Left) || is_mouse_button_pressed(MouseButton::Right))
        && !contains_mouse(panel)
    {
        state.hierarchy_context_entity = None;
    }
}

fn draw_inspector_scene_ui_canvas(game: &mut Game, rect: RectSpec) {
    let mut y = rect.y + 58.0;
    draw_text(
        "Scene Inspector",
        rect.x + 14.0,
        y,
        16.0,
        Color::from_rgba(200, 210, 230, 255),
    );
    y += 26.0;
    let visible_count = game
        .runtime_world
        .units
        .iter()
        .filter(|entity| entity.visible)
        .count();
    let locked_count = game
        .runtime_world
        .units
        .iter()
        .filter(|entity| entity.locked)
        .count();
    let script_count: usize = game
        .units
        .iter()
        .map(|entity| entity.scripts.len() + usize::from(entity.script.is_some()))
        .sum();
    draw_text(
        &format!(
            "{} entities | {} visible | {} locked | {} scripts",
            game.runtime_world.units.len(),
            visible_count,
            locked_count,
            script_count
        ),
        rect.x + 14.0,
        y,
        13.0,
        Color::from_rgba(160, 172, 196, 255),
    );
    y += 24.0;
    if button(rect.x + 14.0, y, 78.0, 24.0, "Validate", false) {
        if game.validate_project() {
            game.console.log("Proyecto y escena validos", "VALIDATE");
        } else {
            game.console
                .log("Validacion con errores o warnings", "WARNING");
        }
    }
    if button(rect.x + 100.0, y, 74.0, 24.0, "Save", false) {
        save_scene(game);
    }
    if button(rect.x + 182.0, y, 84.0, 24.0, "Manifest", false) {
        match game.build_manifest() {
            Ok(manifest) => game.console.log(
                format!(
                    "Manifest listo: {} assets, {} scenes",
                    manifest
                        .get("assets")
                        .and_then(Value::as_array)
                        .map(Vec::len)
                        .unwrap_or(0),
                    manifest
                        .get("scenes")
                        .and_then(Value::as_array)
                        .map(Vec::len)
                        .unwrap_or(0)
                ),
                "BUILD",
            ),
            Err(error) => game
                .console
                .log(format!("Manifest fallo: {error}"), "ERROR"),
        }
    }
    y += 34.0;
    let roots = ui_canvases_from_value(&game.ui_canvases);
    let el_total: usize = roots.iter().map(|c| c.elements.len()).sum();
    draw_text(
        &format!(
            "Canvases en escena: {} | elementos: {}",
            roots.len(),
            el_total
        ),
        rect.x + 14.0,
        y,
        14.0,
        Color::from_rgba(160, 172, 196, 255),
    );
    y += 28.0;
    if button(rect.x + 14.0, y, 120.0, 26.0, "Init HUD canvas", false) {
        game.ensure_default_ui_canvas_scene_data();
        game.console
            .log("Canvas HUD por defecto añadido a la escena", "UI");
    }
    if button(rect.x + 142.0, y, 118.0, 26.0, "Add scene label", false) {
        game.add_ui_canvas_scene_label("Nuevo label");
        game.console
            .log("Elemento Label añadido al primer canvas", "UI");
    }
    y += 36.0;
    draw_text(
        "Vista responsive (preview): escala = viewport / referencia canvas",
        rect.x + 14.0,
        y,
        13.0,
        Color::from_rgba(140, 150, 170, 255),
    );
    y += 22.0;
    if let Some(c) = roots.first() {
        let vw = rect.w - 28.0;
        let vh = (rect.h - y - 24.0).max(40.0);
        draw_rectangle(rect.x + 14.0, y, vw, vh, Color::from_rgba(16, 18, 24, 255));
        draw_rectangle_lines(
            rect.x + 14.0,
            y,
            vw,
            vh,
            1.0,
            Color::from_rgba(80, 92, 120, 255),
        );
        for el in &c.elements {
            let (ex, ey, ew, eh) = match el {
                UiCanvasElement::Panel { rect: r, .. }
                | UiCanvasElement::Button { rect: r, .. }
                | UiCanvasElement::Label { rect: r, .. }
                | UiCanvasElement::Image { rect: r, .. } => layout_element_pixels(c, r, vw, vh),
            };
            let px = rect.x + 14.0 + ex;
            let py = y + ey;
            match el {
                UiCanvasElement::Panel { color, .. } => {
                    draw_rectangle(
                        px,
                        py,
                        ew,
                        eh,
                        Color::from_rgba(color[0], color[1], color[2], color[3]),
                    );
                }
                UiCanvasElement::Button { label, .. } => {
                    draw_rectangle(px, py, ew, eh, Color::from_rgba(68, 126, 196, 255));
                    draw_text_fit(
                        label,
                        RectSpec {
                            x: px + 6.0,
                            y: py + 3.0,
                            w: (ew - 12.0).max(1.0),
                            h: (eh - 6.0).max(1.0),
                        },
                        14,
                        WHITE,
                        TextAlign::Center,
                    );
                }
                UiCanvasElement::Label {
                    text, font_size, ..
                } => {
                    draw_text_fit(
                        text,
                        RectSpec {
                            x: px,
                            y: py,
                            w: ew.max(1.0),
                            h: eh.max(*font_size + 2.0),
                        },
                        (*font_size).round().clamp(8.0, 64.0) as u16,
                        WHITE,
                        TextAlign::Left,
                    );
                }
                UiCanvasElement::Image { .. } => {
                    draw_rectangle_lines(px, py, ew, eh, 1.0, Color::from_rgba(200, 200, 220, 200));
                }
            }
        }
    }
}

fn draw_inspector(game: &mut Game, state: &mut EditorState, rect: RectSpec) {
    if rect.w <= 0.0 {
        return;
    }
    draw_surface(rect, false);
    draw_panel_header(rect, "Inspector", "selection details");
    let Some(id) = selected_id(game) else {
        draw_inspector_scene_ui_canvas(game, rect);
        return;
    };
    let Some(entity) = game.get_entity_by_id(id).cloned() else {
        return;
    };

    let mut y = rect.y + 58.0;
    draw_text(
        &ellipsize(
            &format!("{}  #{}  {}", entity.name, entity.id, entity.entity_type),
            36,
        ),
        rect.x + 14.0,
        y,
        16.0,
        ui_text(),
    );
    y += 22.0;
    draw_inspector_identity_summary(&entity, rect, y);
    y += 32.0;
    if button(rect.x + 14.0, y, 84.0, 24.0, "Duplicate", false) {
        duplicate_selected(game);
    }
    if button(rect.x + 104.0, y, 62.0, 24.0, "Delete", false) {
        delete_selected(game);
        return;
    }
    if button(rect.x + 172.0, y, 92.0, 24.0, "Center Cam", false) {
        game.camera.x = entity.x * game.grid.tile_size as f64 - 280.0;
        game.camera.y = entity.y * game.grid.tile_size as f64 - 180.0;
        game.camera.clamp_to_bounds();
    }
    if button(rect.x + 270.0, y, 68.0, 24.0, "Script", false) {
        if let Some(path) = first_entity_script_path(&entity) {
            open_project_file_in_editor(game, state, path);
        } else if let Ok(path) =
            game.create_luau_script_asset(&format!("{}_Controller", entity.name))
        {
            set_open_file_editor_state(state, &path);
        }
    }
    y += 34.0;
    if button(rect.x + 14.0, y, 96.0, 23.0, "Reset Xform", false) {
        let before = game.capture_editor_snapshot();
        if let Some(entity) = game.get_entity_by_id_mut(id) {
            InspectorEditor::reset_transform(entity);
            game.sync_world();
            game.mark_scene_dirty("Reset Transform");
            game.push_editor_command(
                "Reset Transform",
                EditorCommandKind::SceneOperation {
                    name: "Reset Transform".to_string(),
                },
                before,
            );
        }
    }
    if button(
        rect.x + 118.0,
        y,
        72.0,
        23.0,
        if entity.visible { "Visible" } else { "Hidden" },
        entity.visible,
    ) {
        edit_field(
            game,
            id,
            &InspectorField {
                target: "Identity".to_string(),
                key: "visible".to_string(),
                value: json!(entity.visible),
                value_type: "bool".to_string(),
                editable: true,
            },
            json!(!entity.visible),
        );
    }
    if button(
        rect.x + 198.0,
        y,
        66.0,
        23.0,
        if entity.locked { "Locked" } else { "Lock" },
        entity.locked,
    ) {
        edit_field(
            game,
            id,
            &InspectorField {
                target: "Identity".to_string(),
                key: "locked".to_string(),
                value: json!(entity.locked),
                value_type: "bool".to_string(),
                editable: true,
            },
            json!(!entity.locked),
        );
    }
    if button(
        rect.x + 272.0,
        y,
        66.0,
        23.0,
        if entity.enabled { "Enabled" } else { "Off" },
        entity.enabled,
    ) {
        edit_field(
            game,
            id,
            &InspectorField {
                target: "Identity".to_string(),
                key: "enabled".to_string(),
                value: json!(entity.enabled),
                value_type: "bool".to_string(),
                editable: true,
            },
            json!(!entity.enabled),
        );
    }
    y += 35.0;

    draw_text("Transform", rect.x + 14.0, y, 18.0, ui_text());
    y += 22.0;
    let fields = InspectorEditor::editable_fields(&entity);
    for key in [
        "x", "y", "rotation", "scale_x", "scale_y", "width", "height",
    ] {
        if let Some(field) = fields
            .iter()
            .find(|field| field.target == "Transform" && field.key == key)
            .cloned()
        {
            draw_inspector_field(game, state, id, &field, rect, y);
            y += 28.0;
        }
    }
    if let Some(field) = fields
        .iter()
        .find(|field| field.target == "Transform" && field.key == "name")
        .cloned()
    {
        draw_inspector_field(game, state, id, &field, rect, y);
        y += 28.0;
    }
    if button(rect.x + 14.0, y, 80.0, 23.0, "Tag", false) {
        cycle_identity(game, id, "tag");
    }
    draw_text(
        &ellipsize(&entity.tag, 18),
        rect.x + 102.0,
        y + 17.0,
        14.0,
        ui_text(),
    );
    if button(rect.x + 190.0, y, 80.0, 23.0, "Layer", false) {
        cycle_identity(game, id, "layer");
    }
    y += 34.0;

    draw_text("Prefab / Graph", rect.x + 14.0, y, 18.0, ui_text());
    y += 24.0;
    if button(rect.x + 14.0, y, 92.0, 24.0, "Save Prefab", false)
        && let Err(error) = game.save_selected_as_prefab()
    {
        game.console
            .log(format!("Error guardando prefab: {error}"), "ERROR");
    }
    if button(rect.x + 112.0, y, 74.0, 24.0, "Variant", false)
        && let Err(error) = game.create_selected_prefab_variant()
    {
        game.console
            .log(format!("Error creando variant: {error}"), "ERROR");
    }
    if button(rect.x + 194.0, y, 82.0, 24.0, "Attach VS", false) {
        game.attach_program_template_to_selected("LogAndMove");
    }
    if button(rect.x + 282.0, y, 72.0, 24.0, "Open VS", false) {
        if let Some(path) = entity_visual_graph_path(&entity) {
            open_project_file_in_editor(game, state, path);
        } else {
            game.attach_program_template_to_selected("LogAndMove");
        }
    }
    y += 34.0;
    if let Some(report) = game.analyze_selected_prefab() {
        draw_text(
            &ellipsize(
                &format!(
                    "Prefab: {} overrides | apply {}",
                    report.override_count,
                    if report.can_apply { "ready" } else { "blocked" }
                ),
                34,
            ),
            rect.x + 14.0,
            y,
            14.0,
            ui_accent(),
        );
        y += 22.0;
    }

    draw_text("Add / Remove Component", rect.x + 14.0, y, 18.0, ui_text());
    y += 24.0;
    let buttons = [
        ("Health", "Health"),
        ("Stats", "Stats"),
        ("Body", "Rigidbody2D"),
        ("Inventory", "Inventory"),
        ("Nav", "NavAgent"),
        ("AI", "AIController"),
        ("RTS", "Commandable"),
        ("Quest", "Quest"),
        ("Dialog", "Dialogue"),
        ("Tween", "Tween"),
        ("TileCol", "TilemapCollider"),
        ("Audio", "AudioSource"),
    ];
    let mut bx = rect.x + 14.0;
    for (label, component) in buttons {
        if button(
            bx,
            y,
            76.0,
            23.0,
            label,
            entity.get_component(component).is_some(),
        ) {
            add_component(game, id, component);
        }
        bx += 82.0;
        if bx + 76.0 > rect.x + rect.w - 12.0 {
            bx = rect.x + 14.0;
            y += 28.0;
        }
    }
    y += 30.0;
    if button(rect.x + 14.0, y, 96.0, 24.0, "Player Kit", false)
        && game.apply_component_preset(id, "TopDown Player")
    {
        game.console.log("Preset TopDown Player aplicado", "EDITOR");
    }
    if button(rect.x + 118.0, y, 82.0, 24.0, "Enemy Kit", false)
        && game.apply_component_preset(id, "Enemy AI")
    {
        game.console.log("Preset Enemy AI aplicado", "EDITOR");
    }
    if button(rect.x + 208.0, y, 70.0, 24.0, "Visual", false)
        && let Some(name) = game.add_visual_script_template(id, "LogAndMove")
    {
        game.console
            .log(format!("VisualScript {name} agregado"), "EDITOR");
    }
    if button(rect.x + 284.0, y, 72.0, 24.0, "Physics", false) {
        add_component(game, id, "Rigidbody2D");
        add_component(game, id, "Collider2D");
    }
    y += 38.0;

    if let Some(report) = &game.physics_system.rapier_report {
        draw_text(
            &ellipsize(&report.status_line(), 44),
            rect.x + 14.0,
            y,
            13.0,
            Color::from_rgba(130, 210, 235, 255),
        );
        y += 20.0;
    }

    draw_text("Components", rect.x + 14.0, y, 18.0, ui_text());
    y += 24.0;
    for component in &entity.components {
        if y > rect.y + rect.h - 38.0 {
            break;
        }
        let category = advanced_component_category(&component.component_type).unwrap_or("Core");
        let header = RectSpec {
            x: rect.x + 12.0,
            y: y - 16.0,
            w: rect.w - 24.0,
            h: 23.0,
        };
        draw_rect(header, Color::from_rgba(26, 35, 50, 255));
        draw_rectangle(header.x, header.y, 3.0, header.h, ui_accent_2());
        draw_text(
            &ellipsize(&format!("{} [{}]", component.component_type, category), 28),
            rect.x + 18.0,
            y,
            15.0,
            ui_accent(),
        );
        if button(rect.x + rect.w - 52.0, y - 16.0, 38.0, 20.0, "-", false)
            && let Err(error) = game.remove_component_from_entity(id, &component.component_type)
        {
            game.console.log(error, "WARNING");
        }
        y += 27.0;
        let enabled_field = InspectorField {
            target: component.component_type.clone(),
            key: "enabled".to_string(),
            value: json!(component.enabled),
            value_type: "bool".to_string(),
            editable: true,
        };
        draw_inspector_field(game, state, id, &enabled_field, rect, y);
        y += 27.0;
        for (key, value) in &component.data {
            if y > rect.y + rect.h - 30.0 {
                break;
            }
            let field = InspectorField {
                target: component.component_type.clone(),
                key: key.clone(),
                value: value.clone(),
                value_type: inspector_value_type(value),
                editable: !matches!(value, Value::Array(_) | Value::Object(_)),
            };
            draw_inspector_field(game, state, id, &field, rect, y);
            y += 27.0;
        }
    }
}

fn draw_inspector_field(
    game: &mut Game,
    state: &mut EditorState,
    entity_id: u64,
    field: &InspectorField,
    rect: RectSpec,
    y: f32,
) {
    let row = RectSpec {
        x: rect.x + 12.0,
        y: y - 17.0,
        w: rect.w - 24.0,
        h: 24.0,
    };
    draw_rect(
        row,
        if contains_mouse(row) {
            Color::from_rgba(42, 49, 63, 255)
        } else {
            Color::from_rgba(30, 35, 46, 255)
        },
    );
    draw_text(
        &ellipsize(&field.key, 14),
        row.x + 7.0,
        y,
        14.0,
        Color::from_rgba(180, 190, 210, 255),
    );

    match &field.value {
        Value::Bool(value) => {
            let active = *value;
            if button(
                row.x + row.w - 66.0,
                row.y + 2.0,
                56.0,
                20.0,
                if active { "On" } else { "Off" },
                active,
            ) {
                edit_field(game, entity_id, field, json!(!active));
            }
        }
        Value::Number(number) => {
            let current = number.as_f64().unwrap_or(0.0);
            draw_text(
                &format!("{current:.2}"),
                row.x + 116.0,
                y,
                14.0,
                Color::from_rgba(232, 236, 246, 255),
            );
            let step = numeric_step(&field.key);
            if button(row.x + row.w - 64.0, row.y + 2.0, 26.0, 20.0, "-", false) {
                edit_field(game, entity_id, field, json!(current - step));
            }
            if button(row.x + row.w - 34.0, row.y + 2.0, 26.0, 20.0, "+", false) {
                edit_field(game, entity_id, field, json!(current + step));
            }
        }
        Value::String(text) => {
            let active = state.active_text_field.as_ref().is_some_and(|edit| {
                edit.entity_id == entity_id && edit.target == field.target && edit.key == field.key
            });
            let box_rect = RectSpec {
                x: row.x + 102.0,
                y: row.y + 3.0,
                w: row.w - 112.0,
                h: 18.0,
            };
            draw_rect(
                box_rect,
                if active {
                    Color::from_rgba(70, 94, 128, 255)
                } else {
                    Color::from_rgba(22, 26, 34, 255)
                },
            );
            let shown = state
                .active_text_field
                .as_ref()
                .filter(|edit| {
                    edit.entity_id == entity_id
                        && edit.target == field.target
                        && edit.key == field.key
                })
                .map(|edit| edit.buffer.as_str())
                .unwrap_or(text);
            draw_text(
                &ellipsize(shown, 24),
                box_rect.x + 6.0,
                y,
                14.0,
                Color::from_rgba(232, 236, 246, 255),
            );
            if contains_mouse(box_rect) && is_mouse_button_pressed(MouseButton::Left) {
                state.active_text_field = Some(TextEditState {
                    entity_id,
                    target: field.target.clone(),
                    key: field.key.clone(),
                    buffer: text.clone(),
                });
            }
        }
        Value::Array(items) => {
            draw_text(
                &format!("Array[{}]", items.len()),
                row.x + 116.0,
                y,
                14.0,
                Color::from_rgba(160, 170, 190, 255),
            );
        }
        Value::Object(map) => {
            draw_text(
                &format!("Object[{}]", map.len()),
                row.x + 116.0,
                y,
                14.0,
                Color::from_rgba(160, 170, 190, 255),
            );
        }
        Value::Null => {
            draw_text(
                "null",
                row.x + 116.0,
                y,
                14.0,
                Color::from_rgba(160, 170, 190, 255),
            );
        }
    }
}

fn edit_field(game: &mut Game, entity_id: u64, field: &InspectorField, value: Value) {
    if let Err(error) = game.edit_inspector_value(entity_id, &field.target, &field.key, value) {
        game.console.log(format!("Inspector: {error}"), "WARNING");
    }
}

fn cycle_identity(game: &mut Game, entity_id: u64, key: &str) {
    let Some(entity) = game.get_entity_by_id(entity_id) else {
        return;
    };
    let (items, current) = if key == "tag" {
        (&game.tags_layers_manager.tags, entity.tag.as_str())
    } else {
        (&game.tags_layers_manager.layers, entity.layer.as_str())
    };
    if items.is_empty() {
        return;
    }
    let index = items.iter().position(|item| item == current).unwrap_or(0);
    let next = items[(index + 1) % items.len()].clone();
    if let Err(error) = game.edit_inspector_value(entity_id, "Identity", key, json!(next)) {
        game.console.log(error, "WARNING");
    }
}

fn numeric_step(key: &str) -> f64 {
    match key {
        "x" | "y" | "width" | "height" => 0.25,
        "rotation" => 5.0,
        "scale_x" | "scale_y" => 0.1,
        key if key.contains("health") || key.contains("damage") || key.contains("amount") => 5.0,
        key if key.contains("speed") || key.contains("radius") || key.contains("time") => 0.25,
        _ => 1.0,
    }
}

fn inspector_value_type(value: &Value) -> String {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
    .to_string()
}

fn draw_inspector_identity_summary(entity: &GameObject, rect: RectSpec, y: f32) {
    let card = RectSpec {
        x: rect.x + 12.0,
        y: y - 18.0,
        w: rect.w - 24.0,
        h: 26.0,
    };
    draw_rect(card, Color::from_rgba(18, 23, 32, 255));
    draw_rectangle_lines(
        card.x,
        card.y,
        card.w,
        card.h,
        1.0,
        Color::from_rgba(58, 72, 96, 255),
    );
    let scripts = entity.scripts.len() + usize::from(entity.script.is_some());
    draw_text(
        &ellipsize(
            &format!(
                "{} comps | {} scripts | layer {} | {}",
                entity.components.len(),
                scripts,
                entity.layer,
                if entity.enabled {
                    "enabled"
                } else {
                    "disabled"
                }
            ),
            48,
        ),
        card.x + 8.0,
        y,
        13.0,
        Color::from_rgba(174, 194, 224, 255),
    );
}

fn first_entity_script_path(entity: &GameObject) -> Option<String> {
    entity
        .scripts
        .iter()
        .filter_map(|script| script.get("path").and_then(Value::as_str))
        .next()
        .or(entity.script.as_deref())
        .map(normalize_script_path)
}

fn entity_visual_graph_path(entity: &GameObject) -> Option<String> {
    entity
        .get_component("VisualScript")
        .and_then(|component| {
            component
                .get("graph_path")
                .and_then(Value::as_str)
                .or_else(|| component.get("graph_name").and_then(Value::as_str))
        })
        .map(|raw| {
            if raw.starts_with("scripts/") {
                raw.to_string()
            } else if raw.ends_with(".mfgraph") {
                format!("scripts/visual_graphs/{raw}")
            } else {
                format!("scripts/visual_graphs/{raw}.mfgraph")
            }
        })
}

fn normalize_script_path(raw: &str) -> String {
    if raw.starts_with("scripts/") {
        raw.to_string()
    } else {
        format!("scripts/{raw}")
    }
}

fn draw_scene(game: &Game, state: &EditorState, viewport: Viewport) {
    draw_gradient_rect(
        viewport.rect,
        Color::from_rgba(19, 25, 36, 255),
        Color::from_rgba(11, 15, 23, 255),
    );
    draw_tiles(game, viewport);
    if state.show_grid {
        draw_grid(game, viewport);
    }
    if state.show_camera_frame {
        draw_scene_camera_frame(viewport);
    }

    for entity in &game.runtime_world.units {
        if !entity.enabled || !entity.visible {
            continue;
        }
        draw_entity(entity, viewport, game.selected_units.contains(&entity.id));
    }

    draw_scene_gizmos(game, state, viewport);
    draw_paint_cursor(game, state, viewport);
    draw_drag_preview(state);
    draw_scene_ui_canvases(game, Some(state), viewport);
    draw_ui_elements(game);
    draw_ui_selection_gizmos(game);
    draw_scene_hud(game, state, viewport);
    draw_rectangle_lines(
        viewport.rect.x,
        viewport.rect.y,
        viewport.rect.w,
        viewport.rect.h,
        1.0,
        ui_line_soft(),
    );
}

fn draw_world_player(game: &Game, viewport: Viewport) {
    draw_gradient_rect(
        viewport.rect,
        Color::from_rgba(19, 25, 36, 255),
        Color::from_rgba(11, 15, 23, 255),
    );
    draw_tiles(game, viewport);
    for entity in &game.runtime_world.units {
        if !entity.enabled || !entity.visible {
            continue;
        }
        draw_entity(entity, viewport, false);
    }
    draw_scene_ui_canvases(game, None, viewport);
    draw_ui_elements(game);
}

fn draw_scene_gizmos(game: &Game, state: &EditorState, viewport: Viewport) {
    if state.show_collisions || state.toolbar.active_tool() == EditorTool::Collision {
        for entity in game
            .units
            .iter()
            .filter(|entity| entity.enabled && entity.visible)
        {
            let selected = game.selected_units.contains(&entity.id);
            let path = EditorSpatialTools2D::collision_path(
                entity,
                VectorStyle2D {
                    fill: selected.then_some([52, 211, 153, 28]),
                    stroke: Some(if selected {
                        [87, 244, 177, 255]
                    } else {
                        [52, 211, 153, 135]
                    }),
                    stroke_width: if selected { 0.065 } else { 0.035 },
                    tolerance: 0.02,
                    ..VectorStyle2D::default()
                },
            );
            if let Ok(geometry) = path.tessellate() {
                draw_vector_geometry_world(&geometry, viewport);
            }
            if selected && state.toolbar.active_tool() == EditorTool::Collision {
                for point in EditorSpatialTools2D::collision_points(entity) {
                    let world = entity_local_to_world(entity, point);
                    let screen = world_to_screen(viewport, world.0 as f32, world.1 as f32);
                    draw_circle(screen.0, screen.1, 5.0, Color::from_rgba(20, 28, 38, 255));
                    draw_circle_lines(
                        screen.0,
                        screen.1,
                        5.5,
                        2.0,
                        Color::from_rgba(111, 255, 196, 255),
                    );
                }
            }
        }
    }

    for id in &game.selected_units {
        let Some(entity) = game.get_entity_by_id(*id) else {
            continue;
        };
        let bounds = SceneViewTools::bounding_box(entity);
        let (min_x, min_y) = world_to_screen(viewport, bounds.min_x as f32, bounds.min_y as f32);
        let (max_x, max_y) = world_to_screen(viewport, bounds.max_x as f32, bounds.max_y as f32);
        let w = max_x - min_x;
        let h = max_y - min_y;
        let selection = VectorPath2D::rounded_rectangle(
            VectorPoint2D::new(min_x, min_y),
            VectorPoint2D::new(max_x, max_y),
            4.0,
            VectorStyle2D {
                fill: Some([70, 160, 255, 18]),
                stroke: Some([94, 196, 255, 255]),
                stroke_width: 2.0,
                tolerance: 0.1,
                ..VectorStyle2D::default()
            },
        );
        if let Ok(geometry) = selection.tessellate() {
            draw_vector_geometry_screen(&geometry);
        }
        let (cx, cy) = world_to_screen(viewport, entity.x as f32, entity.y as f32);
        if matches!(
            state.toolbar.active_tool(),
            EditorTool::Move | EditorTool::Rotate | EditorTool::Scale | EditorTool::Pivot
        ) {
            for path in translation_gizmo(VectorPoint2D::new(cx, cy), 42.0) {
                if let Ok(geometry) = path.tessellate() {
                    draw_vector_geometry_screen(&geometry);
                }
            }
            draw_circle(cx, cy, 5.0, Color::from_rgba(245, 245, 255, 255));
        }
        if state.toolbar.active_tool() == EditorTool::Rotate {
            let ring = VectorPath2D::circle(
                VectorPoint2D::new(cx, cy),
                w.max(h).max(28.0) * 0.65,
                VectorStyle2D {
                    fill: None,
                    stroke: Some([255, 212, 120, 255]),
                    stroke_width: 2.0,
                    ..VectorStyle2D::default()
                },
            );
            if let Ok(geometry) = ring.tessellate() {
                draw_vector_geometry_screen(&geometry);
            }
        }
        if state.toolbar.active_tool() == EditorTool::Scale {
            draw_rectangle(
                max_x - 6.0,
                max_y - 6.0,
                12.0,
                12.0,
                Color::from_rgba(185, 145, 255, 255),
            );
        }
        if state.toolbar.active_tool() == EditorTool::Pivot {
            let pivot = EditorSpatialTools2D::pivot(entity);
            let local = (
                (pivot.0 - 0.5) * entity.width,
                (pivot.1 - 0.5) * entity.height,
            );
            let world = entity_local_to_world(entity, local);
            let (px, py) = world_to_screen(viewport, world.0 as f32, world.1 as f32);
            draw_line(px - 9.0, py, px + 9.0, py, 2.0, WHITE);
            draw_line(px, py - 9.0, px, py + 9.0, 2.0, WHITE);
            draw_circle_lines(px, py, 6.0, 2.0, Color::from_rgba(255, 210, 96, 255));
        }
    }

    for guide in &state.snap_guides {
        match guide.axis {
            crate::engine::editor_spatial_tools_2d::SnapAxis2D::X => {
                let x = world_to_screen(viewport, guide.value as f32, 0.0).0;
                draw_line(
                    x,
                    viewport.rect.y,
                    x,
                    viewport.rect.y + viewport.rect.h,
                    1.5,
                    Color::from_rgba(255, 204, 92, 210),
                );
            }
            crate::engine::editor_spatial_tools_2d::SnapAxis2D::Y => {
                let y = world_to_screen(viewport, 0.0, guide.value as f32).1;
                draw_line(
                    viewport.rect.x,
                    y,
                    viewport.rect.x + viewport.rect.w,
                    y,
                    1.5,
                    Color::from_rgba(255, 204, 92, 210),
                );
            }
        }
    }
}

fn draw_scene_camera_frame(viewport: Viewport) {
    let camera = CameraFrame2D::default();
    let frame = camera.fit_inside((
        viewport.rect.x + 18.0,
        viewport.rect.y + 18.0,
        viewport.rect.w - 36.0,
        viewport.rect.h - 36.0,
    ));
    let safe = camera.safe_rect(frame);
    for (rect, color, width) in [
        (frame, [255, 255, 255, 175], 1.5),
        (safe, [255, 204, 92, 125], 1.0),
    ] {
        let path = VectorPath2D::rounded_rectangle(
            VectorPoint2D::new(rect.0, rect.1),
            VectorPoint2D::new(rect.0 + rect.2, rect.1 + rect.3),
            5.0,
            VectorStyle2D {
                fill: None,
                stroke: Some(color),
                stroke_width: width,
                ..VectorStyle2D::default()
            },
        );
        if let Ok(geometry) = path.tessellate() {
            draw_vector_geometry_screen(&geometry);
        }
    }
    draw_text(
        "CAMERA 16:9",
        frame.0 + 8.0,
        frame.1 + 18.0,
        13.0,
        Color::from_rgba(235, 240, 250, 190),
    );
}

fn draw_vector_geometry_screen(geometry: &VectorGeometry2D) {
    if let Some(fill) = &geometry.fill {
        draw_vector_mesh(fill, |point| point);
    }
    if let Some(stroke) = &geometry.stroke {
        draw_vector_mesh(stroke, |point| point);
    }
}

fn draw_vector_geometry_world(geometry: &VectorGeometry2D, viewport: Viewport) {
    if let Some(fill) = &geometry.fill {
        draw_vector_mesh(fill, |point| {
            let screen = world_to_screen(viewport, point[0], point[1]);
            [screen.0, screen.1]
        });
    }
    if let Some(stroke) = &geometry.stroke {
        draw_vector_mesh(stroke, |point| {
            let screen = world_to_screen(viewport, point[0], point[1]);
            [screen.0, screen.1]
        });
    }
}

fn draw_vector_mesh(mesh: &VectorMesh2D, transform: impl Fn([f32; 2]) -> [f32; 2]) {
    let color = Color::from_rgba(mesh.color[0], mesh.color[1], mesh.color[2], mesh.color[3]);
    for triangle in mesh.indices.chunks_exact(3) {
        let a = transform(mesh.vertices[triangle[0] as usize]);
        let b = transform(mesh.vertices[triangle[1] as usize]);
        let c = transform(mesh.vertices[triangle[2] as usize]);
        draw_triangle(
            Vec2::new(a[0], a[1]),
            Vec2::new(b[0], b[1]),
            Vec2::new(c[0], c[1]),
            color,
        );
    }
}

fn entity_local_to_world(entity: &GameObject, local: (f64, f64)) -> (f64, f64) {
    let x = local.0 * entity.scale_x;
    let y = local.1 * entity.scale_y;
    let radians = entity.rotation.to_radians();
    (
        entity.x + x * radians.cos() - y * radians.sin(),
        entity.y + x * radians.sin() + y * radians.cos(),
    )
}

fn draw_drag_preview(state: &EditorState) {
    let Some(payload) = &state.drag_payload else {
        return;
    };
    let (mx, my) = mouse_position();
    draw_rectangle(
        mx + 14.0,
        my + 14.0,
        190.0,
        38.0,
        Color::from_rgba(35, 42, 56, 205),
    );
    draw_rectangle_lines(
        mx + 14.0,
        my + 14.0,
        190.0,
        38.0,
        1.0,
        Color::from_rgba(130, 170, 220, 255),
    );
    draw_text(
        &ellipsize(&payload.drop_hint(), 28),
        mx + 24.0,
        my + 38.0,
        16.0,
        Color::from_rgba(235, 240, 250, 255),
    );
}

fn draw_tiles(game: &Game, viewport: Viewport) {
    let tile_px = viewport.tile * viewport.zoom;
    if tile_px < 2.0 {
        return;
    }
    let (start_x, end_x, start_y, end_y) = visible_tile_bounds(game, viewport);
    for (layer_index, layer) in game.tilemap_layers.layers.iter().enumerate() {
        if !layer.visible {
            continue;
        }
        for y in start_y..end_y {
            for x in start_x..end_x {
                let value = layer.get(x, y);
                if value == 0 {
                    continue;
                }
                let (sx, sy) = world_to_screen(viewport, x as f32, y as f32);
                let mut color = tile_color(value, layer_index);
                if layer_index != game.tilemap_layers.active_layer {
                    color.a *= 0.58;
                }
                draw_rectangle(sx, sy, tile_px.ceil(), tile_px.ceil(), color);
            }
        }
    }
}

fn draw_grid(game: &Game, viewport: Viewport) {
    let tile_px = viewport.tile * viewport.zoom;
    if tile_px < 4.0 {
        return;
    }
    let (start_x, end_x, start_y, end_y) = visible_tile_bounds(game, viewport);
    let grid_color = Color::from_rgba(49, 56, 70, 255);
    for x in start_x..=end_x {
        let sx = world_to_screen(viewport, x as f32, 0.0).0;
        draw_line(
            sx,
            viewport.rect.y,
            sx,
            viewport.rect.y + viewport.rect.h,
            1.0,
            grid_color,
        );
    }
    for y in start_y..=end_y {
        let sy = world_to_screen(viewport, 0.0, y as f32).1;
        draw_line(
            viewport.rect.x,
            sy,
            viewport.rect.x + viewport.rect.w,
            sy,
            1.0,
            grid_color,
        );
    }
}

fn visible_tile_bounds(game: &Game, viewport: Viewport) -> (usize, usize, usize, usize) {
    let tile_px = viewport.tile * viewport.zoom;
    let world_left = viewport.camera_x / viewport.tile;
    let world_top = viewport.camera_y / viewport.tile;
    let world_right = world_left + viewport.rect.w / tile_px.max(1.0);
    let world_bottom = world_top + viewport.rect.h / tile_px.max(1.0);
    let start_x = world_left.floor().max(0.0) as usize;
    let end_x = world_right.ceil().min(game.grid.width as f32) as usize;
    let start_y = world_top.floor().max(0.0) as usize;
    let end_y = world_bottom.ceil().min(game.grid.height as f32) as usize;
    (start_x, end_x, start_y, end_y)
}

fn draw_entity(entity: &GameObject, viewport: Viewport, selected: bool) {
    let (sx, sy) = world_to_screen(viewport, entity.x as f32, entity.y as f32);
    let size = (entity.width.max(entity.height).max(0.8) as f32 * viewport.tile * viewport.zoom)
        .clamp(8.0, 82.0);
    let color = entity_color(entity);
    if entity.entity_type == "Unit" {
        draw_circle(sx, sy, size * 0.38, color);
        draw_circle_lines(sx, sy, size * 0.38, 2.0, Color::from_rgba(10, 15, 22, 255));
    } else {
        draw_rectangle(sx - size * 0.5, sy - size * 0.5, size, size, color);
        draw_rectangle_lines(
            sx - size * 0.5,
            sy - size * 0.5,
            size,
            size,
            2.0,
            Color::from_rgba(10, 15, 22, 255),
        );
    }
    if selected {
        draw_circle_lines(
            sx,
            sy,
            size * 0.55,
            3.0,
            Color::from_rgba(88, 196, 255, 255),
        );
    }
    draw_health_bar(entity, sx - size * 0.5, sy - size * 0.75, size);
    draw_component_badges(entity, sx - size * 0.5, sy + size * 0.55);
    draw_text(
        &ellipsize(&entity.name, 18),
        sx + 10.0,
        sy - 10.0,
        14.0,
        Color::from_rgba(230, 235, 245, 255),
    );

    let mut last = (entity.x as f32, entity.y as f32);
    for point in &entity.path {
        let next = (point.0 as f32, point.1 as f32);
        let (a_x, a_y) = world_to_screen(viewport, last.0, last.1);
        let (b_x, b_y) = world_to_screen(viewport, next.0, next.1);
        draw_line(a_x, a_y, b_x, b_y, 2.0, Color::from_rgba(255, 210, 80, 255));
        last = next;
    }
}

fn draw_health_bar(entity: &GameObject, x: f32, y: f32, w: f32) {
    let Some(health) = entity.get_component("Health") else {
        return;
    };
    let max_health = health.get_f64("max_health", 100.0).max(1.0);
    let value = (health.get_f64("health", max_health) / max_health).clamp(0.0, 1.0) as f32;
    draw_rectangle(x, y, w, 5.0, Color::from_rgba(35, 35, 42, 230));
    draw_rectangle(x, y, w * value, 5.0, Color::from_rgba(105, 230, 145, 240));
}

fn draw_component_badges(entity: &GameObject, x: f32, y: f32) {
    let mut bx = x;
    let badges = [
        ("AIController", "AI", Color::from_rgba(255, 160, 110, 230)),
        ("NavAgent", "NAV", Color::from_rgba(105, 190, 255, 230)),
        ("Inventory", "INV", Color::from_rgba(178, 142, 255, 230)),
        ("ResourceNode", "RES", Color::from_rgba(245, 195, 78, 230)),
        ("VisualScript", "VS", Color::from_rgba(110, 230, 205, 230)),
    ];
    for (component, label, color) in badges {
        if entity.get_component(component).is_none() {
            continue;
        }
        draw_rectangle(bx, y, 28.0, 13.0, color);
        draw_text(
            label,
            bx + 3.0,
            y + 10.0,
            10.0,
            Color::from_rgba(18, 20, 26, 255),
        );
        bx += 31.0;
    }
}

fn draw_paint_cursor(game: &Game, state: &EditorState, viewport: Viewport) {
    if state.toolbar.active_tool() != EditorTool::Paint {
        return;
    }
    let (mx, my) = mouse_position();
    if !contains(viewport.rect, mx, my) {
        return;
    }
    let world = screen_to_world(viewport, mx, my);
    let x = world.0.floor();
    let y = world.1.floor();
    let size = game.brush_size.max(1) as f32;
    let (sx, sy) = world_to_screen(viewport, x, y);
    let tile = viewport.tile * viewport.zoom;
    let color = match state.tile_brush_mode {
        TileBrushMode::Pencil => Color::from_rgba(255, 235, 120, 255),
        TileBrushMode::Eraser => Color::from_rgba(255, 130, 130, 255),
        TileBrushMode::Fill => Color::from_rgba(120, 210, 255, 255),
        TileBrushMode::Rectangle => Color::from_rgba(185, 145, 255, 255),
        TileBrushMode::Line => Color::from_rgba(255, 205, 90, 255),
        TileBrushMode::Collision => Color::from_rgba(255, 170, 90, 255),
    };
    draw_rectangle_lines(sx, sy, tile * size, tile * size, 2.0, color);
    if matches!(
        state.tile_brush_mode,
        TileBrushMode::Rectangle | TileBrushMode::Line
    ) && let Some(start) = state.paint_start
        && let Some(end) = world_to_cell(game, world)
    {
        let min_x = start.0.min(end.0) as f32;
        let min_y = start.1.min(end.1) as f32;
        let max_x = start.0.max(end.0) as f32 + 1.0;
        let max_y = start.1.max(end.1) as f32 + 1.0;
        let (rx, ry) = world_to_screen(viewport, min_x, min_y);
        let (rw, rh) = world_to_screen(viewport, max_x, max_y);
        if state.tile_brush_mode == TileBrushMode::Line {
            let (start_x, start_y) =
                world_to_screen(viewport, start.0 as f32 + 0.5, start.1 as f32 + 0.5);
            let (end_x, end_y) = world_to_screen(viewport, end.0 as f32 + 0.5, end.1 as f32 + 0.5);
            draw_line(start_x, start_y, end_x, end_y, 3.0, color);
        } else {
            draw_rectangle_lines(rx, ry, rw - rx, rh - ry, 2.0, color);
        }
    }
}

fn draw_scene_ui_canvases(game: &Game, state: Option<&EditorState>, viewport: Viewport) {
    let roots = ui_canvases_from_value(&game.ui_canvases);
    if roots.is_empty() {
        return;
    }
    for root in &roots {
        for element in &root.elements {
            let (x, y, w, h) =
                layout_element_pixels(root, element.rect(), viewport.rect.w, viewport.rect.h);
            draw_ui_canvas_element(
                element,
                viewport.rect.x + x,
                viewport.rect.y + y,
                w,
                h,
                0.88,
            );
        }
    }

    let Some(state) = state else {
        return;
    };
    let Some(canvas_id) = state.selected_ui_canvas_id.as_deref() else {
        return;
    };
    let Some(element_id) = state.selected_ui_element_id.as_deref() else {
        return;
    };
    let Some(root) = roots.iter().find(|root| root.id == canvas_id) else {
        return;
    };
    let Some(gizmo) = root.gizmo_for_element(element_id, viewport.rect.w, viewport.rect.h) else {
        return;
    };

    let gx = viewport.rect.x + gizmo.x;
    let gy = viewport.rect.y + gizmo.y;
    draw_rectangle_lines(
        gx - 2.0,
        gy - 2.0,
        gizmo.width + 4.0,
        gizmo.height + 4.0,
        2.0,
        Color::from_rgba(255, 214, 92, 255),
    );
    for handle in gizmo.handles {
        let hx = viewport.rect.x + handle.x;
        let hy = viewport.rect.y + handle.y;
        draw_rectangle(
            hx,
            hy,
            handle.width,
            handle.height,
            Color::from_rgba(18, 22, 30, 245),
        );
        draw_rectangle_lines(
            hx,
            hy,
            handle.width,
            handle.height,
            1.5,
            Color::from_rgba(255, 214, 92, 255),
        );
    }
    draw_rectangle(
        gx,
        (gy - 22.0).max(viewport.rect.y + 4.0),
        172.0,
        18.0,
        Color::from_rgba(18, 22, 30, 225),
    );
    draw_text(
        &ellipsize(&format!("{} / {}", root.name, gizmo.element_id), 22),
        gx + 6.0,
        (gy - 8.0).max(viewport.rect.y + 18.0),
        12.0,
        Color::from_rgba(245, 248, 255, 255),
    );
}

fn draw_ui_canvas_element(element: &UiCanvasElement, x: f32, y: f32, w: f32, h: f32, alpha: f32) {
    let a = (255.0 * alpha.clamp(0.0, 1.0)) as u8;
    match element {
        UiCanvasElement::Panel { color, .. } => {
            draw_rectangle(
                x,
                y,
                w,
                h,
                Color::from_rgba(
                    color[0],
                    color[1],
                    color[2],
                    ((color[3] as f32) * alpha) as u8,
                ),
            );
            draw_rectangle_lines(x, y, w, h, 1.0, Color::from_rgba(120, 138, 168, a));
        }
        UiCanvasElement::Button { label, .. } => {
            draw_rectangle(
                x,
                y,
                w,
                h,
                Color::from_rgba(58, 116, 188, (230.0 * alpha) as u8),
            );
            draw_rectangle_lines(x, y, w, h, 1.0, Color::from_rgba(190, 220, 255, a));
            draw_text_fit(
                label,
                RectSpec {
                    x: x + 8.0,
                    y: y + 4.0,
                    w: (w - 16.0).max(1.0),
                    h: (h - 8.0).max(1.0),
                },
                14,
                Color::from_rgba(245, 248, 255, a),
                TextAlign::Center,
            );
        }
        UiCanvasElement::Label {
            text, font_size, ..
        } => {
            draw_text_fit(
                text,
                RectSpec {
                    x,
                    y,
                    w: w.max(1.0),
                    h: h.max(*font_size + 2.0),
                },
                (*font_size).round().clamp(8.0, 64.0) as u16,
                Color::from_rgba(245, 248, 255, a),
                TextAlign::Left,
            );
        }
        UiCanvasElement::Image { sprite_path, .. } => {
            draw_rectangle(
                x,
                y,
                w,
                h,
                Color::from_rgba(36, 42, 56, (160.0 * alpha) as u8),
            );
            draw_rectangle_lines(x, y, w, h, 1.0, Color::from_rgba(190, 205, 230, a));
            draw_text_fit(
                sprite_path,
                RectSpec {
                    x: x + 6.0,
                    y: y + 4.0,
                    w: (w - 12.0).max(1.0),
                    h: (h - 8.0).max(1.0),
                },
                12,
                Color::from_rgba(205, 216, 235, a),
                TextAlign::Center,
            );
        }
    }
}

fn draw_ui_elements(game: &Game) {
    for entity in &game.runtime_world.units {
        let Some(ui) = entity.get_component("UIElement") else {
            continue;
        };
        let x = ui.get_f64("x", 0.0) as f32;
        let y = ui.get_f64("y", 0.0) as f32;
        let w = ui.get_f64("width", 160.0) as f32;
        let h = ui.get_f64("height", 36.0) as f32;
        let text = ui.get_string("text", "Label");
        let kind = ui.get_string("element_type", "Label");
        let base = if kind == "Button" {
            Color::from_rgba(74, 122, 194, 230)
        } else {
            Color::from_rgba(238, 241, 247, 230)
        };
        draw_rectangle(x, y, w, h, base);
        draw_rectangle_lines(x, y, w, h, 1.0, Color::from_rgba(40, 44, 54, 255));
        draw_text_fit(
            &text,
            RectSpec {
                x: x + 8.0,
                y: y + 4.0,
                w: (w - 16.0).max(1.0),
                h: (h - 8.0).max(1.0),
            },
            18,
            Color::from_rgba(25, 28, 35, 255),
            if kind == "Button" {
                TextAlign::Center
            } else {
                TextAlign::Left
            },
        );
    }
}

fn draw_ui_selection_gizmos(game: &Game) {
    for id in &game.selected_units {
        let Some(entity) = game.get_entity_by_id(*id) else {
            continue;
        };
        let Some(ui) = entity.get_component("UIElement") else {
            continue;
        };
        let x = ui.get_f64("x", 0.0) as f32;
        let y = ui.get_f64("y", 0.0) as f32;
        let w = ui.get_f64("width", 160.0) as f32;
        let h = ui.get_f64("height", 36.0) as f32;
        draw_rectangle_lines(
            x - 2.0,
            y - 2.0,
            w + 4.0,
            h + 4.0,
            2.0,
            Color::from_rgba(88, 196, 255, 255),
        );
        draw_circle(x, y, 4.0, Color::from_rgba(255, 212, 120, 255));
    }
}

fn draw_scene_hud(game: &Game, state: &EditorState, viewport: Viewport) {
    let layer = &game.tilemap_layers.layers[game.tilemap_layers.active_layer].name;
    let play_bit = if game.mode == "PLAY" {
        format!(" | LIVE {}f", game.play_mode_manager.frame_count)
    } else {
        String::new()
    };
    let text = format!(
        "{} | {} selected | Tool {} | Layer {} | Grid {} | Smart {} | Zoom {:.0}% | Scene {}{}",
        game.editor_workspace.active_mode.label(),
        game.selected_units.len(),
        state.toolbar.active_tool().label(),
        layer,
        if state.snap_to_grid { "on" } else { "off" },
        if state.smart_snap { "on" } else { "off" },
        game.camera.zoom * 100.0,
        if game.scene_dirty { "dirty" } else { "clean" },
        play_bit
    );
    let w = (measure_text(&text, None, 16, 1.0).width + 22.0).min(viewport.rect.w - 24.0);
    draw_rectangle(
        viewport.rect.x + 12.0,
        viewport.rect.y + 12.0,
        w,
        30.0,
        Color::from_rgba(14, 17, 22, 205),
    );
    draw_text(
        &ellipsize(&text, 110),
        viewport.rect.x + 23.0,
        viewport.rect.y + 32.0,
        16.0,
        Color::from_rgba(226, 232, 244, 255),
    );
    draw_text(
        "Shift/Cmd click multi-select | F frame | Space/MMB pan | Cmd+wheel smooth zoom | 1-7 tools",
        viewport.rect.x + 14.0,
        viewport.rect.y + viewport.rect.h - 14.0,
        12.0,
        Color::from_rgba(164, 178, 202, 210),
    );
}

fn draw_bottom_panel(game: &mut Game, state: &mut EditorState, rect: RectSpec) {
    if rect.h <= 0.0 {
        return;
    }
    draw_surface(rect, false);
    draw_rectangle(rect.x, rect.y, rect.w, 2.0, ui_accent());
    let mut x = rect.x + 12.0;
    for tab in [
        BottomTab::Console,
        BottomTab::Assets,
        BottomTab::Programming,
        BottomTab::Prefabs,
        BottomTab::Scenes,
        BottomTab::Sprites,
        BottomTab::Profiler,
    ] {
        let width = match tab {
            BottomTab::Console => 76.0,
            BottomTab::Assets => 70.0,
            BottomTab::Programming => 70.0,
            BottomTab::Prefabs => 78.0,
            BottomTab::Scenes => 74.0,
            BottomTab::Sprites => 76.0,
            BottomTab::Profiler => 78.0,
        };
        if button(
            x,
            rect.y + 10.0,
            width,
            24.0,
            tab.label(),
            state.bottom_tab == tab,
        ) {
            state.bottom_tab = tab;
        }
        x += width + 6.0;
    }
    if button(
        rect.x + rect.w - 80.0,
        rect.y + 10.0,
        62.0,
        24.0,
        "Hide",
        false,
    ) {
        state.show_console = false;
    }

    let content = RectSpec {
        x: rect.x,
        y: rect.y + 38.0,
        w: rect.w,
        h: rect.h - 38.0,
    };
    match state.bottom_tab {
        BottomTab::Console => draw_console(game, content),
        BottomTab::Assets => draw_assets_panel(game, state, content),
        BottomTab::Programming => draw_programming_panel(game, state, content),
        BottomTab::Prefabs => draw_prefab_panel(game, state, content),
        BottomTab::Scenes => draw_scenes_panel(game, state, content),
        BottomTab::Sprites => draw_sprite_editor_panel(game, state, content),
        BottomTab::Profiler => draw_profiler_panel(game, content),
    }
}

fn draw_console(game: &mut Game, rect: RectSpec) {
    draw_text("Console", rect.x + 14.0, rect.y + 20.0, 18.0, ui_text());
    if button(rect.x + 92.0, rect.y + 4.0, 54.0, 22.0, "Clear", false) {
        game.console.clear();
    }
    if button(rect.x + 154.0, rect.y + 4.0, 84.0, 22.0, "Clear Err", false) {
        game.console.clear_channel("ERROR");
    }
    draw_text(
        &game.console.summary(),
        rect.x + 250.0,
        rect.y + 20.0,
        14.0,
        Color::from_rgba(164, 178, 202, 255),
    );
    let lines = ((rect.h - 34.0) / 19.0).max(0.0) as usize;
    let start = game.console.structured_entries.len().saturating_sub(lines);
    let mut y = rect.y + 42.0;
    for entry in game.console.structured_entries.iter().skip(start) {
        let color = console_color(&entry.channel, entry.severity);
        draw_text(
            &ellipsize(
                &format!(
                    "#{} [{:?}][{}] {}",
                    entry.frame, entry.severity, entry.channel, entry.message
                ),
                155,
            ),
            rect.x + 14.0,
            y,
            16.0,
            color,
        );
        y += 19.0;
    }
}

fn console_color(
    channel: &str,
    severity: crate::engine::developer_console::ConsoleSeverity,
) -> Color {
    use crate::engine::developer_console::ConsoleSeverity;
    match severity {
        ConsoleSeverity::Error => Color::from_rgba(255, 110, 110, 255),
        ConsoleSeverity::Warning => Color::from_rgba(255, 210, 100, 255),
        ConsoleSeverity::Debug => Color::from_rgba(145, 152, 172, 255),
        ConsoleSeverity::Info => match channel {
            "SCRIPT" => Color::from_rgba(130, 220, 255, 255),
            "SCENE" => Color::from_rgba(150, 255, 180, 255),
            "VALIDATOR" => Color::from_rgba(180, 230, 255, 255),
            "TILEMAP" => Color::from_rgba(255, 225, 120, 255),
            "SPRITE" => Color::from_rgba(210, 180, 255, 255),
            _ => Color::from_rgba(210, 216, 230, 255),
        },
    }
}

fn draw_assets_panel(game: &mut Game, state: &mut EditorState, rect: RectSpec) {
    let mut counts = std::collections::BTreeMap::<String, usize>::new();
    for asset in game.asset_database.assets.values() {
        *counts.entry(asset.asset_type.clone()).or_default() += 1;
    }
    draw_panel_chrome(
        rect,
        "Content Browser",
        &format!(
            "{} assets | {} graphs | source {}",
            game.asset_database.assets.len(),
            game.visual_graph_asset_count(),
            state.content_source
        ),
    );
    if button(rect.x + 14.0, rect.y + 31.0, 74.0, 22.0, "+ Add", false) {
        if let Ok(path) = game.create_luau_script_asset("NewGameplayScript") {
            set_open_file_editor_state(state, &path);
        }
        state.bottom_tab = BottomTab::Programming;
    }
    if button(rect.x + 96.0, rect.y + 31.0, 78.0, 22.0, "Import 2D", false) {
        import_2d_asset_dialog(game, state);
    }
    if button(rect.x + 182.0, rect.y + 31.0, 72.0, 22.0, "Save All", false) {
        save_project(game);
    }
    if button(rect.x + 262.0, rect.y + 31.0, 76.0, 22.0, "Refresh", false) {
        refresh_assets(game);
    }
    if button(rect.x + 346.0, rect.y + 31.0, 78.0, 22.0, "Manifest", false) {
        build_manifest(game);
    }
    if button(rect.x + 432.0, rect.y + 31.0, 80.0, 22.0, "+ Graph", false) {
        if let Ok(path) = game.create_program_asset("LogAndMove") {
            set_open_file_editor_state(state, &path);
        }
        state.bottom_tab = BottomTab::Programming;
    }
    if button(rect.x + 520.0, rect.y + 31.0, 82.0, 22.0, "+ Prefab", false) {
        game.save_selected_as_prefab().ok();
    }
    if button(rect.x + 610.0, rect.y + 31.0, 78.0, 22.0, "+ Sprite", false) {
        game.create_sprite_import_asset("NewSprite", "assets/sprites/new.png")
            .ok();
    }
    if button(rect.x + 696.0, rect.y + 31.0, 78.0, 22.0, "+ Sound", false) {
        game.create_sound_cue_asset("NewCue", "assets/audio/new.wav")
            .ok();
    }
    if button(
        rect.x + 782.0,
        rect.y + 31.0,
        92.0,
        22.0,
        "+ Material",
        false,
    ) {
        game.create_material_asset("SpriteMaterial").ok();
    }
    if button(
        rect.x + rect.w - 164.0,
        rect.y + 31.0,
        74.0,
        22.0,
        "Build D",
        false,
    ) {
        export_runtime(game, ExportProfile::Debug);
    }
    if button(
        rect.x + rect.w - 84.0,
        rect.y + 31.0,
        74.0,
        22.0,
        "Build R",
        false,
    ) {
        export_runtime(game, ExportProfile::Release);
    }

    let sources_panel = RectSpec {
        x: rect.x + 10.0,
        y: rect.y + 64.0,
        w: if rect.w > 980.0 { 188.0 } else { 152.0 },
        h: rect.h - 72.0,
    };
    let preview_w = if rect.w > 1180.0 {
        (rect.w * 0.24).clamp(300.0, 460.0)
    } else {
        (rect.w * 0.28).clamp(240.0, 320.0)
    };
    let browser_panel = RectSpec {
        x: sources_panel.x + sources_panel.w + 10.0,
        y: sources_panel.y,
        w: (rect.w - sources_panel.w - preview_w - 42.0).max(300.0),
        h: sources_panel.h,
    };
    let preview_panel = RectSpec {
        x: browser_panel.x + browser_panel.w + 10.0,
        y: browser_panel.y,
        w: preview_w.min(rect.x + rect.w - browser_panel.x - browser_panel.w - 20.0),
        h: browser_panel.h,
    };
    draw_sources_panel(game, state, sources_panel);
    draw_surface(browser_panel, false);
    draw_rectangle_lines(
        browser_panel.x,
        browser_panel.y,
        browser_panel.w,
        browser_panel.h,
        1.0,
        ui_line_soft(),
    );

    let search = RectSpec {
        x: browser_panel.x + 10.0,
        y: browser_panel.y + 10.0,
        w: browser_panel.w - 20.0,
        h: 26.0,
    };
    draw_search_box(state, search);
    let mut fx = browser_panel.x + 10.0;
    let filter_y = browser_panel.y + 44.0;
    if button(
        fx,
        filter_y,
        58.0,
        21.0,
        "All",
        state.content_type_filter.is_none(),
    ) {
        state.content_type_filter = None;
        state.content_scroll = 0.0;
    }
    fx += 66.0;
    for (asset_type, count) in counts.iter().take(7) {
        let label = format!("{asset_type} {count}");
        let width = (measure_text(&label, None, 12, 1.0).width + 18.0).min(116.0);
        if button(
            fx,
            filter_y,
            width,
            21.0,
            &ellipsize(&label, 14),
            state.content_type_filter.as_deref() == Some(asset_type.as_str()),
        ) {
            state.content_type_filter = Some(asset_type.clone());
            state.content_scroll = 0.0;
        }
        fx += width + 6.0;
    }

    let source_prefix = source_prefix(&state.content_source);
    let mut assets = game
        .asset_database
        .assets
        .values()
        .filter(|asset| source_prefix.is_empty() || asset.relative_path.starts_with(source_prefix))
        .filter(|asset| {
            state
                .content_type_filter
                .as_deref()
                .is_none_or(|filter| asset.asset_type == filter)
        })
        .cloned()
        .collect::<Vec<_>>();
    if state.content_search.trim().is_empty() {
        assets.sort_by(|a, b| {
            a.asset_type
                .cmp(&b.asset_type)
                .then_with(|| a.relative_path.cmp(&b.relative_path))
        });
    } else {
        let searchable = assets
            .iter()
            .map(|asset| {
                format!(
                    "{} {} {} {}",
                    asset.name,
                    asset.relative_path,
                    asset.asset_type,
                    asset.labels.join(" ")
                )
            })
            .collect::<Vec<_>>();
        assets = fuzzy_rank(&state.content_search, &searchable, searchable.len())
            .into_iter()
            .map(|result| assets[result.index].clone())
            .collect();
    }

    let asset_view = RectSpec {
        x: browser_panel.x + 10.0,
        y: browser_panel.y + 74.0,
        w: browser_panel.w - 20.0,
        h: browser_panel.h - 86.0,
    };
    draw_asset_grid(game, state, asset_view, &assets);
    draw_surface(preview_panel, false);
    draw_rectangle_lines(
        preview_panel.x,
        preview_panel.y,
        preview_panel.w,
        preview_panel.h,
        1.0,
        ui_line_soft(),
    );
    if assets.is_empty() {
        draw_text(
            "No assets in this view",
            asset_view.x + 12.0,
            asset_view.y + 28.0,
            14.0,
            Color::from_rgba(150, 166, 192, 255),
        );
    }
    draw_asset_preview(game, state, preview_panel);
}

#[derive(Debug, Clone)]
struct Imported2DAsset {
    image_path: PathBuf,
    sprite_manifest: PathBuf,
    sheet_manifest: Option<PathBuf>,
    frames_manifest: Option<PathBuf>,
    frame_count: usize,
}

#[derive(Debug, Clone, Copy)]
struct SpriteGridGuess {
    cell_width: u32,
    cell_height: u32,
    columns: u32,
    rows: u32,
}

fn import_2d_asset_dialog(game: &mut Game, state: &mut EditorState) {
    let Some(path) = rfd::FileDialog::new()
        .set_title("Importar asset 2D")
        .add_filter(
            "Assets 2D",
            &["png", "jpg", "jpeg", "webp", "aseprite", "spriteframes"],
        )
        .add_filter("Imagenes", &["png", "jpg", "jpeg", "webp"])
        .pick_file()
    else {
        game.console.log("Import 2D cancelado", "ASSETS");
        return;
    };

    match import_2d_asset(game, &path) {
        Ok(imported) => {
            let sprite_path = relative_project_path(game, &imported.sprite_manifest);
            state.selected_asset_path = Some(sprite_path.clone());
            state.content_source = "Sprites".to_string();
            state.content_type_filter = Some("Sprite".to_string());
            state.content_scroll = 0.0;
            state.bottom_tab = BottomTab::Assets;
            refresh_assets(game);
            let mut details = format!(
                "Import 2D listo: {} -> {}",
                imported
                    .image_path
                    .file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or("asset"),
                sprite_path
            );
            if let Some(sheet) = imported.sheet_manifest.as_ref() {
                details.push_str(&format!(" | sheet {}", relative_project_path(game, sheet)));
            }
            if let Some(frames) = imported.frames_manifest.as_ref() {
                details.push_str(&format!(
                    " | anim {} frames {}",
                    imported.frame_count,
                    relative_project_path(game, frames)
                ));
            }
            game.console.log(details, "ASSETS");
        }
        Err(error) => game
            .console
            .log(format!("Import 2D fallo: {error}"), "ERROR"),
    }
}

fn import_2d_asset(game: &mut Game, source_path: &Path) -> io::Result<Imported2DAsset> {
    let extension = source_path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let supported = matches!(
        extension.as_str(),
        "png" | "jpg" | "jpeg" | "webp" | "aseprite"
    );
    if !supported {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "selecciona PNG, JPG, WEBP o ASEPRITE para importar sprites 2D",
        ));
    }

    let target_folder = AssetTools::create_special_folder(&game.project_path, "sprites")?;
    let image_path = AssetTools::safe_copy_to_folder(source_path, target_folder)?;
    let sprite_name = image_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("Sprite")
        .to_string();
    let source_ref = relative_project_path(game, &image_path);
    let sprite_manifest = game.create_sprite_import_asset(&sprite_name, &source_ref)?;

    let mut sheet_manifest = None;
    let mut frames_manifest = None;
    let mut frame_count = 1usize;

    if SpriteSheetImporter::supports_image(&image_path)
        && let Ok(grid) = infer_sprite_grid(&image_path)
    {
        let sheet = SpriteSheetImporter::build_metadata(
            &image_path,
            grid.cell_width,
            grid.cell_height,
            0,
            0,
        )?;
        frame_count = sheet.slices.len().max(1);
        let sheet_path = SpriteSheetImporter::write_sidecar(&image_path, &sheet)?;
        sheet_manifest = Some(sheet_path.clone());

        if frame_count > 1 {
            let animation_folder = game.project_path.join("assets").join("animations");
            fs::create_dir_all(&animation_folder)?;
            let animation_path =
                AssetTools::unique_path(&animation_folder, &format!("{sprite_name}.spriteframes"));
            let frames = SpriteFrames2D::grid_slice(
                sprite_name.clone(),
                source_ref.clone(),
                grid.columns,
                grid.rows,
                grid.cell_width,
                grid.cell_height,
                8.0,
            );
            let frames_value = serde_json::to_value(frames).map_err(io::Error::other)?;
            AssetTools::write_json(&animation_path, &frames_value)?;
            frames_manifest = Some(animation_path.clone());
            patch_sprite_import_links(
                game,
                &sprite_manifest,
                Some(&sheet_path),
                Some(&animation_path),
                frame_count,
            )?;
        } else {
            patch_sprite_import_links(
                game,
                &sprite_manifest,
                Some(&sheet_path),
                None,
                frame_count,
            )?;
        }
    }

    Ok(Imported2DAsset {
        image_path,
        sprite_manifest,
        sheet_manifest,
        frames_manifest,
        frame_count,
    })
}

fn patch_sprite_import_links(
    game: &Game,
    sprite_manifest: &Path,
    sheet_manifest: Option<&Path>,
    frames_manifest: Option<&Path>,
    frame_count: usize,
) -> io::Result<()> {
    let mut sprite = AssetTools::read_json(sprite_manifest)?;
    if let Some(sheet) = sheet_manifest {
        sprite["atlas"] = json!(relative_project_path(game, sheet));
    }
    if let Some(frames) = frames_manifest {
        sprite["animations"] = json!([{
            "name": "default",
            "asset": relative_project_path(game, frames),
            "fps": 8.0,
            "frames": frame_count
        }]);
    }
    AssetTools::write_json(sprite_manifest, &sprite)
}

fn infer_sprite_grid(path: &Path) -> io::Result<SpriteGridGuess> {
    let reader = image::ImageReader::open(path)
        .map_err(io::Error::other)?
        .with_guessed_format()
        .map_err(io::Error::other)?;
    let (width, height) = reader.into_dimensions().map_err(io::Error::other)?;
    if width == 0 || height == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "la imagen no tiene dimensiones validas",
        ));
    }

    let horizontal_frames = width / height.max(1);
    if width % height == 0 && (2..=64).contains(&horizontal_frames) {
        return Ok(SpriteGridGuess {
            cell_width: height,
            cell_height: height,
            columns: horizontal_frames,
            rows: 1,
        });
    }

    let vertical_frames = height / width.max(1);
    if height % width == 0 && (2..=64).contains(&vertical_frames) {
        return Ok(SpriteGridGuess {
            cell_width: width,
            cell_height: width,
            columns: 1,
            rows: vertical_frames,
        });
    }

    for cell in [64, 32, 16, 8] {
        if width % cell == 0 && height % cell == 0 && (width / cell) * (height / cell) > 1 {
            return Ok(SpriteGridGuess {
                cell_width: cell,
                cell_height: cell,
                columns: width / cell,
                rows: height / cell,
            });
        }
    }

    Ok(SpriteGridGuess {
        cell_width: width,
        cell_height: height,
        columns: 1,
        rows: 1,
    })
}

fn relative_project_path(game: &Game, path: &Path) -> String {
    path.strip_prefix(&game.project_path)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn draw_panel_chrome(rect: RectSpec, title: &str, subtitle: &str) {
    draw_surface(rect, false);
    draw_panel_header(rect, title, subtitle);
}

fn draw_sources_panel(game: &Game, state: &mut EditorState, rect: RectSpec) {
    draw_surface(rect, false);
    draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 1.0, ui_line_soft());
    draw_text("Sources", rect.x + 12.0, rect.y + 23.0, 16.0, ui_text());
    let sources = [
        ("Content", "", "project"),
        ("Assets", "assets/", "assets"),
        ("Sprites", "assets/sprites/", "image"),
        ("Audio", "assets/audio/", "sound"),
        ("Prefabs", "assets/prefabs/", "prefab"),
        ("Scripts", "scripts/", "code"),
        ("Graphs", "scripts/visual_graphs/", "graph"),
        ("Scenes", "saves/scenes/", "scene"),
        ("Settings", "settings/", "gear"),
    ];
    let mut y = rect.y + 46.0;
    for (label, prefix, icon) in sources {
        let count = if prefix.is_empty() {
            game.asset_database.assets.len()
        } else {
            game.asset_database
                .assets
                .values()
                .filter(|asset| asset.relative_path.starts_with(prefix))
                .count()
        };
        let row = RectSpec {
            x: rect.x + 8.0,
            y: y - 16.0,
            w: rect.w - 16.0,
            h: 24.0,
        };
        let selected = state.content_source == label;
        let hovered = contains_mouse(row);
        draw_rect(
            row,
            if selected {
                Color::from_rgba(42, 78, 116, 255)
            } else if hovered {
                Color::from_rgba(33, 45, 64, 255)
            } else {
                Color::from_rgba(18, 24, 35, 255)
            },
        );
        draw_source_icon(
            icon,
            RectSpec {
                x: row.x + 7.0,
                y: row.y + 4.0,
                w: 18.0,
                h: 16.0,
            },
            selected,
        );
        draw_text_fit(
            label,
            RectSpec {
                x: row.x + 32.0,
                y: row.y + 2.0,
                w: (row.w - 66.0).max(24.0),
                h: row.h - 4.0,
            },
            14,
            ui_text(),
            TextAlign::Left,
        );
        draw_text_fit(
            &count.to_string(),
            RectSpec {
                x: row.x + row.w - 30.0,
                y: row.y + 3.0,
                w: 22.0,
                h: row.h - 6.0,
            },
            12,
            ui_text_muted(),
            TextAlign::Right,
        );
        if hovered && is_mouse_button_pressed(MouseButton::Left) {
            state.content_source = label.to_string();
            state.content_scroll = 0.0;
        }
        y += 26.0;
    }
    draw_text(
        "Collections",
        rect.x + 12.0,
        rect.y + rect.h - 56.0,
        13.0,
        Color::from_rgba(150, 166, 192, 255),
    );
    draw_text(
        "Local | Shared | Runtime",
        rect.x + 12.0,
        rect.y + rect.h - 34.0,
        12.0,
        Color::from_rgba(112, 132, 162, 255),
    );
}

fn draw_search_box(state: &mut EditorState, rect: RectSpec) {
    draw_gradient_rect(
        rect,
        if state.content_search_active {
            Color::from_rgba(30, 43, 63, 255)
        } else {
            Color::from_rgba(16, 21, 31, 255)
        },
        Color::from_rgba(10, 14, 22, 255),
    );
    draw_rectangle_lines(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        1.0,
        if state.content_search_active {
            ui_accent()
        } else {
            ui_line_soft()
        },
    );
    draw_text(
        "Search",
        rect.x + 10.0,
        rect.y + 18.0,
        13.0,
        ui_text_muted(),
    );
    let text = if state.content_search.is_empty() {
        "name, type, path"
    } else {
        &state.content_search
    };
    draw_text(
        &ellipsize(text, 74),
        rect.x + 70.0,
        rect.y + 18.0,
        13.0,
        if state.content_search.is_empty() {
            Color::from_rgba(93, 110, 138, 255)
        } else {
            ui_text()
        },
    );
    if contains_mouse(rect) && is_mouse_button_pressed(MouseButton::Left) {
        state.content_search_active = true;
    }
}

fn draw_graph_node_search_box(state: &mut EditorState, rect: RectSpec) {
    draw_inline_search_box(
        rect,
        "Node",
        "health, move, ui",
        &state.graph_node_search,
        state.graph_node_search_active,
    );
    if contains_mouse(rect) && is_mouse_button_pressed(MouseButton::Left) {
        state.graph_node_search_active = true;
        state.graph_template_search_active = false;
        state.code_editor_active = false;
    }
}

fn draw_graph_template_search_box(state: &mut EditorState, rect: RectSpec) {
    draw_inline_search_box(
        rect,
        "Search",
        "blueprint template",
        &state.graph_template_search,
        state.graph_template_search_active,
    );
    if contains_mouse(rect) && is_mouse_button_pressed(MouseButton::Left) {
        state.graph_template_search_active = true;
        state.graph_node_search_active = false;
        state.code_editor_active = false;
    }
}

fn draw_script_search_box(state: &mut EditorState, rect: RectSpec) {
    draw_inline_search_box(
        rect,
        "Find",
        "function, error, text",
        &state.script_search,
        state.script_search_active,
    );
    if contains_mouse(rect) && is_mouse_button_pressed(MouseButton::Left) {
        state.script_search_active = true;
        state.graph_node_search_active = false;
        state.graph_template_search_active = false;
        state.content_search_active = false;
        state.code_editor_active = false;
    }
}

fn draw_inline_search_box(
    rect: RectSpec,
    label: &str,
    placeholder: &str,
    value: &str,
    active: bool,
) {
    draw_gradient_rect(
        rect,
        if active {
            Color::from_rgba(30, 43, 63, 255)
        } else {
            Color::from_rgba(16, 21, 31, 255)
        },
        Color::from_rgba(10, 14, 22, 255),
    );
    draw_rectangle_lines(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        1.0,
        if active { ui_accent() } else { ui_line_soft() },
    );
    draw_text(label, rect.x + 8.0, rect.y + 17.0, 12.0, ui_text_muted());
    let text = if value.is_empty() { placeholder } else { value };
    draw_text(
        &ellipsize(text, 34),
        rect.x + 64.0,
        rect.y + 17.0,
        12.0,
        if value.is_empty() {
            Color::from_rgba(93, 110, 138, 255)
        } else {
            ui_text()
        },
    );
}

fn draw_asset_grid(
    game: &mut Game,
    state: &mut EditorState,
    rect: RectSpec,
    assets: &[AssetRecord],
) {
    let card_w = 116.0;
    let card_h = 104.0;
    let gap = 10.0;
    let cols = ((rect.w + gap) / (card_w + gap)).floor().max(1.0) as usize;
    let row_h = card_h + gap;
    let rows = if assets.is_empty() {
        0
    } else {
        assets.len().div_ceil(cols)
    };
    let content_h = rows as f32 * row_h;
    let max_scroll = (content_h - rect.h).max(0.0);
    if contains_mouse(rect) {
        let (_, wheel_y) = mouse_wheel();
        if wheel_y.abs() > f32::EPSILON {
            state.content_scroll =
                (state.content_scroll - wheel_y * row_h * 0.65).clamp(0.0, max_scroll);
        }
    }
    state.content_scroll = state.content_scroll.clamp(0.0, max_scroll);
    let first_row = (state.content_scroll / row_h).floor().max(0.0) as usize;
    let visible_rows = (rect.h / row_h).ceil().max(1.0) as usize + 2;
    let start_index = first_row.saturating_mul(cols).min(assets.len());
    let end_index = ((first_row + visible_rows).saturating_mul(cols)).min(assets.len());

    for (index, asset) in assets
        .iter()
        .enumerate()
        .skip(start_index)
        .take(end_index.saturating_sub(start_index))
    {
        let col = index % cols;
        let row = index / cols;
        let x = rect.x + col as f32 * (card_w + gap);
        let y = rect.y + row as f32 * row_h - state.content_scroll;
        let card = RectSpec {
            x,
            y,
            w: card_w,
            h: card_h,
        };
        if card.y + card.h < rect.y || card.y > rect.y + rect.h {
            continue;
        }
        let selected = state.selected_asset_path.as_deref() == Some(asset.relative_path.as_str());
        let hovered = contains_mouse(card);
        draw_rect(
            card,
            if selected {
                Color::from_rgba(47, 73, 108, 255)
            } else if hovered {
                Color::from_rgba(36, 45, 58, 255)
            } else {
                Color::from_rgba(25, 30, 40, 255)
            },
        );
        draw_rectangle_lines(
            card.x,
            card.y,
            card.w,
            card.h,
            1.0,
            if selected {
                Color::from_rgba(92, 169, 255, 255)
            } else {
                Color::from_rgba(54, 64, 80, 255)
            },
        );
        draw_asset_thumbnail(
            &asset.asset_type,
            RectSpec {
                x: card.x + 12.0,
                y: card.y + 10.0,
                w: card.w - 24.0,
                h: 48.0,
            },
            !asset.compatibility.is_empty(),
        );
        if !asset.compatibility.is_empty() {
            draw_text(
                "!",
                card.x + card.w - 21.0,
                card.y + 24.0,
                18.0,
                Color::from_rgba(255, 206, 130, 255),
            );
        }
        draw_text_fit(
            &asset.name,
            RectSpec {
                x: card.x + 10.0,
                y: card.y + 62.0,
                w: card.w - 20.0,
                h: 22.0,
            },
            13,
            Color::from_rgba(232, 238, 248, 255),
            TextAlign::Center,
        );
        draw_text_fit(
            &asset.asset_type,
            RectSpec {
                x: card.x + 10.0,
                y: card.y + 82.0,
                w: card.w - 20.0,
                h: 18.0,
            },
            11,
            Color::from_rgba(145, 162, 190, 255),
            TextAlign::Center,
        );
        if hovered && is_mouse_button_pressed(MouseButton::Left) {
            activate_asset_from_browser(game, state, asset);
        }
        if hovered && is_mouse_button_pressed(MouseButton::Right) {
            activate_asset_from_browser(game, state, asset);
        }
    }
    draw_scrollbar(
        rect,
        state.content_scroll,
        max_scroll,
        content_h.max(rect.h),
    );
}

fn draw_scrollbar(rect: RectSpec, scroll: f32, max_scroll: f32, content_h: f32) {
    if max_scroll <= 0.0 || content_h <= rect.h {
        return;
    }
    let track = RectSpec {
        x: rect.x + rect.w - 6.0,
        y: rect.y,
        w: 5.0,
        h: rect.h,
    };
    draw_rect(track, Color::from_rgba(10, 13, 19, 220));
    let thumb_h = (rect.h * (rect.h / content_h)).clamp(28.0, rect.h);
    let thumb_y = rect.y + (rect.h - thumb_h) * (scroll / max_scroll).clamp(0.0, 1.0);
    draw_rect(
        RectSpec {
            x: track.x,
            y: thumb_y,
            w: track.w,
            h: thumb_h,
        },
        Color::from_rgba(84, 116, 158, 255),
    );
}

fn activate_asset_from_browser(game: &mut Game, state: &mut EditorState, asset: &AssetRecord) {
    state.selected_asset_path = Some(asset.relative_path.clone());
    state.drag_payload = Some(DragPayload::from_asset(asset));

    match asset.asset_type.as_str() {
        "Scene" => open_scene_asset(game, asset),
        "Prefab" | "UI" => {
            state.show_console = true;
            state.bottom_tab = BottomTab::Prefabs;
            game.console.log(
                format!("Prefab seleccionado: {}", asset.relative_path),
                "ASSETS",
            );
        }
        "LuauScript" | "VisualGraph" | "Data" | "Material" | "Shader" | "AudioEvent" => {
            open_project_file_in_editor(game, state, asset.relative_path.clone());
        }
        _ if asset.relative_path.ends_with(".luau")
            || asset.relative_path.ends_with(".mfgraph")
            || asset.relative_path.ends_with(".json")
            || asset.relative_path.ends_with(".prefab")
            || asset.relative_path.ends_with(".material")
            || asset.relative_path.ends_with(".shader") =>
        {
            open_project_file_in_editor(game, state, asset.relative_path.clone());
        }
        _ => {
            game.console.log(
                format!(
                    "Asset seleccionado para preview/drag: {}",
                    asset.relative_path
                ),
                "ASSETS",
            );
        }
    }
}

fn open_scene_asset(game: &mut Game, asset: &AssetRecord) {
    let name = asset
        .relative_path
        .rsplit('/')
        .next()
        .unwrap_or(asset.relative_path.as_str());
    match game.load_scene(name) {
        Ok(count) => game.console.log(
            format!("Escena abierta desde Browser: {name} ({count} entidades)"),
            "SCENE",
        ),
        Err(error) => game
            .console
            .log(format!("No se pudo abrir escena {name}: {error}"), "ERROR"),
    }
}

fn open_project_file_in_editor(
    game: &mut Game,
    state: &mut EditorState,
    path: impl AsRef<Path>,
) -> bool {
    match game.open_project_file(path.as_ref()) {
        Ok(opened) => {
            set_open_file_editor_state(state, &opened);
            state.show_console = true;
            state.bottom_tab = BottomTab::Programming;
            if state.editor_preferences.open_external_on_script
                && opened.extension().and_then(|value| value.to_str()) != Some("mfgraph")
            {
                open_file_in_external_editor(game, state, &opened);
            }
            true
        }
        Err(error) => {
            game.console
                .log(format!("No se pudo abrir asset: {error}"), "ERROR");
            false
        }
    }
}

fn set_open_file_editor_state(state: &mut EditorState, path: &Path) {
    let extension = path.extension().and_then(|value| value.to_str());
    state.code_editor_active = extension != Some("mfgraph");
    state.script_window_open = true;
    state.code_cursor_line = 0;
    state.code_cursor_column = 0;
    state.code_scroll_line = 0;
    state.graph_selected_node = None;
    state.graph_connect_from = None;
    state.graph_drag_node = None;
}

fn source_prefix(source: &str) -> &'static str {
    match source {
        "Assets" => "assets/",
        "Sprites" => "assets/sprites/",
        "Audio" => "assets/audio/",
        "Prefabs" => "assets/prefabs/",
        "Scripts" => "scripts/",
        "Graphs" => "scripts/visual_graphs/",
        "Scenes" => "saves/scenes/",
        "Settings" => "settings/",
        _ => "",
    }
}

fn draw_source_icon(kind: &str, rect: RectSpec, active: bool) {
    let color = if active { ui_accent_2() } else { ui_accent() };
    match kind {
        "assets" | "project" => {
            draw_rectangle(rect.x, rect.y + 4.0, rect.w, rect.h - 4.0, color);
            draw_rectangle(rect.x + 2.0, rect.y, rect.w * 0.48, 6.0, color);
            draw_rectangle_lines(rect.x, rect.y + 4.0, rect.w, rect.h - 4.0, 1.0, ui_line());
        }
        "image" => {
            draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 1.0, color);
            draw_rectangle(rect.x + 3.0, rect.y + 9.0, rect.w - 6.0, 4.0, color);
            draw_circle(rect.x + rect.w - 5.0, rect.y + 5.0, 2.0, color);
        }
        "sound" => {
            for index in 0..4 {
                let h = 5.0 + index as f32 * 3.0;
                draw_rectangle(
                    rect.x + index as f32 * 4.0 + 1.0,
                    rect.y + rect.h - h,
                    2.5,
                    h,
                    color,
                );
            }
        }
        "graph" => {
            draw_circle(rect.x + 4.0, rect.y + 5.0, 3.0, color);
            draw_circle(rect.x + rect.w - 4.0, rect.y + rect.h - 5.0, 3.0, color);
            draw_line(
                rect.x + 6.0,
                rect.y + 6.0,
                rect.x + rect.w - 6.0,
                rect.y + rect.h - 6.0,
                2.0,
                color,
            );
        }
        "scene" => {
            draw_rectangle_lines(
                rect.x + 2.0,
                rect.y + 1.0,
                rect.w - 4.0,
                rect.h - 2.0,
                1.0,
                color,
            );
            draw_line(
                rect.x + 5.0,
                rect.y + rect.h - 4.0,
                rect.x + rect.w - 5.0,
                rect.y + 4.0,
                1.0,
                color,
            );
        }
        "gear" => {
            draw_circle_lines(
                rect.x + rect.w * 0.5,
                rect.y + rect.h * 0.5,
                6.0,
                2.0,
                color,
            );
            draw_circle(rect.x + rect.w * 0.5, rect.y + rect.h * 0.5, 2.0, color);
        }
        "code" => {
            draw_text("{}", rect.x + 1.0, rect.y + 13.0, 13.0, color);
        }
        "prefab" => {
            draw_rectangle_lines(rect.x + 2.0, rect.y + 2.0, 10.0, 10.0, 1.0, color);
            draw_rectangle_lines(rect.x + 7.0, rect.y + 6.0, 10.0, 10.0, 1.0, color);
        }
        _ => draw_circle(rect.x + rect.w * 0.5, rect.y + rect.h * 0.5, 5.0, color),
    }
}

fn asset_icon(asset_type: &str) -> &'static str {
    match asset_type {
        "Sprite" | "Sprite2D" => "SPR",
        "SpriteSheet" => "SHT",
        "SpriteFrames2D" | "FlipbookAnimation2D" => "ANI",
        "Audio" | "AudioEvent" | "Audio2D" => "AUD",
        "Prefab" | "UI" => "PFB",
        "VisualGraph" | "BlueprintGraph2D" | "AnimationBlueprint2D" => "NOD",
        "LuauScript" => "LUA",
        "Scene" | "Scene2D" => "LVL",
        "Material" | "Material2D" => "MAT",
        "Shader" => "SHD",
        "Tilemap" | "Tilemap2D" | "Tileset2D" => "TIL",
        "ParticlePreset" | "Particles2D" => "FX",
        "Font" => "FNT",
        _ => "DAT",
    }
}

fn asset_type_color(asset_type: &str) -> Color {
    match asset_type {
        "Sprite" | "Sprite2D" | "SpriteSheet" | "SpriteFrames2D" => {
            Color::from_rgba(52, 128, 176, 255)
        }
        "Audio" | "AudioEvent" | "Audio2D" => Color::from_rgba(184, 114, 58, 255),
        "Prefab" | "UI" => Color::from_rgba(122, 94, 190, 255),
        "VisualGraph" | "BlueprintGraph2D" | "AnimationBlueprint2D" | "FlipbookAnimation2D" => {
            Color::from_rgba(54, 150, 137, 255)
        }
        "LuauScript" => Color::from_rgba(68, 118, 190, 255),
        "Scene" | "Scene2D" => Color::from_rgba(98, 150, 82, 255),
        "Material" | "Material2D" | "Shader" => Color::from_rgba(137, 99, 184, 255),
        "Tilemap" | "Tilemap2D" | "Tileset2D" => Color::from_rgba(102, 148, 92, 255),
        "ParticlePreset" | "Particles2D" => Color::from_rgba(194, 134, 66, 255),
        "Font" => Color::from_rgba(120, 132, 168, 255),
        _ => Color::from_rgba(82, 96, 118, 255),
    }
}

fn draw_asset_thumbnail(asset_type: &str, rect: RectSpec, has_warning: bool) {
    let color = asset_type_color(asset_type);
    draw_gradient_rect(
        rect,
        blend_color(color, WHITE, 0.08),
        blend_color(color, BLACK, 0.2),
    );
    draw_rectangle(
        rect.x + 4.0,
        rect.y + 4.0,
        rect.w - 8.0,
        rect.h - 8.0,
        Color::from_rgba(12, 15, 20, 68),
    );
    match asset_type {
        "Sprite" | "Sprite2D" | "SpriteSheet" | "SpriteFrames2D" => {
            let cell = 9.0;
            for row in 0..3 {
                for col in 0..5 {
                    let alpha = if (row + col) % 2 == 0 { 170 } else { 80 };
                    draw_rectangle(
                        rect.x + 18.0 + col as f32 * cell,
                        rect.y + 11.0 + row as f32 * cell,
                        cell - 1.0,
                        cell - 1.0,
                        Color::from_rgba(235, 247, 255, alpha),
                    );
                }
            }
        }
        "Audio" | "AudioEvent" | "Audio2D" => {
            for index in 0..14 {
                let h = ((index * 19) % 26 + 8) as f32;
                draw_rectangle(
                    rect.x + 12.0 + index as f32 * 5.1,
                    rect.y + rect.h * 0.5 - h * 0.5,
                    2.8,
                    h,
                    Color::from_rgba(255, 232, 186, 220),
                );
            }
        }
        "VisualGraph" | "BlueprintGraph2D" | "AnimationBlueprint2D" | "FlipbookAnimation2D" => {
            let points = [
                (rect.x + 24.0, rect.y + 15.0),
                (rect.x + rect.w - 24.0, rect.y + 15.0),
                (rect.x + rect.w * 0.5, rect.y + rect.h - 14.0),
            ];
            for pair in [(0, 1), (1, 2), (2, 0)] {
                draw_line(
                    points[pair.0].0,
                    points[pair.0].1,
                    points[pair.1].0,
                    points[pair.1].1,
                    2.0,
                    Color::from_rgba(230, 248, 255, 180),
                );
            }
            for point in points {
                draw_circle(point.0, point.1, 5.0, Color::from_rgba(230, 248, 255, 235));
            }
        }
        "Scene" | "Scene2D" => {
            draw_rectangle_lines(
                rect.x + 18.0,
                rect.y + 10.0,
                rect.w - 36.0,
                rect.h - 18.0,
                2.0,
                Color::from_rgba(235, 248, 230, 220),
            );
            draw_line(
                rect.x + 22.0,
                rect.y + rect.h - 12.0,
                rect.x + rect.w - 22.0,
                rect.y + 14.0,
                2.0,
                Color::from_rgba(235, 248, 230, 160),
            );
        }
        "Prefab" | "UI" => {
            draw_rectangle_lines(rect.x + 24.0, rect.y + 12.0, 26.0, 22.0, 2.0, WHITE);
            draw_rectangle_lines(
                rect.x + rect.w - 50.0,
                rect.y + 16.0,
                26.0,
                22.0,
                2.0,
                Color::from_rgba(230, 224, 255, 230),
            );
        }
        "Material" | "Material2D" | "Shader" => {
            draw_circle(rect.x + rect.w * 0.5, rect.y + rect.h * 0.5, 15.0, WHITE);
            draw_circle(
                rect.x + rect.w * 0.58,
                rect.y + rect.h * 0.42,
                13.0,
                Color::from_rgba(86, 214, 180, 190),
            );
        }
        _ => {
            draw_text_fit(
                asset_icon(asset_type),
                RectSpec {
                    x: rect.x + 8.0,
                    y: rect.y + 8.0,
                    w: rect.w - 16.0,
                    h: rect.h - 16.0,
                },
                19,
                WHITE,
                TextAlign::Center,
            );
        }
    }
    if has_warning {
        draw_circle(rect.x + rect.w - 9.0, rect.y + 9.0, 6.0, ui_warning());
    }
}

fn draw_asset_preview(game: &mut Game, state: &mut EditorState, rect: RectSpec) {
    draw_text("Asset Preview", rect.x + 12.0, rect.y + 23.0, 18.0, WHITE);
    let Some(path) = state.selected_asset_path.clone() else {
        draw_text(
            "Selecciona un asset para ver GUID, settings y dependencias.",
            rect.x + 12.0,
            rect.y + 54.0,
            14.0,
            Color::from_rgba(160, 170, 190, 255),
        );
        return;
    };
    let Some(preview) = game.asset_database.preview(&path) else {
        draw_text(
            "Asset no encontrado en database.",
            rect.x + 12.0,
            rect.y + 54.0,
            14.0,
            Color::from_rgba(255, 160, 130, 255),
        );
        return;
    };

    draw_preview_visual(
        &preview.asset_type,
        rect.x + 12.0,
        rect.y + 36.0,
        104.0,
        74.0,
    );
    let mut y = rect.y + 54.0;
    draw_text(
        &ellipsize(&format!("{} | {}", preview.kind.label(), preview.name), 34),
        rect.x + 128.0,
        y,
        16.0,
        Color::from_rgba(230, 235, 246, 255),
    );
    y += 20.0;
    draw_text(
        &ellipsize(&format!("GUID {}", preview.guid), 42),
        rect.x + 128.0,
        y,
        13.0,
        Color::from_rgba(170, 185, 210, 255),
    );
    y += 16.0;
    for chunk in preview.guid.as_bytes().chunks(24) {
        let s = std::str::from_utf8(chunk).unwrap_or("");
        if !s.is_empty() {
            draw_text(
                s,
                rect.x + 128.0,
                y,
                11.0,
                Color::from_rgba(130, 148, 175, 255),
            );
            y += 13.0;
        }
    }
    y += 6.0;
    draw_text(
        &ellipsize(&preview.path, 46),
        rect.x + 128.0,
        y,
        13.0,
        Color::from_rgba(170, 185, 210, 255),
    );
    y += 20.0;
    let button_y = (y + 6.0).max(rect.y + 118.0);
    if button(
        rect.x + 12.0,
        button_y,
        94.0,
        22.0,
        "Build On",
        preview.build_included(),
    ) {
        game.asset_database
            .set_import_setting(&path, "include_in_build", json!(!preview.build_included()))
            .ok();
    }
    if button(rect.x + 112.0, button_y, 80.0, 22.0, "Deps", false) {
        match game.asset_database.rebuild_dependency_graph() {
            Ok(_) => {
                let report = game.asset_database.dependency_report();
                game.console.log(
                    format!(
                        "Grafo de assets: {} nodos, {} enlaces, {} ciclos",
                        report.build_order.len(),
                        report.edge_count,
                        report.cycles.len()
                    ),
                    if report.cycles.is_empty() {
                        "ASSETS"
                    } else {
                        "WARNING"
                    },
                );
            }
            Err(error) => game.console.log(
                format!("No se pudo reconstruir dependencias: {error}"),
                "ERROR",
            ),
        }
    }
    if button(rect.x + 198.0, button_y, 80.0, 22.0, "Drag", false)
        && let Some(asset) = game.asset_database.assets.get(&path)
    {
        state.drag_payload = Some(DragPayload::from_asset(asset));
    }
    if button(rect.x + 284.0, button_y, 76.0, 22.0, "Open", false) {
        if let Some(asset) = game.asset_database.assets.get(&path).cloned() {
            activate_asset_from_browser(game, state, &asset);
        } else {
            open_project_file_in_editor(game, state, path.clone());
        }
    }

    y = button_y + 38.0;
    draw_asset_preview_section(rect.x + 12.0, &mut y, "Labels", &preview.labels);
    draw_asset_preview_section(rect.x + 12.0, &mut y, "Details", &preview.details);
    draw_asset_preview_section(rect.x + 12.0, &mut y, "Deps", &preview.dependencies);
    draw_asset_preview_section(
        rect.x + 12.0,
        &mut y,
        "Used By",
        &preview.reverse_dependencies,
    );
    draw_asset_preview_section(rect.x + 12.0, &mut y, "Warnings", &preview.warnings);

    draw_text(
        &ellipsize(&format!("Import {}", preview.import_settings), 74),
        rect.x + 12.0,
        rect.y + rect.h - 14.0,
        12.0,
        Color::from_rgba(150, 162, 184, 255),
    );
}

fn draw_preview_visual(asset_type: &str, x: f32, y: f32, w: f32, h: f32) {
    draw_rectangle(x, y, w, h, Color::from_rgba(18, 22, 30, 255));
    draw_rectangle_lines(x, y, w, h, 1.0, Color::from_rgba(70, 82, 104, 255));
    match asset_type {
        "Sprite" | "Sprite2D" | "SpriteSheet" | "SpriteFrames2D" => {
            for row in 0..4 {
                for col in 0..6 {
                    let c = if (row + col) % 2 == 0 {
                        Color::from_rgba(80, 120, 155, 255)
                    } else {
                        Color::from_rgba(34, 42, 56, 255)
                    };
                    draw_rectangle(
                        x + col as f32 * 16.0 + 4.0,
                        y + row as f32 * 16.0 + 5.0,
                        16.0,
                        16.0,
                        c,
                    );
                }
            }
            if matches!(asset_type, "SpriteFrames2D" | "SpriteSheet") {
                draw_rectangle(
                    x + 8.0,
                    y + h - 15.0,
                    w - 16.0,
                    5.0,
                    Color::from_rgba(255, 206, 105, 220),
                );
                for index in 1..5 {
                    let px = x + 8.0 + (w - 16.0) * index as f32 / 5.0;
                    draw_line(
                        px,
                        y + h - 17.0,
                        px,
                        y + h - 8.0,
                        1.0,
                        Color::from_rgba(20, 24, 32, 220),
                    );
                }
            }
        }
        "Audio" | "AudioEvent" | "Audio2D" => {
            for index in 0..16 {
                let height = ((index * 37) % 44 + 8) as f32;
                draw_rectangle(
                    x + 8.0 + index as f32 * 5.5,
                    y + h * 0.5 - height * 0.5,
                    3.0,
                    height,
                    Color::from_rgba(255, 185, 120, 255),
                );
            }
        }
        "VisualGraph" | "BlueprintGraph2D" | "AnimationBlueprint2D" => {
            let nodes = [
                (x + 20.0, y + 16.0, 30.0, 18.0),
                (x + w - 54.0, y + 20.0, 34.0, 18.0),
                (x + w * 0.5 - 18.0, y + h - 30.0, 36.0, 18.0),
            ];
            for pair in [(0, 1), (1, 2), (0, 2)] {
                let a = nodes[pair.0];
                let b = nodes[pair.1];
                draw_line(
                    a.0 + a.2 * 0.5,
                    a.1 + a.3 * 0.5,
                    b.0 + b.2 * 0.5,
                    b.1 + b.3 * 0.5,
                    2.0,
                    Color::from_rgba(120, 220, 255, 170),
                );
            }
            for node in nodes {
                draw_rectangle(
                    node.0,
                    node.1,
                    node.2,
                    node.3,
                    Color::from_rgba(50, 85, 120, 245),
                );
                draw_rectangle_lines(node.0, node.1, node.2, node.3, 1.0, ui_accent());
            }
        }
        "LuauScript" => {
            for row in 0..4 {
                let width = [52.0, 74.0, 44.0, 62.0][row];
                draw_rectangle(
                    x + 14.0,
                    y + 14.0 + row as f32 * 12.0,
                    width,
                    4.0,
                    Color::from_rgba(142, 188, 255, 220),
                );
            }
        }
        "Material" | "Material2D" | "Shader" => {
            draw_rectangle(
                x + 12.0,
                y + 12.0,
                w - 24.0,
                h - 24.0,
                Color::from_rgba(95, 160, 230, 255),
            );
            draw_rectangle(
                x + 28.0,
                y + 20.0,
                w - 46.0,
                h - 40.0,
                Color::from_rgba(155, 115, 230, 180),
            );
        }
        _ => {
            draw_text(
                "MF",
                x + 36.0,
                y + 46.0,
                24.0,
                Color::from_rgba(190, 205, 232, 255),
            );
        }
    }
}

fn draw_asset_preview_section(x: f32, y: &mut f32, title: &str, values: &[String]) {
    draw_text(title, x, *y, 13.0, Color::from_rgba(130, 202, 220, 255));
    *y += 16.0;
    if values.is_empty() {
        draw_text(
            "none",
            x + 10.0,
            *y,
            12.0,
            Color::from_rgba(126, 136, 156, 255),
        );
        *y += 16.0;
        return;
    }
    for value in values.iter().take(3) {
        draw_text(
            &ellipsize(value, 52),
            x + 10.0,
            *y,
            12.0,
            Color::from_rgba(190, 202, 222, 255),
        );
        *y += 16.0;
    }
}

fn draw_programming_panel(game: &mut Game, state: &mut EditorState, rect: RectSpec) {
    let catalog = game.programming.catalog_summary();
    draw_text("Programming", rect.x + 14.0, rect.y + 20.0, 18.0, WHITE);
    draw_text_fit(
        &format!(
            "{} | {} nodes | {} cats | {} actions",
            game.programming.summary(),
            catalog.node_count,
            catalog.categories.len(),
            catalog.quick_action_count
        ),
        RectSpec {
            x: rect.x + 128.0,
            y: rect.y + 4.0,
            w: (rect.w - 1060.0).max(180.0),
            h: 24.0,
        },
        14,
        Color::from_rgba(185, 202, 224, 255),
        TextAlign::Left,
    );
    if button(rect.x + 430.0, rect.y + 4.0, 92.0, 22.0, "New Graph", false)
        && let Ok(path) = game.create_program_asset("LogAndMove")
    {
        set_open_file_editor_state(state, &path);
    }
    if button(rect.x + 530.0, rect.y + 4.0, 92.0, 22.0, "+ Script", false)
        && let Ok(path) = game.create_luau_script_asset("PlayerController")
    {
        set_open_file_editor_state(state, &path);
    }
    if button(rect.x + 630.0, rect.y + 4.0, 74.0, 22.0, "Attach", false) {
        game.attach_program_template_to_selected("LogAndMove");
    }
    if button(rect.x + 712.0, rect.y + 4.0, 88.0, 22.0, "RTS Order", false) {
        game.attach_program_template_to_selected("RTSOrder");
    }
    if button(
        rect.x + 808.0,
        rect.y + 4.0,
        104.0,
        22.0,
        "Blueprints",
        false,
    ) {
        state.blueprint_picker_open = true;
    }

    let mut y = rect.y + 48.0;
    draw_text(
        "Templates",
        rect.x + 14.0,
        y,
        16.0,
        Color::from_rgba(125, 216, 205, 255),
    );
    y += 22.0;
    let search_rect = RectSpec {
        x: rect.x + 14.0,
        y: y - 16.0,
        w: 360.0,
        h: 24.0,
    };
    draw_graph_template_search_box(state, search_rect);
    y += 32.0;
    let templates = game
        .programming
        .search_templates(&state.graph_template_search);
    for template in templates.iter().take(4) {
        let row = RectSpec {
            x: rect.x + 14.0,
            y: y - 16.0,
            w: 360.0,
            h: 23.0,
        };
        let hovered = contains_mouse(row);
        draw_rect(
            row,
            if hovered {
                Color::from_rgba(46, 57, 70, 255)
            } else {
                Color::from_rgba(31, 36, 46, 255)
            },
        );
        draw_text_fit(
            &template.name,
            RectSpec {
                x: row.x + 8.0,
                y: row.y + 2.0,
                w: 112.0,
                h: row.h - 4.0,
            },
            14,
            Color::from_rgba(226, 235, 244, 255),
            TextAlign::Left,
        );
        draw_text_fit(
            &template.description,
            RectSpec {
                x: row.x + 128.0,
                y: row.y + 3.0,
                w: row.w - 136.0,
                h: row.h - 6.0,
            },
            12,
            Color::from_rgba(150, 164, 186, 255),
            TextAlign::Left,
        );
        if hovered && is_mouse_button_pressed(MouseButton::Left) {
            game.attach_program_template_to_selected(&template.name);
        }
        y += 25.0;
    }

    let mut y2 = rect.y + 48.0;
    draw_text(
        "Runtime",
        rect.x + 430.0,
        y2,
        16.0,
        Color::from_rgba(180, 220, 255, 255),
    );
    y2 += 22.0;
    let events = game.programming.runtime_events.clone();
    for event in events.iter().rev().take(5) {
        draw_text(
            &ellipsize(event, 58),
            rect.x + 390.0,
            y2,
            14.0,
            Color::from_rgba(212, 220, 235, 255),
        );
        y2 += 20.0;
    }
    if game.programming.last_warnings.is_empty() {
        draw_text(
            "Graph validator clean",
            rect.x + 655.0,
            rect.y + 70.0,
            14.0,
            Color::from_rgba(135, 230, 165, 255),
        );
    } else {
        let mut wy = rect.y + 70.0;
        for warning in game.programming.last_warnings.iter().take(4) {
            draw_text(
                &ellipsize(warning, 46),
                rect.x + 655.0,
                wy,
                14.0,
                Color::from_rgba(255, 206, 130, 255),
            );
            wy += 18.0;
        }
    }
    let mut qa_y = rect.y + 126.0;
    draw_text(
        "Quick Actions",
        rect.x + 655.0,
        qa_y,
        16.0,
        Color::from_rgba(125, 216, 205, 255),
    );
    qa_y += 21.0;
    for action in game.programming.quick_actions().iter().take(4) {
        let row = RectSpec {
            x: rect.x + 655.0,
            y: qa_y - 15.0,
            w: (rect.w - 670.0).clamp(220.0, 380.0),
            h: 24.0,
        };
        let hovered = contains_mouse(row);
        draw_rect(
            row,
            if hovered {
                Color::from_rgba(42, 55, 72, 255)
            } else {
                Color::from_rgba(25, 31, 42, 255)
            },
        );
        draw_rectangle(row.x, row.y, 3.0, row.h, ui_accent_2());
        draw_text(
            &ellipsize(&action.label, 18),
            row.x + 10.0,
            qa_y,
            13.0,
            ui_text(),
        );
        draw_text(
            &ellipsize(&action.description, 30),
            row.x + 126.0,
            qa_y,
            12.0,
            ui_text_muted(),
        );
        if hovered && is_mouse_button_pressed(MouseButton::Left) {
            match game.add_quick_action_to_open_graph(action) {
                Ok(ids) if !ids.is_empty() => {
                    state.bottom_tab = BottomTab::Programming;
                    state.graph_pan = (24.0, 24.0);
                }
                Ok(_) => game
                    .console
                    .log("Abre un .mfgraph para insertar accion", "SCRIPT"),
                Err(error) => game
                    .console
                    .log(format!("Quick action no aplicada: {error}"), "WARNING"),
            }
        }
        qa_y += 27.0;
    }
    if !catalog.categories.is_empty() {
        draw_text(
            &ellipsize(
                &format!("Categorias: {}", catalog.categories.join(", ")),
                58,
            ),
            rect.x + 655.0,
            qa_y + 4.0,
            12.0,
            ui_text_muted(),
        );
    }

    let input_x = rect.x + 14.0;
    let mut input_y = rect.y + 214.0;
    draw_text(
        "Input Map",
        input_x,
        input_y,
        16.0,
        Color::from_rgba(180, 220, 255, 255),
    );
    if button(input_x + 92.0, input_y - 16.0, 66.0, 21.0, "Save", false) {
        game.input_map.save().ok();
    }
    input_y += 22.0;
    for action in game.input_map.action_infos().iter().take(8) {
        let bindings = game
            .input_map
            .bindings
            .get(&action.name)
            .cloned()
            .unwrap_or_default();
        let row = RectSpec {
            x: input_x,
            y: input_y - 15.0,
            w: 360.0,
            h: 22.0,
        };
        draw_rect(row, Color::from_rgba(31, 36, 46, 255));
        draw_text(
            &ellipsize(&action.display_name, 13),
            row.x + 8.0,
            input_y,
            14.0,
            Color::from_rgba(230, 235, 246, 255),
        );
        draw_text(
            &ellipsize(&bindings.join(", "), 34),
            row.x + 112.0,
            input_y,
            13.0,
            Color::from_rgba(160, 176, 202, 255),
        );
        if button(row.x + row.w - 45.0, row.y + 1.0, 38.0, 20.0, "+", false) {
            let generated = format!("keyboard:{}", action.name.to_lowercase());
            game.input_map.add_binding(&action.name, generated).ok();
        }
        input_y += 24.0;
    }

    let tab_strip = RectSpec {
        x: rect.x + 390.0,
        y: rect.y + 98.0,
        w: rect.w - 405.0,
        h: 26.0,
    };
    draw_editor_tab_strip(game, state, tab_strip);

    let editor = RectSpec {
        x: rect.x + 390.0,
        y: rect.y + 130.0,
        w: rect.w - 405.0,
        h: rect.h - 140.0,
    };
    if state.script_window_open {
        draw_rect(editor, Color::from_rgba(15, 18, 25, 245));
        draw_rectangle_lines(
            editor.x,
            editor.y,
            editor.w,
            editor.h,
            1.0,
            Color::from_rgba(62, 74, 96, 255),
        );
        draw_text(
            "Editor separado activo. Mueve la ventana flotante para programar scripts o blueprints.",
            editor.x + 16.0,
            editor.y + 32.0,
            14.0,
            Color::from_rgba(170, 185, 210, 255),
        );
        if button(
            editor.x + 16.0,
            editor.y + 52.0,
            112.0,
            24.0,
            "Focus Window",
            false,
        ) {
            state.script_window_open = true;
        }
        return;
    }
    if open_file_extension(game).as_deref() == Some("mfgraph") && !state.code_editor_active {
        draw_visual_graph_editor(game, state, editor);
    } else {
        draw_code_editor(game, state, editor);
    }
}

fn draw_editor_tab_strip(game: &mut Game, state: &mut EditorState, rect: RectSpec) {
    draw_rect(rect, Color::from_rgba(15, 18, 25, 245));
    draw_rectangle_lines(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        1.0,
        Color::from_rgba(54, 64, 82, 255),
    );
    let tabs = game.script_editor.tabs.clone();
    if tabs.is_empty() {
        draw_text(
            "Abre un script, graph, prefab o escena desde el Content Browser.",
            rect.x + 10.0,
            rect.y + 18.0,
            13.0,
            Color::from_rgba(132, 150, 176, 255),
        );
        return;
    }

    let current = game.script_editor.document.path.clone();
    let mut x = rect.x + 4.0;
    let max_x = rect.x + rect.w - 6.0;
    for path in tabs.iter().rev().take(8).rev() {
        let label = crate::engine::script_editor::ScriptEditor::tab_label(path);
        let active = current.as_ref() == Some(path);
        let width = (measure_text(&label, None, 12, 1.0).width + 42.0).clamp(92.0, 168.0);
        if x + width > max_x {
            break;
        }
        if button(
            x,
            rect.y + 3.0,
            width - 24.0,
            20.0,
            &ellipsize(&label, 18),
            active,
        ) && open_project_file_in_editor(game, state, path)
        {
            game.console
                .log(format!("Pestana activa: {label}"), "EDITOR");
        }
        if button(x + width - 23.0, rect.y + 3.0, 20.0, 20.0, "X", false) {
            request_close_script_tab(game, state, Some(path.clone()));
            return;
        }
        x += width + 4.0;
    }
}

#[allow(dead_code)]
fn draw_floating_sprite_window_legacy(game: &mut Game, state: &mut EditorState, sw: f32, sh: f32) {
    if !state.sprite_window_open {
        return;
    }
    state.sprite_window_rect = update_floating_drag(
        state,
        FloatingWindowKind::Sprite,
        state.sprite_window_rect,
        sw,
        sh,
    );
    let rect = clamp_window_rect(state.sprite_window_rect, sw, sh);
    state.sprite_window_rect = rect;
    draw_floating_shell(rect, "Sprite Editor");
    if button(
        rect.x + rect.w - 122.0,
        rect.y + 5.0,
        58.0,
        22.0,
        "Dock",
        false,
    ) {
        state.sprite_window_open = false;
        state.show_console = true;
        state.bottom_tab = BottomTab::Sprites;
    }
    if button(rect.x + rect.w - 56.0, rect.y + 5.0, 42.0, 22.0, "X", false) {
        state.sprite_window_open = false;
        return;
    }
    draw_sprite_editor_panel(
        game,
        state,
        RectSpec {
            x: rect.x + 8.0,
            y: rect.y + 38.0,
            w: rect.w - 16.0,
            h: rect.h - 46.0,
        },
    );
}

#[allow(dead_code)]
fn draw_python_tools_window_legacy(game: &mut Game, state: &mut EditorState, sw: f32, sh: f32) {
    if !state.python_tools_open {
        return;
    }
    state.python_tools_rect = update_floating_drag(
        state,
        FloatingWindowKind::PythonTools,
        state.python_tools_rect,
        sw,
        sh,
    );
    let rect = clamp_window_rect(state.python_tools_rect, sw, sh);
    state.python_tools_rect = rect;
    draw_floating_shell(rect, "Python Automation Tools");
    if button(rect.x + rect.w - 56.0, rect.y + 5.0, 42.0, 22.0, "X", false) {
        state.python_tools_open = false;
        return;
    }

    let host = PythonAutomationHost::new(&game.project_path);
    if button(
        rect.x + 14.0,
        rect.y + 42.0,
        106.0,
        22.0,
        "Install tools",
        false,
    ) {
        match host.install_builtin_tools() {
            Ok(paths) => game.console.log(
                format!("Python tools instalados/verificados: {}", paths.len()),
                "PYTHON",
            ),
            Err(error) => game.console.log(format!("Python tools: {error}"), "ERROR"),
        }
    }
    if button(
        rect.x + 128.0,
        rect.y + 42.0,
        112.0,
        22.0,
        "Scene report",
        false,
    ) {
        run_python_tool(game, state, "scene_report");
    }
    if button(
        rect.x + 248.0,
        rect.y + 42.0,
        112.0,
        22.0,
        "Import drop",
        false,
    ) {
        let report = batch_import_assets(&game.project_path, "ImportDrop", "assets/imported");
        log_python_batch(game, "Batch import", report);
        refresh_assets(game);
    }
    if button(
        rect.x + 368.0,
        rect.y + 42.0,
        116.0,
        22.0,
        "Convert sprites",
        false,
    ) {
        let report = batch_convert_sprites(
            &game.project_path,
            "assets/sprites",
            "assets/sprites/converted",
        );
        log_python_batch(game, "Sprite conversion", report);
        refresh_assets(game);
    }
    if button(
        rect.x + 492.0,
        rect.y + 42.0,
        112.0,
        22.0,
        "Build atlases",
        false,
    ) {
        let report = generate_paged_sprite_atlases(
            &game.project_path,
            "assets/sprites",
            "assets/sprite_atlases",
            2048,
            2,
        );
        log_python_batch(game, "Atlas generation", report);
        refresh_assets(game);
    }

    draw_text(
        "Herramientas declarativas descubiertas en project/tools",
        rect.x + 16.0,
        rect.y + 90.0,
        15.0,
        ui_text(),
    );
    let tools = host.discover().unwrap_or_default();
    let mut y = rect.y + 118.0;
    for tool in tools.iter().take(14) {
        let row = RectSpec {
            x: rect.x + 14.0,
            y: y - 17.0,
            w: rect.w - 28.0,
            h: 26.0,
        };
        draw_rect(row, Color::from_rgba(20, 27, 39, 245));
        draw_text(
            &ellipsize(&format!("{}  —  {}", tool.label, tool.description), 82),
            row.x + 8.0,
            y,
            13.0,
            ui_text_muted(),
        );
        if button(row.x + row.w - 58.0, row.y + 2.0, 52.0, 22.0, "Run", false) {
            run_python_tool(game, state, &tool.id);
        }
        y += 29.0;
        if y > rect.y + rect.h - 24.0 {
            break;
        }
    }
}

fn log_python_batch(game: &mut Game, label: &str, report: io::Result<PythonBatchReport>) {
    match report {
        Ok(report) => game.console.log(
            format!(
                "{label}: {} procesados, {} omitidos, {} salidas, {} warnings",
                report.processed,
                report.skipped,
                report.output_files.len(),
                report.warnings.len()
            ),
            if report.warnings.is_empty() {
                "PYTHON"
            } else {
                "WARNING"
            },
        ),
        Err(error) => game.console.log(format!("{label}: {error}"), "ERROR"),
    }
}

fn draw_floating_script_window(game: &mut Game, state: &mut EditorState, sw: f32, sh: f32) {
    if !state.script_window_open {
        return;
    }
    state.script_window_rect = update_floating_drag(
        state,
        FloatingWindowKind::Script,
        state.script_window_rect,
        sw,
        sh,
    );
    let rect = clamp_window_rect(state.script_window_rect, sw, sh);
    state.script_window_rect = rect;
    draw_floating_shell(rect, "Detached Programming");
    if button(
        rect.x + rect.w - 212.0,
        rect.y + 5.0,
        58.0,
        22.0,
        "Dock",
        false,
    ) {
        state.script_window_open = false;
        state.show_console = true;
        state.bottom_tab = BottomTab::Programming;
    }
    if open_file_extension(game).as_deref() == Some("mfgraph")
        && button(
            rect.x + rect.w - 148.0,
            rect.y + 5.0,
            86.0,
            22.0,
            if state.code_editor_active {
                "Nodes"
            } else {
                "JSON"
            },
            false,
        )
    {
        state.code_editor_active = !state.code_editor_active;
    }
    if button(rect.x + rect.w - 56.0, rect.y + 5.0, 42.0, 22.0, "X", false) {
        request_close_script_tab(game, state, game.script_editor.document.path.clone());
    }
    draw_text(
        &ellipsize(
            game.script_editor
                .document
                .path
                .as_ref()
                .and_then(|path| path.file_name())
                .and_then(|value| value.to_str())
                .unwrap_or("No file open"),
            42,
        ),
        rect.x + 184.0,
        rect.y + 23.0,
        13.0,
        ui_text_muted(),
    );
    let content = RectSpec {
        x: rect.x + 10.0,
        y: rect.y + 38.0,
        w: rect.w - 20.0,
        h: rect.h - 48.0,
    };
    if open_file_extension(game).as_deref() == Some("mfgraph") && !state.code_editor_active {
        draw_visual_graph_editor(game, state, content);
    } else {
        draw_code_editor(game, state, content);
    }
}

fn draw_floating_sprite_window(game: &mut Game, state: &mut EditorState, sw: f32, sh: f32) {
    if !state.sprite_window_open {
        return;
    }
    state.sprite_window_rect = update_floating_drag(
        state,
        FloatingWindowKind::Sprite,
        state.sprite_window_rect,
        sw,
        sh,
    );
    let rect = clamp_window_rect(state.sprite_window_rect, sw, sh);
    state.sprite_window_rect = rect;
    draw_floating_shell(rect, "Sprite Studio 2D");
    if button(
        rect.x + rect.w - 134.0,
        rect.y + 5.0,
        68.0,
        22.0,
        "Dock",
        false,
    ) {
        state.sprite_window_open = false;
        state.show_console = true;
        state.bottom_tab = BottomTab::Sprites;
        return;
    }
    if button(rect.x + rect.w - 58.0, rect.y + 5.0, 42.0, 22.0, "X", false) {
        state.sprite_window_open = false;
        return;
    }
    draw_sprite_editor_panel(
        game,
        state,
        RectSpec {
            x: rect.x + 8.0,
            y: rect.y + 38.0,
            w: rect.w - 16.0,
            h: rect.h - 46.0,
        },
    );
}

fn draw_python_tools_window(game: &mut Game, state: &mut EditorState, sw: f32, sh: f32) {
    if !state.python_tools_open {
        return;
    }
    state.python_tools_rect = update_floating_drag(
        state,
        FloatingWindowKind::PythonTools,
        state.python_tools_rect,
        sw,
        sh,
    );
    let rect = clamp_window_rect(state.python_tools_rect, sw, sh);
    state.python_tools_rect = rect;
    draw_floating_shell(rect, "Python Tools");
    if button(
        rect.x + rect.w - 178.0,
        rect.y + 5.0,
        108.0,
        22.0,
        "Install/Refresh",
        false,
    ) {
        let host = PythonAutomationHost::new(&game.project_path);
        match host.install_builtin_tools() {
            Ok(files) => game.console.log(
                format!("Python tools instaladas: {} archivos", files.len()),
                "PYTHON",
            ),
            Err(error) => game
                .console
                .log(format!("Instalar Python tools: {error}"), "ERROR"),
        }
    }
    if button(rect.x + rect.w - 60.0, rect.y + 5.0, 44.0, 22.0, "X", false) {
        state.python_tools_open = false;
        return;
    }

    let host = PythonAutomationHost::new(&game.project_path);
    let version = host
        .interpreter_version()
        .unwrap_or_else(|_| "Python no disponible".to_string());
    let tools = host.discover().unwrap_or_default();
    draw_text(
        &format!("{version} | {} herramientas confiables", tools.len()),
        rect.x + 16.0,
        rect.y + 56.0,
        14.0,
        ui_text_muted(),
    );
    let list = RectSpec {
        x: rect.x + 12.0,
        y: rect.y + 70.0,
        w: rect.w - 24.0,
        h: rect.h - 82.0,
    };
    draw_rect(list, Color::from_rgba(13, 16, 23, 245));
    draw_rectangle_lines(list.x, list.y, list.w, list.h, 1.0, ui_line_soft());
    if tools.is_empty() {
        draw_text(
            "Pulsa Install/Refresh para instalar la suite de automatización.",
            list.x + 16.0,
            list.y + 30.0,
            14.0,
            ui_text_muted(),
        );
        return;
    }
    let mut y = list.y + 10.0;
    let max_rows = ((list.h - 16.0) / 46.0).max(1.0) as usize;
    for tool in tools.into_iter().take(max_rows) {
        let row = RectSpec {
            x: list.x + 8.0,
            y,
            w: list.w - 16.0,
            h: 40.0,
        };
        draw_rect(row, Color::from_rgba(23, 29, 40, 245));
        draw_text(
            &ellipsize(&tool.label, 48),
            row.x + 10.0,
            row.y + 16.0,
            13.0,
            ui_text(),
        );
        draw_text(
            &ellipsize(&format!("{} — {}", tool.menu_path, tool.description), 78),
            row.x + 10.0,
            row.y + 32.0,
            11.0,
            ui_text_muted(),
        );
        if button(row.x + row.w - 74.0, row.y + 8.0, 62.0, 24.0, "Run", false) {
            run_python_tool(game, state, &tool.id);
        }
        y += 46.0;
    }
}

fn request_close_script_tab(game: &mut Game, state: &mut EditorState, path: Option<PathBuf>) {
    let Some(path) = path else {
        state.script_window_open = false;
        state.code_editor_active = false;
        game.console
            .log("Editor de scripts cerrado sin archivo activo", "EDITOR");
        return;
    };
    if game.script_editor.is_dirty(&path) {
        state.pending_script_close = Some(path);
        game.console.log(
            "Cierre de script pendiente: hay cambios sin guardar",
            "EDITOR",
        );
        return;
    }
    close_script_tab_with_choice(game, state, path, CloseDocumentChoice::Discard);
}

fn close_script_tab_with_choice(
    game: &mut Game,
    state: &mut EditorState,
    path: PathBuf,
    choice: CloseDocumentChoice,
) {
    match game.script_editor.close_tab_with_choice(&path, choice) {
        Ok(outcome) if outcome.cancelled => {
            state.pending_script_close = None;
            game.console.log("Cierre de script cancelado", "EDITOR");
        }
        Ok(outcome) => {
            state.pending_script_close = None;
            game.event_bus.emit(
                "ScriptClosed",
                json!({"path": path.to_string_lossy(), "saved": outcome.saved}),
            );
            if let Some(opened) = outcome.active {
                set_open_file_editor_state(state, &opened);
                game.console.log(
                    format!("Pestana cerrada; activo: {}", opened.display()),
                    "EDITOR",
                );
            } else {
                state.code_editor_active = false;
                state.code_cursor_line = 0;
                state.code_cursor_column = 0;
                state.code_scroll_line = 0;
                state.script_window_open = false;
                game.console
                    .log("Pestana cerrada; no quedan documentos abiertos", "EDITOR");
            }
        }
        Err(error) => {
            state.pending_script_close = None;
            game.console
                .log(format!("No se pudo cerrar pestana: {error}"), "ERROR");
        }
    }
}

fn draw_script_close_prompt(game: &mut Game, state: &mut EditorState, sw: f32, sh: f32) {
    let Some(path) = state.pending_script_close.clone() else {
        return;
    };
    let rect = RectSpec {
        x: (sw - 460.0) * 0.5,
        y: (sh - 170.0) * 0.5,
        w: 460.0,
        h: 170.0,
    };
    draw_rectangle(0.0, 0.0, sw, sh, Color::from_rgba(0, 0, 0, 130));
    draw_panel_chrome(rect, "Cambios sin guardar", "Script Editor");
    draw_text(
        &ellipsize(
            &format!(
                "{} tiene cambios sin guardar.",
                path.file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or("Documento")
            ),
            58,
        ),
        rect.x + 18.0,
        rect.y + 58.0,
        15.0,
        ui_text(),
    );
    draw_text(
        "Guardar antes de cerrar, descartar cambios o cancelar.",
        rect.x + 18.0,
        rect.y + 82.0,
        13.0,
        ui_text_muted(),
    );
    let y = rect.y + rect.h - 44.0;
    if button(rect.x + 18.0, y, 96.0, 26.0, "Guardar", false) {
        close_script_tab_with_choice(game, state, path, CloseDocumentChoice::Save);
    } else if button(rect.x + 124.0, y, 104.0, 26.0, "Descartar", false) {
        close_script_tab_with_choice(game, state, path, CloseDocumentChoice::Discard);
    } else if button(rect.x + rect.w - 116.0, y, 96.0, 26.0, "Cancelar", false) {
        state.pending_script_close = None;
        game.console.log("Cierre de script cancelado", "EDITOR");
    }
}

fn editor_preferences_path(game: &Game) -> PathBuf {
    game.project_paths.settings.join("editor_preferences.json")
}

fn load_editor_preferences(game: &Game) -> EditorPreferences {
    let path = editor_preferences_path(game);
    let Ok(value) = AssetTools::read_json(&path) else {
        return EditorPreferences::default();
    };
    EditorPreferences {
        prefer_vscode: value
            .get("prefer_vscode")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        external_editor_command: value
            .get("external_editor_command")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("code")
            .to_string(),
        open_external_on_script: value
            .get("open_external_on_script")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        keyboard_layout_hint: value
            .get("keyboard_layout_hint")
            .and_then(Value::as_str)
            .unwrap_or("international")
            .to_string(),
    }
}

fn save_editor_preferences(game: &mut Game, state: &EditorState) {
    let path = editor_preferences_path(game);
    let prefs = &state.editor_preferences;
    let value = json!({
        "prefer_vscode": prefs.prefer_vscode,
        "external_editor_command": prefs.external_editor_command,
        "open_external_on_script": prefs.open_external_on_script,
        "keyboard_layout_hint": prefs.keyboard_layout_hint,
    });
    match AssetTools::write_json(&path, &value) {
        Ok(()) => game.console.log(
            format!("Preferencias guardadas: {}", path.display()),
            "EDITOR",
        ),
        Err(error) => game.console.log(
            format!("No se pudieron guardar preferencias: {error}"),
            "ERROR",
        ),
    }
}

fn open_current_file_in_external_editor(game: &mut Game, state: &EditorState) {
    let Some(path) = game.script_editor.document.path.clone() else {
        game.console.log(
            "No hay archivo abierto para enviar al editor externo",
            "WARNING",
        );
        return;
    };
    open_file_in_external_editor(game, state, &path);
}

fn open_file_in_external_editor(game: &mut Game, state: &EditorState, path: &Path) {
    let line = state.code_cursor_line + 1;
    let column = line_visual_column(
        game.script_editor
            .lines
            .get(state.code_cursor_line)
            .map(String::as_str)
            .unwrap_or(""),
        state.code_cursor_column,
    ) + 1;
    let command = if state.editor_preferences.prefer_vscode {
        "code"
    } else {
        state.editor_preferences.external_editor_command.as_str()
    };
    let launch = Command::new(command)
        .arg("-g")
        .arg(format!("{}:{line}:{column}", path.display()))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();

    match launch {
        Ok(_) => game.console.log(
            format!("Abriendo editor externo: {}", path.display()),
            "EDITOR",
        ),
        Err(error) => {
            if open_in_default_application(path).is_ok() {
                game.console.log(
                    format!("Abriendo con aplicación del sistema: {}", path.display()),
                    "EDITOR",
                );
                return;
            }
            game.console.log(
                format!("No se pudo abrir editor externo ({command}): {error}"),
                "ERROR",
            );
        }
    }
}

fn draw_preferences_window(game: &mut Game, state: &mut EditorState, sw: f32, sh: f32) {
    if !state.preferences_open {
        return;
    }
    let rect = RectSpec {
        x: (sw - 560.0).max(20.0) * 0.5,
        y: (sh - 360.0).max(20.0) * 0.5,
        w: 560.0_f32.min(sw - 32.0),
        h: 360.0_f32.min(sh - 32.0),
    };
    draw_rectangle(0.0, 0.0, sw, sh, Color::from_rgba(0, 0, 0, 145));
    draw_panel_chrome(rect, "Preferences", "editor and input");
    if button(rect.x + rect.w - 58.0, rect.y + 8.0, 42.0, 22.0, "X", false) {
        state.preferences_open = false;
        return;
    }

    let mut y = rect.y + 64.0;
    draw_text("Script Editor", rect.x + 18.0, y, 17.0, ui_text());
    y += 28.0;
    if button(
        rect.x + 18.0,
        y - 18.0,
        152.0,
        24.0,
        "Prefer VS Code",
        state.editor_preferences.prefer_vscode,
    ) {
        state.editor_preferences.prefer_vscode = !state.editor_preferences.prefer_vscode;
        save_editor_preferences(game, state);
    }
    if button(
        rect.x + 182.0,
        y - 18.0,
        184.0,
        24.0,
        "Open scripts externally",
        state.editor_preferences.open_external_on_script,
    ) {
        state.editor_preferences.open_external_on_script =
            !state.editor_preferences.open_external_on_script;
        save_editor_preferences(game, state);
    }
    if button(rect.x + 378.0, y - 18.0, 124.0, 24.0, "Open Current", false) {
        open_current_file_in_external_editor(game, state);
    }

    y += 42.0;
    draw_rect(
        RectSpec {
            x: rect.x + 18.0,
            y: y - 20.0,
            w: rect.w - 36.0,
            h: 48.0,
        },
        Color::from_rgba(14, 18, 27, 240),
    );
    draw_text_fit(
        &format!(
            "External command: {}",
            state.editor_preferences.external_editor_command
        ),
        RectSpec {
            x: rect.x + 30.0,
            y: y - 15.0,
            w: rect.w - 60.0,
            h: 22.0,
        },
        13,
        ui_text_muted(),
        TextAlign::Left,
    );
    draw_text_fit(
        "Uses `code -g file:line:column`; macOS falls back to Visual Studio Code.app.",
        RectSpec {
            x: rect.x + 30.0,
            y: y + 6.0,
            w: rect.w - 60.0,
            h: 20.0,
        },
        12,
        ui_text_muted(),
        TextAlign::Left,
    );

    y += 74.0;
    draw_text("Keyboard", rect.x + 18.0, y, 17.0, ui_text());
    y += 28.0;
    draw_text_fit(
        "International input is enabled: text input uses Unicode characters, Cmd/Ctrl+/ also accepts Cmd/Ctrl+Shift+7 for Spanish/Latin layouts.",
        RectSpec {
            x: rect.x + 18.0,
            y: y - 18.0,
            w: rect.w - 36.0,
            h: 42.0,
        },
        13,
        ui_text_muted(),
        TextAlign::Left,
    );
    y += 56.0;
    draw_text("Blueprint Navigation", rect.x + 18.0, y, 17.0, ui_text());
    y += 28.0;
    draw_text_fit(
        "Hold right mouse anywhere over the Blueprint editor to pan, middle mouse still pans, wheel scrolls vertically, Shift+wheel scrolls horizontally.",
        RectSpec {
            x: rect.x + 18.0,
            y: y - 18.0,
            w: rect.w - 36.0,
            h: 44.0,
        },
        13,
        ui_text_muted(),
        TextAlign::Left,
    );
}

fn draw_floating_play_window(game: &mut Game, state: &mut EditorState, sw: f32, sh: f32) {
    if !state.play_window_open {
        return;
    }
    state.play_window_rect = update_floating_drag(
        state,
        FloatingWindowKind::Play,
        state.play_window_rect,
        sw,
        sh,
    );
    let rect = clamp_window_rect(state.play_window_rect, sw, sh);
    state.play_window_rect = rect;
    draw_floating_shell(rect, "Play Window");
    let active_external = state.external_play_child.is_some();
    if button(
        rect.x + rect.w - 156.0,
        rect.y + 5.0,
        86.0,
        22.0,
        "Stop Play",
        false,
    ) {
        toggle_play_mode(game, state);
        return;
    }
    if button(
        rect.x + rect.w - 62.0,
        rect.y + 5.0,
        46.0,
        22.0,
        "Hide",
        false,
    ) {
        state.play_window_open = false;
    }
    let content = RectSpec {
        x: rect.x + 10.0,
        y: rect.y + 38.0,
        w: rect.w - 20.0,
        h: rect.h - 48.0,
    };
    if active_external {
        draw_rect(content, Color::from_rgba(12, 15, 21, 255));
        draw_text(
            "Runtime separado activo",
            content.x + 18.0,
            content.y + 34.0,
            18.0,
            Color::from_rgba(150, 220, 255, 255),
        );
        let path = state
            .external_play_path
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "build debug".to_string());
        draw_text(
            &ellipsize(&path, 92),
            content.x + 18.0,
            content.y + 62.0,
            13.0,
            Color::from_rgba(170, 185, 210, 255),
        );
        draw_text(
            "Cierra esta sesión con Stop Play o cierra la ventana runtime.",
            content.x + 18.0,
            content.y + 88.0,
            13.0,
            Color::from_rgba(132, 150, 176, 255),
        );
        return;
    }
    if game.mode == "PLAY" {
        let viewport = Viewport {
            rect: content,
            tile: game.grid.tile_size as f32,
            zoom: game.camera.zoom as f32,
            camera_x: game.camera.x as f32,
            camera_y: game.camera.y as f32,
        };
        draw_world_player(game, viewport);
        draw_rectangle_lines(
            content.x,
            content.y,
            content.w,
            content.h,
            1.0,
            Color::from_rgba(88, 140, 210, 255),
        );
    } else {
        draw_rect(content, Color::from_rgba(12, 15, 21, 255));
        draw_text(
            "Play no está activo. Pulsa Play para abrir runtime.",
            content.x + 18.0,
            content.y + 34.0,
            14.0,
            Color::from_rgba(170, 185, 210, 255),
        );
    }
}

fn draw_blueprint_picker_window(game: &mut Game, state: &mut EditorState, sw: f32, sh: f32) {
    if !state.blueprint_picker_open {
        return;
    }
    state.blueprint_picker_rect = update_floating_drag(
        state,
        FloatingWindowKind::BlueprintPicker,
        state.blueprint_picker_rect,
        sw,
        sh,
    );
    let rect = clamp_window_rect(state.blueprint_picker_rect, sw, sh);
    state.blueprint_picker_rect = rect;
    draw_floating_shell(rect, "Blueprint Library");
    if button(rect.x + rect.w - 58.0, rect.y + 5.0, 42.0, 22.0, "X", false) {
        state.blueprint_picker_open = false;
    }

    let content = RectSpec {
        x: rect.x + 12.0,
        y: rect.y + 42.0,
        w: rect.w - 24.0,
        h: rect.h - 54.0,
    };
    draw_rect(content, Color::from_rgba(13, 16, 23, 245));
    let search = RectSpec {
        x: content.x + 12.0,
        y: content.y + 12.0,
        w: content.w - 24.0,
        h: 28.0,
    };
    draw_graph_template_search_box(state, search);

    let templates = game
        .programming
        .search_templates(&state.graph_template_search);
    let graph_query = state.graph_template_search.to_lowercase();
    let graph_assets = game
        .asset_database
        .assets
        .values()
        .filter(|asset| {
            asset.asset_type == "VisualGraph"
                && (graph_query.is_empty()
                    || asset.name.to_lowercase().contains(&graph_query)
                    || asset.relative_path.to_lowercase().contains(&graph_query))
        })
        .cloned()
        .collect::<Vec<_>>();

    let left = RectSpec {
        x: content.x + 12.0,
        y: content.y + 58.0,
        w: content.w * 0.48,
        h: content.h - 70.0,
    };
    let right = RectSpec {
        x: left.x + left.w + 16.0,
        y: left.y,
        w: content.w - left.w - 40.0,
        h: left.h,
    };
    draw_text(
        "Templates",
        left.x,
        left.y,
        16.0,
        Color::from_rgba(126, 205, 255, 255),
    );
    let mut y = left.y + 26.0;
    for template in templates.iter().take(9) {
        let row = RectSpec {
            x: left.x,
            y: y - 16.0,
            w: left.w,
            h: 42.0,
        };
        let hovered = contains_mouse(row);
        draw_rect(
            row,
            if hovered {
                Color::from_rgba(45, 55, 72, 255)
            } else {
                Color::from_rgba(24, 30, 42, 255)
            },
        );
        draw_text(
            &ellipsize(&template.name, 28),
            row.x + 9.0,
            y,
            14.0,
            Color::from_rgba(232, 238, 248, 255),
        );
        draw_text(
            &ellipsize(&template.description, 52),
            row.x + 9.0,
            y + 18.0,
            11.0,
            Color::from_rgba(150, 166, 192, 255),
        );
        if button(row.x + row.w - 112.0, row.y + 8.0, 48.0, 22.0, "New", false)
            && let Ok(path) = game.create_program_asset(&template.name)
        {
            set_open_file_editor_state(state, &path);
            state.script_window_open = true;
            state.blueprint_picker_open = false;
        }
        if button(row.x + row.w - 58.0, row.y + 8.0, 52.0, 22.0, "Use", false) {
            game.attach_program_template_to_selected(&template.name);
        }
        y += 48.0;
    }

    draw_text(
        "Project Graphs",
        right.x,
        right.y,
        16.0,
        Color::from_rgba(126, 205, 255, 255),
    );
    let mut y = right.y + 26.0;
    for asset in graph_assets.iter().take(10) {
        let row = RectSpec {
            x: right.x,
            y: y - 16.0,
            w: right.w,
            h: 36.0,
        };
        let hovered = contains_mouse(row);
        draw_rect(
            row,
            if hovered {
                Color::from_rgba(45, 55, 72, 255)
            } else {
                Color::from_rgba(24, 30, 42, 255)
            },
        );
        draw_text(
            &ellipsize(&asset.name, 28),
            row.x + 9.0,
            y,
            14.0,
            Color::from_rgba(232, 238, 248, 255),
        );
        draw_text(
            &ellipsize(&asset.relative_path, 54),
            row.x + 9.0,
            y + 16.0,
            11.0,
            Color::from_rgba(150, 166, 192, 255),
        );
        if button(row.x + row.w - 64.0, row.y + 7.0, 54.0, 22.0, "Open", false) {
            open_project_file_in_editor(game, state, asset.relative_path.clone());
            state.script_window_open = true;
            state.blueprint_picker_open = false;
        }
        y += 40.0;
    }
}

fn draw_floating_shell(rect: RectSpec, title: &str) {
    draw_surface(rect, true);
    draw_gradient_rect(
        RectSpec {
            x: rect.x,
            y: rect.y,
            w: rect.w,
            h: 33.0,
        },
        Color::from_rgba(39, 51, 70, 255),
        Color::from_rgba(24, 31, 44, 255),
    );
    draw_rectangle(rect.x, rect.y + 31.0, rect.w, 2.0, ui_accent_2());
    draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 1.0, ui_line());
    draw_text(title, rect.x + 16.0, rect.y + 18.0, 13.0, ui_text());
}

fn update_floating_drag(
    state: &mut EditorState,
    kind: FloatingWindowKind,
    mut rect: RectSpec,
    sw: f32,
    sh: f32,
) -> RectSpec {
    let (mx, my) = mouse_position();
    let title_bar = RectSpec {
        x: rect.x,
        y: rect.y,
        w: (rect.w - 220.0).max(80.0),
        h: 32.0,
    };
    if contains(title_bar, mx, my) && is_mouse_button_pressed(MouseButton::Left) {
        state.floating_drag = Some((kind, mx - rect.x, my - rect.y));
    }
    if is_mouse_button_down(MouseButton::Left)
        && let Some((drag_kind, ox, oy)) = state.floating_drag
        && drag_kind == kind
    {
        rect.x = mx - ox;
        rect.y = my - oy;
    }
    if is_mouse_button_released(MouseButton::Left)
        && state
            .floating_drag
            .is_some_and(|(drag_kind, _, _)| drag_kind == kind)
    {
        state.floating_drag = None;
    }
    clamp_window_rect(rect, sw, sh)
}

fn clamp_window_rect(mut rect: RectSpec, sw: f32, sh: f32) -> RectSpec {
    rect.w = rect.w.clamp(420.0, (sw - 24.0).max(420.0));
    rect.h = rect.h.clamp(260.0, (sh - 24.0).max(260.0));
    rect.x = rect.x.clamp(8.0, (sw - rect.w - 8.0).max(8.0));
    rect.y = rect.y.clamp(8.0, (sh - rect.h - 8.0).max(8.0));
    rect
}

fn pointer_over_floating_windows(state: &EditorState, mx: f32, my: f32) -> bool {
    (state.script_window_open && contains(state.script_window_rect, mx, my))
        || (state.play_window_open && contains(state.play_window_rect, mx, my))
        || (state.blueprint_picker_open && contains(state.blueprint_picker_rect, mx, my))
        || (state.sprite_window_open && contains(state.sprite_window_rect, mx, my))
        || (state.python_tools_open && contains(state.python_tools_rect, mx, my))
}

fn open_file_extension(game: &Game) -> Option<String> {
    game.script_editor
        .document
        .path
        .as_ref()
        .and_then(|path| path.extension())
        .and_then(|value| value.to_str())
        .map(ToString::to_string)
}

fn draw_visual_graph_editor(game: &mut Game, state: &mut EditorState, rect: RectSpec) {
    draw_surface(rect, false);
    draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 1.0, ui_line_soft());
    let title = game
        .script_editor
        .document
        .path
        .as_ref()
        .and_then(|path| path.file_name())
        .and_then(|value| value.to_str())
        .unwrap_or("VisualGraph");
    draw_text(
        &format!("Visual Graph: {title}"),
        rect.x + 10.0,
        rect.y + 20.0,
        16.0,
        ui_text(),
    );
    let button_y = rect.y + 5.0;
    if button(
        rect.x + rect.w - 522.0,
        button_y,
        62.0,
        21.0,
        "+ Log",
        false,
    ) {
        game.add_node_to_open_graph("Log").ok();
    }
    if button(
        rect.x + rect.w - 454.0,
        button_y,
        70.0,
        21.0,
        "+ Move",
        false,
    ) {
        game.add_node_to_open_graph("Move").ok();
    }
    if button(
        rect.x + rect.w - 378.0,
        button_y,
        78.0,
        21.0,
        "+ Health",
        false,
    ) {
        game.add_node_to_open_graph("SetHealth").ok();
    }
    if button(
        rect.x + rect.w - 294.0,
        button_y,
        58.0,
        21.0,
        "+ Vel",
        false,
    ) {
        game.add_node_to_open_graph("SetVelocity").ok();
    }
    let frame_graph_requested = button(
        rect.x + rect.w - 230.0,
        button_y,
        54.0,
        21.0,
        "Frame",
        false,
    );
    if button(rect.x + rect.w - 170.0, button_y, 72.0, 21.0, "Save", false)
        && let Err(error) = game.save_open_file()
    {
        game.console
            .log(format!("No se pudo guardar graph: {error}"), "ERROR");
    }
    if button(
        rect.x + rect.w - 92.0,
        button_y,
        78.0,
        21.0,
        "Validate",
        false,
    ) {
        if game.script_editor.validate() {
            game.console.log("Graph visual validado", "SCRIPT");
        } else if let Some(error) = &game.script_editor.document.syntax_error {
            game.console.log(error.clone(), "ERROR");
        }
    }

    let canvas = RectSpec {
        x: rect.x + 8.0,
        y: rect.y + 34.0,
        w: rect.w - 246.0,
        h: rect.h - 44.0,
    };
    let details = RectSpec {
        x: canvas.x + canvas.w + 10.0,
        y: canvas.y,
        w: rect.w - canvas.w - 26.0,
        h: canvas.h,
    };
    draw_graph_canvas_background(canvas, state.graph_pan);
    draw_text_fit(
        "RMB drag pan | MMB pan | Shift+wheel horizontal",
        RectSpec {
            x: canvas.x + 12.0,
            y: canvas.y + canvas.h - 26.0,
            w: canvas.w - 24.0,
            h: 18.0,
        },
        12,
        ui_text_muted(),
        TextAlign::Left,
    );
    let Some(view) = game.current_visual_graph_view() else {
        draw_text(
            "Graph invalido o vacio.",
            canvas.x + 16.0,
            canvas.y + 30.0,
            14.0,
            Color::from_rgba(255, 166, 140, 255),
        );
        return;
    };
    if frame_graph_requested {
        frame_graph_nodes(state, canvas, &view);
    }
    let graph_navigation_rect = RectSpec {
        x: canvas.x,
        y: canvas.y,
        w: rect.x + rect.w - canvas.x - 8.0,
        h: canvas.h,
    };
    update_graph_canvas_pan(state, graph_navigation_rect);

    for connection in &view.connections {
        let Some(from) = view.nodes.iter().find(|node| node.id == connection.from) else {
            continue;
        };
        let Some(to) = view.nodes.iter().find(|node| node.id == connection.to) else {
            continue;
        };
        let start = graph_output_pin_pos_for(canvas, state.graph_pan, from, &connection.pin);
        let end = graph_input_pin_pos(canvas, state.graph_pan, to);
        draw_graph_wire(canvas, start, end, Color::from_rgba(112, 184, 255, 255));
    }
    if let Some(from_id) = &state.graph_connect_from
        && let Some(from) = view.nodes.iter().find(|node| &node.id == from_id)
    {
        draw_graph_wire(
            canvas,
            graph_output_pin_pos_for(canvas, state.graph_pan, from, &state.graph_connect_pin),
            mouse_position(),
            Color::from_rgba(255, 210, 115, 255),
        );
    }

    let graph_mouse_released = is_mouse_button_released(MouseButton::Left);
    let mut graph_connection_resolved = false;
    if graph_mouse_released {
        state.graph_drag_node = None;
    }
    for node in &view.nodes {
        let node_rect = graph_node_rect(canvas, state.graph_pan, node);
        if !rects_intersect(canvas, node_rect) {
            continue;
        }
        let input_pin = pin_rect(graph_input_pin_pos(canvas, state.graph_pan, node));
        let hovered = contains_mouse(node_rect);
        let selected = state.graph_selected_node.as_deref() == Some(node.id.as_str());
        draw_graph_node(node_rect, node, selected, hovered);
        draw_pin(input_pin, Color::from_rgba(92, 182, 255, 255));
        for pin in &node.output_pins {
            let output_pin = pin_rect(graph_output_pin_pos_for(canvas, state.graph_pan, node, pin));
            draw_pin(output_pin, graph_pin_color(pin));
            draw_text(
                pin,
                output_pin.x - 12.0,
                output_pin.y - 3.0,
                9.0,
                Color::from_rgba(186, 198, 218, 255),
            );
        }

        let mut output_clicked = false;
        for pin in &node.output_pins {
            let output_pin = pin_rect(graph_output_pin_pos_for(canvas, state.graph_pan, node, pin));
            if !state.graph_panning
                && contains_mouse(output_pin)
                && is_mouse_button_pressed(MouseButton::Left)
            {
                state.graph_connect_from = Some(node.id.clone());
                state.graph_connect_pin = pin.clone();
                state.graph_selected_node = Some(node.id.clone());
                output_clicked = true;
            }
        }

        if output_clicked {
        } else if contains_mouse(input_pin)
            && !state.graph_panning
            && (is_mouse_button_pressed(MouseButton::Left) || graph_mouse_released)
        {
            if let Some(from) = state.graph_connect_from.clone()
                && game
                    .connect_open_graph_nodes_on_pin(&from, &node.id, &state.graph_connect_pin)
                    .unwrap_or(false)
            {
                state.graph_connect_from = None;
                state.graph_connect_pin.clear();
                graph_connection_resolved = true;
            }
            state.graph_selected_node = Some(node.id.clone());
        } else if hovered && !state.graph_panning && is_mouse_button_pressed(MouseButton::Left) {
            let mouse = mouse_position();
            state.graph_selected_node = Some(node.id.clone());
            state.graph_drag_node = Some(node.id.clone());
            state.graph_drag_offset = (mouse.0 - node_rect.x, mouse.1 - node_rect.y);
        }
    }

    if is_mouse_button_down(MouseButton::Left)
        && let Some(node_id) = state.graph_drag_node.clone()
    {
        let mouse = mouse_position();
        let x = (mouse.0 - state.graph_drag_offset.0 - canvas.x - state.graph_pan.0).max(8.0);
        let y = (mouse.1 - state.graph_drag_offset.1 - canvas.y - state.graph_pan.1).max(8.0);
        game.move_open_graph_node(&node_id, x as f64, y as f64).ok();
    }

    if graph_mouse_released && !graph_connection_resolved {
        state.graph_connect_from = None;
        state.graph_connect_pin.clear();
    }

    draw_graph_details(game, state, details, &view);
}

fn draw_graph_canvas_background(rect: RectSpec, pan: (f32, f32)) {
    draw_gradient_rect(
        rect,
        Color::from_rgba(11, 17, 28, 255),
        Color::from_rgba(6, 9, 15, 255),
    );
    let grid = 24.0;
    let mut x = rect.x + pan.0.rem_euclid(grid) - grid;
    while x < rect.x + rect.w {
        draw_line(
            x,
            rect.y,
            x,
            rect.y + rect.h,
            1.0,
            Color::from_rgba(31, 42, 58, 155),
        );
        x += grid;
    }
    let mut y = rect.y + pan.1.rem_euclid(grid) - grid;
    while y < rect.y + rect.h {
        draw_line(
            rect.x,
            y,
            rect.x + rect.w,
            y,
            1.0,
            Color::from_rgba(31, 42, 58, 155),
        );
        y += grid;
    }
}

fn update_graph_canvas_pan(state: &mut EditorState, pan_rect: RectSpec) {
    let mouse = mouse_position();
    let pan_button_down = is_mouse_button_down(MouseButton::Middle)
        || is_mouse_button_down(MouseButton::Right)
        || (is_key_down(KeyCode::Space) && is_mouse_button_down(MouseButton::Left));
    let wants_pan = (contains_mouse(pan_rect) || state.graph_panning)
        && state.graph_drag_node.is_none()
        && state.graph_connect_from.is_none()
        && pan_button_down;
    if wants_pan {
        if state.graph_panning {
            let dx = mouse.0 - state.graph_pan_last.0;
            let dy = mouse.1 - state.graph_pan_last.1;
            state.graph_pan.0 += dx;
            state.graph_pan.1 += dy;
        }
        state.graph_panning = true;
        state.graph_pan_last = mouse;
    } else {
        state.graph_panning = false;
        state.graph_pan_last = mouse;
    }
    if contains_mouse(pan_rect) {
        let (wheel_x, wheel_y) = mouse_wheel();
        if wheel_x.abs() > f32::EPSILON || wheel_y.abs() > f32::EPSILON {
            if is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift) {
                state.graph_pan.0 += wheel_y * 42.0;
            } else {
                state.graph_pan.0 += wheel_x * 42.0;
                state.graph_pan.1 += wheel_y * 42.0;
            }
        }
    }
    state.graph_pan.0 = state.graph_pan.0.clamp(-8000.0, 8000.0);
    state.graph_pan.1 = state.graph_pan.1.clamp(-8000.0, 8000.0);
}

fn frame_graph_nodes(state: &mut EditorState, canvas: RectSpec, view: &VisualGraphView) {
    let Some(first) = view.nodes.first() else {
        state.graph_pan = (24.0, 24.0);
        return;
    };
    let mut min_x = first.x as f32;
    let mut min_y = first.y as f32;
    let mut max_x = first.x as f32 + 148.0;
    let mut max_y = first.y as f32 + 74.0;
    for node in &view.nodes {
        min_x = min_x.min(node.x as f32);
        min_y = min_y.min(node.y as f32);
        max_x = max_x.max(node.x as f32 + 148.0);
        max_y = max_y.max(node.y as f32 + 74.0);
    }
    let graph_center_x = (min_x + max_x) * 0.5;
    let graph_center_y = (min_y + max_y) * 0.5;
    state.graph_pan = (
        canvas.w * 0.5 - graph_center_x,
        canvas.h * 0.5 - graph_center_y,
    );
}

fn graph_node_rect(canvas: RectSpec, pan: (f32, f32), node: &VisualGraphNodeView) -> RectSpec {
    RectSpec {
        x: canvas.x + pan.0 + node.x as f32,
        y: canvas.y + pan.1 + node.y as f32,
        w: 148.0,
        h: 74.0,
    }
}

fn graph_input_pin_pos(
    canvas: RectSpec,
    pan: (f32, f32),
    node: &VisualGraphNodeView,
) -> (f32, f32) {
    let rect = graph_node_rect(canvas, pan, node);
    (rect.x - 1.0, rect.y + 42.0)
}

fn graph_output_pin_pos_for(
    canvas: RectSpec,
    pan: (f32, f32),
    node: &VisualGraphNodeView,
    pin: &str,
) -> (f32, f32) {
    let rect = graph_node_rect(canvas, pan, node);
    let index = node
        .output_pins
        .iter()
        .position(|candidate| candidate == pin)
        .unwrap_or(0);
    let total = node.output_pins.len().max(1);
    let spacing = rect.h / (total as f32 + 1.0);
    (
        rect.x + rect.w + 1.0,
        rect.y + spacing * (index as f32 + 1.0),
    )
}

fn pin_rect(pos: (f32, f32)) -> RectSpec {
    RectSpec {
        x: pos.0 - 6.0,
        y: pos.1 - 6.0,
        w: 12.0,
        h: 12.0,
    }
}

fn draw_pin(rect: RectSpec, color: Color) {
    draw_circle(rect.x + rect.w * 0.5, rect.y + rect.h * 0.5, 5.5, color);
    draw_circle_lines(
        rect.x + rect.w * 0.5,
        rect.y + rect.h * 0.5,
        5.5,
        1.0,
        Color::from_rgba(245, 248, 255, 255),
    );
}

fn graph_pin_color(pin: &str) -> Color {
    match pin {
        "true" => Color::from_rgba(128, 224, 142, 255),
        "false" => Color::from_rgba(255, 136, 112, 255),
        "a" => Color::from_rgba(120, 210, 255, 255),
        "b" => Color::from_rgba(210, 160, 255, 255),
        _ => Color::from_rgba(255, 186, 92, 255),
    }
}

fn draw_graph_wire(canvas: RectSpec, start: (f32, f32), end: (f32, f32), color: Color) {
    let bounds = RectSpec {
        x: start.0.min(end.0),
        y: start.1.min(end.1),
        w: (end.0 - start.0).abs().max(1.0),
        h: (end.1 - start.1).abs().max(1.0),
    };
    if !rects_intersect(canvas, bounds) {
        return;
    }
    let handle = ((end.0 - start.0).abs() * 0.5).clamp(42.0, 180.0);
    let path = VectorPath2D::new(VectorStyle2D {
        fill: None,
        stroke: Some([
            (color.r * 255.0) as u8,
            (color.g * 255.0) as u8,
            (color.b * 255.0) as u8,
            (color.a * 255.0) as u8,
        ]),
        stroke_width: 2.25,
        tolerance: 0.15,
        ..VectorStyle2D::default()
    })
    .move_to(start.0, start.1)
    .cubic_to(
        start.0 + handle,
        start.1,
        end.0 - handle,
        end.1,
        end.0,
        end.1,
    );
    if let Ok(geometry) = path.tessellate() {
        draw_vector_geometry_screen(&geometry);
    }
}

fn rects_intersect(a: RectSpec, b: RectSpec) -> bool {
    a.x < b.x + b.w && a.x + a.w > b.x && a.y < b.y + b.h && a.y + a.h > b.y
}

fn draw_graph_node(rect: RectSpec, node: &VisualGraphNodeView, selected: bool, hovered: bool) {
    let color = graph_node_color(&node.node_type);
    draw_gradient_rect(
        rect,
        if selected {
            Color::from_rgba(38, 52, 72, 255)
        } else if hovered {
            Color::from_rgba(32, 41, 56, 255)
        } else {
            Color::from_rgba(24, 30, 42, 255)
        },
        Color::from_rgba(13, 18, 27, 255),
    );
    draw_rectangle(rect.x, rect.y, rect.w, 22.0, color);
    draw_rectangle_lines(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        1.0,
        if selected {
            Color::from_rgba(255, 202, 94, 255)
        } else {
            Color::from_rgba(64, 76, 94, 255)
        },
    );
    draw_text(
        &ellipsize(&node.title, 17),
        rect.x + 8.0,
        rect.y + 16.0,
        13.0,
        ui_text(),
    );
    draw_text(
        &ellipsize(&node.id, 18),
        rect.x + 10.0,
        rect.y + 42.0,
        12.0,
        ui_text_muted(),
    );
    draw_text(
        &format!(
            "in {} | out {}",
            node.input_pins.len(),
            node.output_pins.len()
        ),
        rect.x + 10.0,
        rect.y + 61.0,
        11.0,
        Color::from_rgba(122, 142, 172, 255),
    );
}

fn graph_node_color(node_type: &str) -> Color {
    match node_type {
        "EventStart" | "EventUpdate" | "EventClick" | "EventTrigger" | "ConstructionScript"
        | "CustomEvent" | "BroadcastEvent" | "CallEvent" => Color::from_rgba(155, 66, 72, 255),
        "Move" | "MoveTowards" | "SetVelocity" | "AddForce" | "StopMovement" | "SetSpeed" => {
            Color::from_rgba(55, 128, 190, 255)
        }
        "Log" => Color::from_rgba(70, 138, 104, 255),
        "SetVariable" | "AddVariable" | "SetEnabled" | "SetBlackboard" => {
            Color::from_rgba(132, 102, 182, 255)
        }
        "Heal" | "Damage" | "SetHealth" | "BranchHealth" => Color::from_rgba(178, 118, 52, 255),
        "BranchVariable" | "Wait" | "Gate" | "OpenGate" | "CloseGate" | "ToggleGate"
        | "FlipFlop" | "Sequence" | "DoOnce" | "ResetDoOnce" => Color::from_rgba(164, 126, 52, 255),
        "SetUiText" => Color::from_rgba(63, 142, 160, 255),
        "ConfigureSpawner" | "AddComponent" | "SetComponentNumber" | "DestroySelf" => {
            Color::from_rgba(97, 118, 92, 255)
        }
        _ => Color::from_rgba(82, 96, 118, 255),
    }
}

fn draw_graph_details(
    game: &mut Game,
    state: &mut EditorState,
    rect: RectSpec,
    view: &VisualGraphView,
) {
    draw_rect(rect, Color::from_rgba(18, 22, 30, 238));
    draw_rectangle_lines(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        1.0,
        Color::from_rgba(62, 74, 96, 255),
    );
    draw_text(
        "Graph Details",
        rect.x + 10.0,
        rect.y + 21.0,
        15.0,
        Color::from_rgba(232, 238, 248, 255),
    );
    draw_text(
        &ellipsize(&view.name, 24),
        rect.x + 10.0,
        rect.y + 43.0,
        12.0,
        Color::from_rgba(150, 166, 192, 255),
    );
    let blueprint_analysis = serde_json::from_str::<Value>(&game.script_editor.text())
        .ok()
        .and_then(|value| {
            crate::engine::miniforge_2d::blueprint::graph_from_value(&value)
                .ok()
                .map(|graph| graph.analyze())
        });
    if let Some(analysis) = &blueprint_analysis {
        draw_text(
            &format!(
                "{} nodes | {} links | depth {}",
                analysis.node_count, analysis.edge_count, analysis.max_exec_depth
            ),
            rect.x + 10.0,
            rect.y + 62.0,
            11.0,
            Color::from_rgba(190, 202, 222, 255),
        );
        draw_text(
            &format!(
                "{} reachable | {} orphan",
                analysis.reachable_node_ids.len(),
                analysis.orphan_node_ids.len()
            ),
            rect.x + 10.0,
            rect.y + 78.0,
            11.0,
            if analysis.orphan_node_ids.is_empty() {
                Color::from_rgba(135, 230, 165, 255)
            } else {
                Color::from_rgba(255, 206, 130, 255)
            },
        );
    } else {
        draw_text(
            &format!(
                "{} nodes | {} links",
                view.nodes.len(),
                view.connections.len()
            ),
            rect.x + 10.0,
            rect.y + 62.0,
            11.0,
            Color::from_rgba(190, 202, 222, 255),
        );
    }
    let selected = state
        .graph_selected_node
        .as_ref()
        .and_then(|id| view.nodes.iter().find(|node| &node.id == id));
    if let Some(node) = selected {
        draw_text(
            &format!("Node: {}", node.title),
            rect.x + 10.0,
            rect.y + 102.0,
            13.0,
            Color::from_rgba(126, 205, 255, 255),
        );
        draw_text(
            &format!("id {}", node.id),
            rect.x + 10.0,
            rect.y + 123.0,
            12.0,
            Color::from_rgba(190, 202, 222, 255),
        );
        draw_text(
            &format!("next {}", node.next.as_deref().unwrap_or("none")),
            rect.x + 10.0,
            rect.y + 142.0,
            12.0,
            Color::from_rgba(190, 202, 222, 255),
        );
        draw_text(
            &format!("pins {}", node.output_pins.join(", ")),
            rect.x + 10.0,
            rect.y + 161.0,
            12.0,
            Color::from_rgba(190, 202, 222, 255),
        );
    } else {
        draw_text(
            "Select a node or drag from output pin.",
            rect.x + 10.0,
            rect.y + 104.0,
            12.0,
            Color::from_rgba(132, 150, 176, 255),
        );
    }
    let mut y = rect.y + 176.0;
    draw_text(
        "Warnings",
        rect.x + 10.0,
        y,
        13.0,
        Color::from_rgba(255, 210, 120, 255),
    );
    y += 18.0;
    if view.warnings.is_empty() {
        draw_text(
            "clean",
            rect.x + 18.0,
            y,
            12.0,
            Color::from_rgba(135, 230, 165, 255),
        );
    } else {
        for warning in view.warnings.iter().take(6) {
            draw_text(
                &ellipsize(warning, 27),
                rect.x + 18.0,
                y,
                12.0,
                Color::from_rgba(255, 206, 130, 255),
            );
            y += 17.0;
        }
    }
    if let Some(analysis) = &blueprint_analysis {
        y += 8.0;
        draw_text(
            "Flow",
            rect.x + 10.0,
            y,
            13.0,
            Color::from_rgba(125, 216, 205, 255),
        );
        y += 18.0;
        let flow = if analysis.orphan_node_ids.is_empty() {
            format!("events {}", analysis.event_node_ids.join(", "))
        } else {
            format!("orphans {}", analysis.orphan_node_ids.join(", "))
        };
        draw_text(
            &ellipsize(&flow, 29),
            rect.x + 18.0,
            y,
            12.0,
            ui_text_muted(),
        );
        y += 17.0;
        for action in analysis.recommended_actions.iter().take(3) {
            draw_text(
                &ellipsize(action, 28),
                rect.x + 18.0,
                y,
                12.0,
                Color::from_rgba(214, 224, 238, 255),
            );
            y += 16.0;
        }
    }
    y += 22.0;
    draw_text(
        "Node Library",
        rect.x + 10.0,
        y,
        13.0,
        Color::from_rgba(126, 205, 255, 255),
    );
    y += 8.0;
    let search = RectSpec {
        x: rect.x + 10.0,
        y,
        w: rect.w - 20.0,
        h: 24.0,
    };
    draw_graph_node_search_box(state, search);
    y += 34.0;
    let definitions = game
        .programming
        .search_node_catalog(&state.graph_node_search);
    for definition in definitions.iter().take(5) {
        let row = RectSpec {
            x: rect.x + 10.0,
            y: y - 14.0,
            w: rect.w - 20.0,
            h: 22.0,
        };
        let hovered = contains_mouse(row);
        draw_rect(
            row,
            if hovered {
                Color::from_rgba(45, 55, 72, 255)
            } else {
                Color::from_rgba(27, 33, 44, 255)
            },
        );
        draw_text(
            &ellipsize(&definition.label, 18),
            row.x + 7.0,
            y,
            12.0,
            Color::from_rgba(232, 238, 248, 255),
        );
        draw_text(
            &ellipsize(&definition.category, 9),
            row.x + row.w - 64.0,
            y,
            11.0,
            Color::from_rgba(150, 166, 192, 255),
        );
        if hovered && is_mouse_button_pressed(MouseButton::Left) {
            game.add_node_to_open_graph(&definition.node_type).ok();
        }
        y += 24.0;
    }
    if button(
        rect.x + 10.0,
        rect.y + rect.h - 34.0,
        96.0,
        22.0,
        "Open JSON",
        false,
    ) {
        state.code_editor_active = true;
        game.console.log("Graph JSON editable activo", "SCRIPT");
    }
}

fn draw_code_editor(game: &mut Game, state: &mut EditorState, rect: RectSpec) {
    draw_gradient_rect(
        rect,
        if state.code_editor_active {
            Color::from_rgba(25, 32, 42, 245)
        } else {
            Color::from_rgba(21, 25, 34, 235)
        },
        Color::from_rgba(12, 16, 24, 245),
    );
    draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 1.0, ui_line_soft());
    let title = game
        .script_editor
        .document
        .path
        .as_ref()
        .and_then(|path| path.strip_prefix(&game.project_path).ok())
        .map(|path| path.to_string_lossy().to_string())
        .unwrap_or_else(|| "No file open".to_string());
    let dirty = if game.script_editor.document.dirty {
        " *"
    } else {
        ""
    };
    draw_text_fit(
        &format!("Editor: {title}{dirty}"),
        RectSpec {
            x: rect.x + 10.0,
            y: rect.y + 3.0,
            w: (rect.w - 456.0).max(160.0),
            h: 24.0,
        },
        16,
        ui_text(),
        TextAlign::Left,
    );
    let button_y = rect.y + 5.0;
    if button(
        rect.x + rect.w - 436.0,
        button_y,
        78.0,
        21.0,
        "VS Code",
        state.editor_preferences.prefer_vscode,
    ) {
        open_current_file_in_external_editor(game, state);
    }
    if button(
        rect.x + rect.w - 352.0,
        button_y,
        72.0,
        21.0,
        "Open Sel",
        false,
    ) && let Some(path) = state.selected_asset_path.clone()
    {
        open_project_file_in_editor(game, state, path);
    }
    if button(rect.x + rect.w - 274.0, button_y, 58.0, 21.0, "Save", false)
        && let Err(error) = game.save_open_file()
    {
        game.console
            .log(format!("No se pudo guardar archivo: {error}"), "ERROR");
    }
    if button(
        rect.x + rect.w - 210.0,
        button_y,
        62.0,
        21.0,
        "Reload",
        false,
    ) && let Err(error) = game.reload_open_file()
    {
        game.console
            .log(format!("No se pudo recargar archivo: {error}"), "ERROR");
    }
    if button(
        rect.x + rect.w - 142.0,
        button_y,
        62.0,
        21.0,
        "Check",
        false,
    ) {
        if game.script_editor.validate() {
            game.console.log("Archivo validado sin errores", "EDITOR");
        } else if let Some(error) = &game.script_editor.document.syntax_error {
            game.console.log(error.clone(), "ERROR");
        }
    }
    let tools_y = rect.y + 30.0;
    if button(rect.x + 10.0, tools_y, 78.0, 21.0, "fn start", false)
        && game.script_editor.insert_luau_event_template("on_start")
    {
        state.code_cursor_line = game.script_editor.lines.len().saturating_sub(1);
        state.code_cursor_column = 0;
    }
    if button(rect.x + 94.0, tools_y, 86.0, 21.0, "fn update", false)
        && game.script_editor.insert_luau_event_template("on_update")
    {
        state.code_cursor_line = game.script_editor.lines.len().saturating_sub(1);
        state.code_cursor_column = 0;
    }
    if button(rect.x + 186.0, tools_y, 76.0, 21.0, "Format", false) {
        match game.script_editor.format_json_pretty() {
            Ok(true) => game.console.log("JSON/graph formateado", "SCRIPT"),
            Ok(false) => {}
            Err(error) => game.console.log(
                format!("No se pudo formatear como JSON: {error}"),
                "WARNING",
            ),
        }
    }
    if button(rect.x + 268.0, tools_y, 68.0, 21.0, "Dupe", false) {
        let (line, column) = game.script_editor.duplicate_line(state.code_cursor_line);
        state.code_cursor_line = line;
        state.code_cursor_column = column;
    }
    if button(rect.x + 342.0, tools_y, 78.0, 21.0, "Comment", false) {
        let (line, column) = game
            .script_editor
            .toggle_line_comment(state.code_cursor_line);
        state.code_cursor_line = line;
        state.code_cursor_column = column;
    }
    if button(rect.x + 426.0, tools_y, 56.0, 21.0, "+Log", false)
        && let Some((line, column)) = game.script_editor.insert_snippet(
            "log",
            state.code_cursor_line,
            state.code_cursor_column,
        )
    {
        state.code_cursor_line = line;
        state.code_cursor_column = column;
    }
    if button(rect.x + 488.0, tools_y, 70.0, 21.0, "+Spawn", false)
        && let Some((line, column)) = game.script_editor.insert_snippet(
            "spawn",
            state.code_cursor_line,
            state.code_cursor_column,
        )
    {
        state.code_cursor_line = line;
        state.code_cursor_column = column;
    }
    if button(rect.x + 564.0, tools_y, 70.0, 21.0, "+Sprite", false)
        && let Some((line, column)) = game.script_editor.insert_snippet(
            "sprite",
            state.code_cursor_line,
            state.code_cursor_column,
        )
    {
        state.code_cursor_line = line;
        state.code_cursor_column = column;
    }
    if button(rect.x + 640.0, tools_y, 62.0, 21.0, "+Anim", false)
        && let Some((line, column)) = game.script_editor.insert_snippet(
            "anim",
            state.code_cursor_line,
            state.code_cursor_column,
        )
    {
        state.code_cursor_line = line;
        state.code_cursor_column = column;
    }

    let side_w = if rect.w > 760.0 { 270.0 } else { 0.0 };
    let code_area = RectSpec {
        x: rect.x + 8.0,
        y: rect.y + 58.0,
        w: rect.w - 16.0 - side_w,
        h: rect.h - 86.0,
    };
    let side_panel = RectSpec {
        x: code_area.x + code_area.w + 8.0,
        y: code_area.y,
        w: side_w - 8.0,
        h: code_area.h,
    };
    draw_rect(code_area, Color::from_rgba(13, 16, 23, 255));
    if contains_mouse(code_area) && is_mouse_button_pressed(MouseButton::Left) {
        state.code_editor_active = true;
        let (mx, _) = mouse_position();
        let row = ((mouse_position().1 - code_area.y) / 17.0).max(0.0) as usize;
        state.code_cursor_line =
            (state.code_scroll_line + row).min(game.script_editor.lines.len().saturating_sub(1));
        state.code_cursor_column = game
            .script_editor
            .lines
            .get(state.code_cursor_line)
            .map(|line| {
                byte_index_from_visual_column(
                    line,
                    ((mx - code_area.x - 48.0) / 7.2).round().max(0.0) as usize,
                )
            })
            .unwrap_or(0);
    }

    if game.script_editor.document.path.is_none() {
        if button(
            code_area.x + 12.0,
            code_area.y + 12.0,
            118.0,
            24.0,
            "New Script",
            false,
        ) && let Ok(path) = game.create_luau_script_asset("NewGameplayScript")
        {
            set_open_file_editor_state(state, &path);
        }
        draw_text(
            "Sin archivo abierto. Click aqui o New Script para empezar.",
            code_area.x + 12.0,
            code_area.y + 58.0,
            14.0,
            Color::from_rgba(150, 166, 192, 255),
        );
        return;
    }

    let visible_lines = (code_area.h / 17.0).max(1.0) as usize;
    if contains_mouse(code_area) {
        let (_, wheel_y) = mouse_wheel();
        if wheel_y.abs() > f32::EPSILON {
            let max_scroll = game.script_editor.lines.len().saturating_sub(visible_lines);
            let next = if wheel_y > 0.0 {
                state
                    .code_scroll_line
                    .saturating_sub(wheel_y.ceil() as usize)
            } else {
                state.code_scroll_line + (-wheel_y).ceil() as usize
            };
            state.code_scroll_line = next.min(max_scroll);
        }
    }
    let max_scroll_line = game.script_editor.lines.len().saturating_sub(visible_lines);
    state.code_scroll_line = state.code_scroll_line.min(max_scroll_line);
    let start_line = state.code_scroll_line;
    let mut y = code_area.y + 15.0;
    for (index, line) in game
        .script_editor
        .lines
        .iter()
        .enumerate()
        .skip(start_line)
        .take(visible_lines)
    {
        if index == state.code_cursor_line {
            draw_rectangle(
                code_area.x + 2.0,
                y - 13.0,
                code_area.w - 4.0,
                16.0,
                Color::from_rgba(34, 45, 62, 255),
            );
        }
        draw_text(
            &format!("{:>3}", index + 1),
            code_area.x + 8.0,
            y,
            13.0,
            Color::from_rgba(96, 116, 145, 255),
        );
        let matches_search = !state.script_search.is_empty()
            && line
                .to_lowercase()
                .contains(&state.script_search.to_lowercase());
        if matches_search {
            draw_rectangle(
                code_area.x + 2.0,
                y - 13.0,
                code_area.w - 4.0,
                16.0,
                Color::from_rgba(66, 54, 24, 210),
            );
        }
        let extension = game
            .script_editor
            .document
            .path
            .as_ref()
            .and_then(|path| path.extension())
            .and_then(|extension| extension.to_str())
            .unwrap_or("rs");
        draw_code_line_colored(
            &state.syntax_highlighter,
            line,
            extension,
            code_area.x + 48.0,
            y,
            (code_area.w - 60.0).max(40.0),
            if matches_search {
                Some(Color::from_rgba(255, 230, 150, 255))
            } else {
                None
            },
        );
        if index == state.code_cursor_line && state.code_editor_active {
            let cursor_x = code_area.x
                + 48.0
                + (line_visual_column(line, state.code_cursor_column) as f32 * 7.2);
            draw_line(
                cursor_x,
                y - 13.0,
                cursor_x,
                y + 2.0,
                1.5,
                Color::from_rgba(120, 220, 255, 255),
            );
        }
        y += 17.0;
    }

    if side_w > 0.0 {
        draw_script_intelligence_panel(game, state, side_panel);
    }

    if let Some(error) = &game.script_editor.document.syntax_error {
        draw_text(
            &ellipsize(error, 110),
            rect.x + 10.0,
            rect.y + rect.h - 16.0,
            13.0,
            Color::from_rgba(255, 145, 120, 255),
        );
    } else {
        draw_text(
            "Editor de codigo listo.",
            rect.x + 10.0,
            rect.y + rect.h - 16.0,
            12.0,
            Color::from_rgba(132, 150, 176, 255),
        );
    }
    let content_h = game.script_editor.lines.len() as f32 * 17.0;
    draw_scrollbar(
        RectSpec {
            x: code_area.x + code_area.w - 7.0,
            y: code_area.y,
            w: 6.0,
            h: code_area.h,
        },
        state.code_scroll_line as f32 * 17.0,
        (content_h - code_area.h).max(0.0),
        content_h.max(code_area.h),
    );
}

fn draw_script_intelligence_panel(game: &mut Game, state: &mut EditorState, rect: RectSpec) {
    draw_rect(rect, Color::from_rgba(16, 20, 29, 245));
    draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 1.0, ui_line_soft());
    draw_text("Script Map", rect.x + 10.0, rect.y + 19.0, 14.0, ui_text());
    draw_script_search_box(
        state,
        RectSpec {
            x: rect.x + 8.0,
            y: rect.y + 28.0,
            w: rect.w - 16.0,
            h: 24.0,
        },
    );

    let stats = game.script_editor.stats();
    let diagnostic_summary = game.script_editor.diagnostic_summary();
    draw_text(
        &format!(
            "{} lines | {} chars | {} fn | {} nodes",
            stats.lines, stats.characters, stats.functions, stats.graph_nodes
        ),
        rect.x + 10.0,
        rect.y + 70.0,
        12.0,
        ui_text_muted(),
    );
    draw_text(
        &format!(
            "E{} W{} H{}",
            diagnostic_summary.errors, diagnostic_summary.warnings, diagnostic_summary.hints
        ),
        rect.x + rect.w - 72.0,
        rect.y + 70.0,
        12.0,
        if diagnostic_summary.errors > 0 {
            Color::from_rgba(255, 130, 110, 255)
        } else {
            ui_text_muted()
        },
    );

    let mut y = rect.y + 94.0;
    draw_text(
        "Outline",
        rect.x + 10.0,
        y,
        13.0,
        Color::from_rgba(125, 216, 205, 255),
    );
    y += 18.0;
    let query = state.script_search.to_lowercase();
    let outline = game.script_editor.outline.clone();
    for symbol in outline
        .iter()
        .filter(|symbol| query.is_empty() || symbol.name.to_lowercase().contains(&query))
        .take(7)
    {
        let row = RectSpec {
            x: rect.x + 8.0,
            y: y - 13.0,
            w: rect.w - 16.0,
            h: 17.0,
        };
        if contains_mouse(row) {
            draw_rect(row, Color::from_rgba(40, 50, 64, 255));
            if is_mouse_button_pressed(MouseButton::Left) {
                state.code_cursor_line = symbol.line.saturating_sub(1);
                state.code_cursor_column = 0;
                keep_code_cursor_visible(state, 12);
            }
        }
        draw_text(
            &ellipsize(&format!("{}  {}", symbol.kind, symbol.name), 27),
            rect.x + 12.0,
            y,
            12.0,
            ui_text(),
        );
        draw_text(
            &format!("{}", symbol.line),
            rect.x + rect.w - 34.0,
            y,
            12.0,
            ui_text_muted(),
        );
        y += 18.0;
    }

    if !state.script_search.is_empty() {
        y += 6.0;
        draw_text(
            "Matches",
            rect.x + 10.0,
            y,
            13.0,
            Color::from_rgba(145, 190, 255, 255),
        );
        y += 18.0;
        for result in game
            .script_editor
            .search_text(&state.script_search)
            .iter()
            .take(4)
        {
            let row = RectSpec {
                x: rect.x + 8.0,
                y: y - 13.0,
                w: rect.w - 16.0,
                h: 17.0,
            };
            if contains_mouse(row) {
                draw_rect(row, Color::from_rgba(40, 50, 64, 255));
                if is_mouse_button_pressed(MouseButton::Left) {
                    state.code_cursor_line = result.line.saturating_sub(1);
                    state.code_cursor_column = result.column;
                    keep_code_cursor_visible(state, 12);
                }
            }
            draw_text(
                &ellipsize(&format!("{}  {}", result.line, result.preview), 31),
                rect.x + 12.0,
                y,
                12.0,
                Color::from_rgba(214, 224, 238, 255),
            );
            y += 17.0;
        }
    }

    y += 8.0;
    draw_text(
        "Diagnostics",
        rect.x + 10.0,
        y,
        13.0,
        Color::from_rgba(255, 206, 130, 255),
    );
    y += 18.0;
    let diagnostics = game.script_editor.diagnostics.clone();
    if diagnostics.is_empty() {
        draw_text(
            "No issues after last check.",
            rect.x + 12.0,
            y,
            12.0,
            Color::from_rgba(135, 230, 165, 255),
        );
    } else {
        for diagnostic in diagnostics.iter().take(7) {
            let color = match diagnostic.severity {
                crate::engine::script_editor::ScriptDiagnosticSeverity::Error => {
                    Color::from_rgba(255, 130, 110, 255)
                }
                crate::engine::script_editor::ScriptDiagnosticSeverity::Warning => {
                    Color::from_rgba(255, 206, 130, 255)
                }
                crate::engine::script_editor::ScriptDiagnosticSeverity::Hint => {
                    Color::from_rgba(145, 190, 255, 255)
                }
            };
            let prefix = diagnostic
                .line
                .map(|line| format!("{line}: "))
                .unwrap_or_default();
            draw_text(
                &ellipsize(&format!("{prefix}{}", diagnostic.message), 32),
                rect.x + 12.0,
                y,
                12.0,
                color,
            );
            y += 17.0;
        }
    }

    y += 10.0;
    draw_text(
        "Actions",
        rect.x + 10.0,
        y,
        13.0,
        Color::from_rgba(126, 205, 255, 255),
    );
    y += 18.0;
    let actions = game.script_editor.code_actions();
    for action in actions.iter().take(4) {
        let row = RectSpec {
            x: rect.x + 8.0,
            y: y - 13.0,
            w: rect.w - 16.0,
            h: 18.0,
        };
        let hovered = contains_mouse(row);
        draw_rect(
            row,
            if hovered {
                Color::from_rgba(38, 50, 68, 255)
            } else {
                Color::from_rgba(24, 30, 41, 255)
            },
        );
        draw_text(
            &ellipsize(&action.title, 31),
            rect.x + 12.0,
            y,
            12.0,
            ui_text(),
        );
        if hovered && is_mouse_button_pressed(MouseButton::Left) {
            match game.script_editor.apply_code_action(action) {
                Ok(true) => {
                    game.console
                        .log(format!("Action applied: {}", action.title), "SCRIPT");
                    keep_code_cursor_visible(state, 12);
                }
                Ok(false) => {
                    if let Some(line) = action.line {
                        state.code_cursor_line = line.saturating_sub(1);
                        state.code_cursor_column = 0;
                        keep_code_cursor_visible(state, 12);
                    }
                }
                Err(error) => game
                    .console
                    .log(format!("No se pudo aplicar accion: {error}"), "WARNING"),
            }
        }
        y += 19.0;
    }

    let mini_y = rect.y + rect.h - 84.0;
    draw_text(
        "Mini",
        rect.x + 10.0,
        mini_y,
        12.0,
        Color::from_rgba(125, 216, 205, 255),
    );
    let mini = game.script_editor.minimap();
    let mini_rect = RectSpec {
        x: rect.x + 48.0,
        y: mini_y - 11.0,
        w: rect.w - 58.0,
        h: 28.0,
    };
    draw_rect(mini_rect, Color::from_rgba(11, 15, 22, 255));
    for item in mini.iter().take(42) {
        let color = match item.kind.as_str() {
            "diagnostic" => Color::from_rgba(255, 130, 110, 255),
            "function" => Color::from_rgba(125, 216, 205, 255),
            "graph_node" => Color::from_rgba(145, 190, 255, 255),
            "comment" => Color::from_rgba(96, 116, 145, 255),
            _ => Color::from_rgba(214, 224, 238, 180),
        };
        let x = mini_rect.x + (item.line.saturating_sub(1) as f32 % 42.0) * 4.0;
        let h = ((item.intensity as f32 / 96.0) * 22.0).clamp(3.0, 22.0);
        draw_rectangle(x, mini_rect.y + mini_rect.h - h - 3.0, 2.0, h, color);
    }

    let help_y = rect.y + rect.h - 46.0;
    draw_text(
        "Cmd+S save | Cmd+D duplicate",
        rect.x + 10.0,
        help_y,
        11.0,
        ui_text_muted(),
    );
    draw_text(
        "Cmd+/ comment | Tab indent",
        rect.x + 10.0,
        help_y + 16.0,
        11.0,
        ui_text_muted(),
    );
}

fn draw_code_line_colored(
    highlighter: &EditorSyntaxHighlighter,
    line: &str,
    extension: &str,
    x: f32,
    y: f32,
    max_w: f32,
    override_color: Option<Color>,
) {
    let mut draw_x = x;
    let max_x = x + max_w;
    let spans = highlighter.highlight_line(line, extension);
    for span in spans {
        if draw_x >= max_x {
            break;
        }
        let color = override_color.unwrap_or_else(|| {
            Color::from_rgba(span.rgba[0], span.rgba[1], span.rgba[2], span.rgba[3])
        });
        let shown = if draw_x + measure_text(&span.text, None, 13, 1.0).width > max_x {
            let remaining = ((max_x - draw_x) / 7.2).floor().max(1.0) as usize;
            ellipsize(&span.text, remaining)
        } else {
            span.text
        };
        if !shown.trim().is_empty() {
            draw_text(&shown, draw_x, y, 13.0, color);
        }
        draw_x += measure_text(&shown, None, 13, 1.0).width;
        if shown.ends_with("...") {
            break;
        }
    }
}

fn byte_index_from_visual_column(line: &str, visual_column: usize) -> usize {
    line.char_indices()
        .nth(visual_column)
        .map(|(index, _)| index)
        .unwrap_or(line.len())
}

fn line_visual_column(line: &str, byte_index: usize) -> usize {
    let byte_index = byte_index.min(line.len());
    line.char_indices()
        .take_while(|(index, _)| *index < byte_index)
        .count()
}

fn previous_code_char_boundary(line: &str, byte_index: usize) -> usize {
    let mut previous = 0usize;
    for (index, _) in line.char_indices() {
        if index >= byte_index {
            break;
        }
        previous = index;
    }
    previous
}

fn next_code_char_boundary(line: &str, byte_index: usize) -> usize {
    let byte_index = byte_index.min(line.len());
    line[byte_index..]
        .char_indices()
        .nth(1)
        .map(|(index, _)| byte_index + index)
        .unwrap_or(line.len())
}

fn draw_prefab_panel(game: &mut Game, state: &mut EditorState, rect: RectSpec) {
    draw_panel_chrome(rect, "Prefabs", &game.advanced_prefabs.status_line());
    if button(rect.x + 14.0, rect.y + 31.0, 92.0, 22.0, "Save Sel", false) {
        game.save_selected_as_prefab().ok();
    }
    if button(rect.x + 114.0, rect.y + 31.0, 78.0, 22.0, "Variant", false) {
        game.create_selected_prefab_variant().ok();
    }
    if button(rect.x + 200.0, rect.y + 31.0, 94.0, 22.0, "Instance", false) {
        let (x, y) = spawn_position(game);
        if let Some(path) = selected_prefab_path(game, state) {
            game.instantiate_prefab_asset(&path, x, y).ok();
        } else {
            game.instantiate_first_prefab(x, y).ok();
        }
    }
    if button(rect.x + 302.0, rect.y + 31.0, 68.0, 22.0, "Apply", false) {
        game.apply_selected_to_prefab_source().ok();
    }
    if button(rect.x + 378.0, rect.y + 31.0, 70.0, 22.0, "Revert", false) {
        game.revert_selected_prefab_instance().ok();
    }
    if button(rect.x + 456.0, rect.y + 31.0, 70.0, 22.0, "Detach", false) {
        game.detach_selected_prefab_instance();
    }

    let list = RectSpec {
        x: rect.x + 10.0,
        y: rect.y + 64.0,
        w: (rect.w * 0.48).max(420.0),
        h: rect.h - 72.0,
    };
    let details = RectSpec {
        x: list.x + list.w + 10.0,
        y: list.y,
        w: rect.w - list.w - 30.0,
        h: list.h,
    };
    draw_rect(list, Color::from_rgba(18, 22, 30, 238));
    draw_rectangle_lines(
        list.x,
        list.y,
        list.w,
        list.h,
        1.0,
        Color::from_rgba(62, 74, 96, 255),
    );
    draw_text(
        "Prefab Assets",
        list.x + 12.0,
        list.y + 24.0,
        16.0,
        Color::from_rgba(232, 238, 248, 255),
    );
    let prefabs = game
        .asset_database
        .assets
        .values()
        .filter(|asset| asset.asset_type == "Prefab")
        .cloned()
        .collect::<Vec<_>>();
    let mut y = list.y + 50.0;
    for prefab in prefabs
        .iter()
        .take(((list.h - 54.0) / 52.0).max(0.0) as usize)
    {
        let row = RectSpec {
            x: list.x + 10.0,
            y: y - 18.0,
            w: list.w - 20.0,
            h: 46.0,
        };
        let selected = state.selected_asset_path.as_deref() == Some(prefab.relative_path.as_str());
        let hovered = contains_mouse(row);
        draw_rect(
            row,
            if selected {
                Color::from_rgba(52, 73, 108, 255)
            } else if hovered {
                Color::from_rgba(36, 45, 58, 255)
            } else {
                Color::from_rgba(25, 30, 40, 255)
            },
        );
        draw_rectangle(
            row.x + 8.0,
            row.y + 8.0,
            30.0,
            30.0,
            asset_type_color("Prefab"),
        );
        draw_text(
            "P",
            row.x + 18.0,
            row.y + 29.0,
            18.0,
            Color::from_rgba(245, 248, 255, 255),
        );
        draw_text(
            &ellipsize(&prefab.name, 32),
            row.x + 48.0,
            y,
            15.0,
            Color::from_rgba(232, 238, 248, 255),
        );
        draw_text(
            &ellipsize(&prefab.relative_path, 64),
            row.x + 48.0,
            y + 18.0,
            12.0,
            Color::from_rgba(145, 162, 190, 255),
        );
        if hovered && is_mouse_button_pressed(MouseButton::Left) {
            state.selected_asset_path = Some(prefab.relative_path.clone());
        }
        y += 52.0;
    }

    draw_rect(details, Color::from_rgba(21, 25, 34, 238));
    draw_rectangle_lines(
        details.x,
        details.y,
        details.w,
        details.h,
        1.0,
        Color::from_rgba(62, 74, 96, 255),
    );
    draw_text(
        "Prefab Details",
        details.x + 12.0,
        details.y + 24.0,
        16.0,
        Color::from_rgba(232, 238, 248, 255),
    );
    if let Some(path) = selected_prefab_path(game, state) {
        let name = path
            .rsplit('/')
            .next()
            .unwrap_or(path.as_str())
            .trim_end_matches(".prefab");
        draw_preview_visual("Prefab", details.x + 12.0, details.y + 42.0, 112.0, 78.0);
        draw_text(
            &ellipsize(name, 34),
            details.x + 136.0,
            details.y + 62.0,
            16.0,
            Color::from_rgba(232, 238, 248, 255),
        );
        draw_text(
            &ellipsize(&path, 48),
            details.x + 136.0,
            details.y + 84.0,
            12.0,
            Color::from_rgba(145, 162, 190, 255),
        );
        if button(
            details.x + 136.0,
            details.y + 100.0,
            88.0,
            22.0,
            "Open",
            false,
        ) {
            open_project_file_in_editor(game, state, path.clone());
        }
        if button(
            details.x + 232.0,
            details.y + 100.0,
            104.0,
            22.0,
            "Instantiate",
            false,
        ) {
            let (x, y) = spawn_position(game);
            game.instantiate_prefab_asset(&path, x, y).ok();
        }
    } else {
        draw_text(
            "Selecciona un prefab para inspeccionarlo.",
            details.x + 12.0,
            details.y + 58.0,
            13.0,
            Color::from_rgba(145, 162, 190, 255),
        );
    }

    if let Some(report) = &game.advanced_prefabs.last_report {
        draw_text(
            &format!(
                "Selected report: entity #{} | {} overrides | source {}",
                report.entity_id,
                report.override_count,
                if report.missing_source {
                    "missing"
                } else {
                    "linked"
                }
            ),
            details.x + 12.0,
            details.y + details.h - 34.0,
            14.0,
            Color::from_rgba(205, 214, 232, 255),
        );
    }
}

fn selected_prefab_path(game: &Game, state: &EditorState) -> Option<String> {
    let path = state.selected_asset_path.as_ref()?;
    game.asset_database
        .assets
        .get(path)
        .filter(|asset| asset.asset_type == "Prefab")
        .map(|asset| asset.relative_path.clone())
}

fn draw_scenes_panel(game: &mut Game, _state: &mut EditorState, rect: RectSpec) {
    let scenes = game.scene_names().unwrap_or_default();
    draw_panel_chrome(
        rect,
        "Scenes",
        &format!(
            "{} loaded | current {} | stack {}",
            game.scene_manager.loaded_scenes.len(),
            game.scene_manager.current_scene,
            game.scene_manager.scene_stack.len()
        ),
    );
    if button(rect.x + 14.0, rect.y + 31.0, 86.0, 22.0, "Save", false) {
        save_project(game);
    }
    if button(rect.x + 108.0, rect.y + 31.0, 92.0, 22.0, "New", false) {
        game.create_empty_scene("NewScene").ok();
    }
    if button(
        rect.x + 208.0,
        rect.y + 31.0,
        104.0,
        22.0,
        "Duplicate",
        false,
    ) {
        game.scene_manager.duplicate_current_scene("SceneCopy").ok();
    }
    if button(rect.x + 320.0, rect.y + 31.0, 104.0, 22.0, "Restart", false) {
        game.restart_scene().ok();
    }

    let list = RectSpec {
        x: rect.x + 10.0,
        y: rect.y + 64.0,
        w: (rect.w * 0.52).max(420.0),
        h: rect.h - 74.0,
    };
    let details = RectSpec {
        x: list.x + list.w + 10.0,
        y: list.y,
        w: rect.w - list.w - 30.0,
        h: list.h,
    };
    draw_rect(list, Color::from_rgba(18, 22, 30, 238));
    draw_rectangle_lines(
        list.x,
        list.y,
        list.w,
        list.h,
        1.0,
        Color::from_rgba(62, 74, 96, 255),
    );
    draw_text(
        "Project Scenes",
        list.x + 12.0,
        list.y + 24.0,
        16.0,
        Color::from_rgba(232, 238, 248, 255),
    );

    let mut y = list.y + 54.0;
    for scene in scenes
        .iter()
        .take(((list.h - 58.0) / 36.0).max(0.0) as usize)
    {
        let row = RectSpec {
            x: list.x + 10.0,
            y: y - 19.0,
            w: list.w - 20.0,
            h: 31.0,
        };
        let active = game.scene_manager.current_scene == *scene;
        let hovered = contains_mouse(row);
        draw_rect(
            row,
            if active {
                Color::from_rgba(52, 73, 108, 255)
            } else if hovered {
                Color::from_rgba(36, 45, 58, 255)
            } else {
                Color::from_rgba(25, 30, 40, 255)
            },
        );
        draw_text(
            &ellipsize(scene, 42),
            row.x + 12.0,
            y,
            15.0,
            Color::from_rgba(232, 238, 248, 255),
        );
        if button(
            row.x + row.w - 224.0,
            row.y + 5.0,
            62.0,
            20.0,
            "Load",
            active,
        ) {
            game.load_scene(scene).ok();
        }
        if button(row.x + row.w - 154.0, row.y + 5.0, 72.0, 20.0, "Add", false) {
            game.load_scene_additive(scene).ok();
        }
        if button(row.x + row.w - 74.0, row.y + 5.0, 62.0, 20.0, "Push", false) {
            game.push_scene(scene).ok();
        }
        y += 36.0;
    }

    draw_rect(details, Color::from_rgba(21, 25, 34, 238));
    draw_rectangle_lines(
        details.x,
        details.y,
        details.w,
        details.h,
        1.0,
        Color::from_rgba(62, 74, 96, 255),
    );
    draw_text(
        "Runtime Scene State",
        details.x + 12.0,
        details.y + 24.0,
        16.0,
        Color::from_rgba(232, 238, 248, 255),
    );
    let rows = [
        format!("Current: {}", game.scene_manager.current_scene),
        format!("Loaded: {}", game.scene_manager.loaded_scenes.join(", ")),
        format!("Stack: {}", game.scene_manager.scene_stack.join(" > ")),
        format!(
            "Transition: {}",
            game.scene_manager
                .transition
                .as_ref()
                .map(|transition| format!(
                    "{} -> {} {:.0}%",
                    transition.from_scene,
                    transition.to_scene,
                    if transition.duration <= 0.0 {
                        100.0
                    } else {
                        transition.elapsed / transition.duration * 100.0
                    }
                ))
                .unwrap_or_else(|| "none".to_string())
        ),
    ];
    let mut y = details.y + 58.0;
    for row in rows {
        draw_text(
            &ellipsize(&row, 72),
            details.x + 12.0,
            y,
            14.0,
            Color::from_rgba(190, 204, 226, 255),
        );
        y += 22.0;
    }
}

fn draw_sprite_editor_panel(game: &mut Game, state: &mut EditorState, rect: RectSpec) {
    let canvas = game.sprite_editor.clone();
    draw_panel_chrome(
        rect,
        "Sprite Editor",
        &format!(
            "{}x{} | zoom {} | {}",
            canvas.width,
            canvas.height,
            canvas.zoom,
            canvas
                .last_path
                .as_ref()
                .and_then(|path| path.file_name())
                .and_then(|value| value.to_str())
                .unwrap_or("unsaved")
        ),
    );
    if button(rect.x + 14.0, rect.y + 31.0, 76.0, 22.0, "New 16", false) {
        game.new_sprite_canvas(16, 16);
    }
    if button(rect.x + 98.0, rect.y + 31.0, 76.0, 22.0, "New 32", false) {
        game.new_sprite_canvas(32, 32);
    }
    if button(rect.x + 182.0, rect.y + 31.0, 68.0, 22.0, "Clear", false) {
        game.sprite_editor.begin_edit();
        game.sprite_editor.clear(SpriteColor::TRANSPARENT);
        game.sprite_editor.commit_edit();
    }
    if button(rect.x + 258.0, rect.y + 31.0, 66.0, 22.0, "Flip H", false) {
        game.sprite_editor.begin_edit();
        game.sprite_editor.flip_horizontal();
        game.sprite_editor.commit_edit();
    }
    if button(rect.x + 332.0, rect.y + 31.0, 66.0, 22.0, "Flip V", false) {
        game.sprite_editor.begin_edit();
        game.sprite_editor.flip_vertical();
        game.sprite_editor.commit_edit();
    }
    if button(rect.x + 406.0, rect.y + 31.0, 62.0, 22.0, "Rotate", false) {
        game.sprite_editor.begin_edit();
        game.sprite_editor.rotate_right();
        game.sprite_editor.commit_edit();
    }
    if button(rect.x + 476.0, rect.y + 31.0, 72.0, 22.0, "Save", false) {
        game.save_sprite_canvas_current("Sprite").ok();
    }
    if button(rect.x + 556.0, rect.y + 31.0, 64.0, 22.0, "Crop", false) {
        game.sprite_editor.begin_edit();
        if game.sprite_editor.crop_to_content(1) {
            game.sprite_editor.commit_edit();
            game.console.log("Sprite recortado al contenido", "SPRITE");
        }
    }
    if button(rect.x + 628.0, rect.y + 31.0, 74.0, 22.0, "Outline", false) {
        game.sprite_editor.begin_edit();
        let color = game.sprite_editor.active_color;
        let changed = game.sprite_editor.outline_alpha(color);
        game.sprite_editor.commit_edit();
        game.console
            .log(format!("Outline aplicado a {changed} pixeles"), "SPRITE");
    }
    if button(rect.x + 710.0, rect.y + 31.0, 54.0, 22.0, "Undo", false) {
        game.sprite_editor.undo();
    }
    if button(rect.x + 772.0, rect.y + 31.0, 54.0, 22.0, "Redo", false) {
        game.sprite_editor.redo();
    }
    let sheet_y = rect.y + 58.0;
    draw_text(
        "Sheet",
        rect.x + 14.0,
        sheet_y + 16.0,
        13.0,
        Color::from_rgba(125, 216, 205, 255),
    );
    if button(rect.x + 70.0, sheet_y, 70.0, 21.0, "2x1", false) {
        save_sprite_sheet_from_canvas(game, state, 2, 1);
    }
    if button(rect.x + 146.0, sheet_y, 70.0, 21.0, "4x1", false) {
        save_sprite_sheet_from_canvas(game, state, 4, 1);
    }
    if button(rect.x + 222.0, sheet_y, 70.0, 21.0, "4x2", false) {
        save_sprite_sheet_from_canvas(game, state, 4, 2);
    }

    let grid_size = (rect.h - 122.0).min(rect.w * 0.45).max(96.0);
    let pixel = (grid_size / canvas.width.max(canvas.height) as f32)
        .floor()
        .max(2.0);
    let origin_x = rect.x + 18.0;
    let origin_y = rect.y + 98.0;
    draw_rect(
        RectSpec {
            x: origin_x - 6.0,
            y: origin_y - 6.0,
            w: canvas.width as f32 * pixel + 12.0,
            h: canvas.height as f32 * pixel + 12.0,
        },
        Color::from_rgba(24, 28, 36, 255),
    );
    for y in 0..canvas.height {
        for x in 0..canvas.width {
            let color = canvas
                .get_pixel(x, y)
                .map(sprite_editor_color)
                .unwrap_or(Color::from_rgba(0, 0, 0, 0));
            let sx = origin_x + x as f32 * pixel;
            let sy = origin_y + y as f32 * pixel;
            draw_rectangle(sx, sy, pixel, pixel, checker_color(x, y));
            draw_rectangle(sx, sy, pixel, pixel, color);
            draw_rectangle_lines(sx, sy, pixel, pixel, 1.0, Color::from_rgba(0, 0, 0, 50));
        }
    }
    let frame_w = canvas.width.clamp(1, 16);
    let frame_h = canvas.height.clamp(1, 16);
    let draft = canvas.animation_clip_draft("SpriteDraft", frame_w, frame_h, 8.0);
    let sample = draft.sample_at(get_time() as f32, true);
    draw_sprite_frame_overlay(
        &draft,
        sample.as_ref().map(|sample| sample.frame_index),
        origin_x,
        origin_y,
        pixel,
    );

    let (mx, my) = mouse_position();
    if is_mouse_button_released(MouseButton::Left) || is_mouse_button_released(MouseButton::Right) {
        game.sprite_editor.commit_edit();
    }
    let local_x = ((mx - origin_x) / pixel).floor() as i32;
    let local_y = ((my - origin_y) / pixel).floor() as i32;
    if local_x >= 0
        && local_y >= 0
        && (local_x as u32) < canvas.width
        && (local_y as u32) < canvas.height
    {
        if is_mouse_button_pressed(MouseButton::Left) || is_mouse_button_pressed(MouseButton::Right)
        {
            game.sprite_editor.begin_edit();
        }
        if is_mouse_button_down(MouseButton::Left) {
            game.paint_sprite_pixel(
                local_x as u32,
                local_y as u32,
                game.sprite_editor.active_color,
            );
        }
        if is_mouse_button_down(MouseButton::Right) {
            game.paint_sprite_pixel(
                local_x as u32,
                local_y as u32,
                game.sprite_editor.secondary_color,
            );
        }
        if is_mouse_button_released(MouseButton::Left)
            || is_mouse_button_released(MouseButton::Right)
        {
            game.sprite_editor.commit_edit();
        }
    }

    let inspector = RectSpec {
        x: origin_x + canvas.width as f32 * pixel + 26.0,
        y: origin_y - 6.0,
        w: rect.w - (origin_x + canvas.width as f32 * pixel + 40.0 - rect.x),
        h: rect.h - 84.0,
    };
    draw_rect(inspector, Color::from_rgba(21, 25, 34, 238));
    draw_rectangle_lines(
        inspector.x,
        inspector.y,
        inspector.w,
        inspector.h,
        1.0,
        Color::from_rgba(62, 74, 96, 255),
    );
    draw_text(
        "Palette",
        inspector.x + 12.0,
        inspector.y + 24.0,
        16.0,
        Color::from_rgba(232, 238, 248, 255),
    );
    let swatches = [
        SpriteColor {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        },
        SpriteColor {
            r: 32,
            g: 36,
            b: 44,
            a: 255,
        },
        SpriteColor {
            r: 88,
            g: 196,
            b: 255,
            a: 255,
        },
        SpriteColor {
            r: 255,
            g: 105,
            b: 105,
            a: 255,
        },
        SpriteColor {
            r: 120,
            g: 225,
            b: 145,
            a: 255,
        },
        SpriteColor {
            r: 255,
            g: 212,
            b: 120,
            a: 255,
        },
        SpriteColor::TRANSPARENT,
    ];
    let mut sx = inspector.x + 12.0;
    let mut sy = inspector.y + 42.0;
    for color in swatches {
        let swatch = RectSpec {
            x: sx,
            y: sy,
            w: 28.0,
            h: 28.0,
        };
        draw_rectangle(swatch.x, swatch.y, swatch.w, swatch.h, checker_color(0, 0));
        draw_rectangle(
            swatch.x,
            swatch.y,
            swatch.w,
            swatch.h,
            sprite_editor_color(color),
        );
        draw_rectangle_lines(
            swatch.x,
            swatch.y,
            swatch.w,
            swatch.h,
            1.0,
            Color::from_rgba(88, 100, 124, 255),
        );
        if contains_mouse(swatch) && is_mouse_button_pressed(MouseButton::Left) {
            game.sprite_editor.active_color = color;
        }
        sx += 36.0;
        if sx + 28.0 > inspector.x + inspector.w - 12.0 {
            sx = inspector.x + 12.0;
            sy += 36.0;
        }
    }
    let timeline_rect = RectSpec {
        x: inspector.x + 12.0,
        y: (sy + 42.0).min(inspector.y + inspector.h - 132.0),
        w: inspector.w - 24.0,
        h: 92.0,
    };
    draw_sprite_animation_timeline(&draft, sample.as_ref(), timeline_rect);
    draw_text(
        "Left paints active color; right paints secondary.",
        inspector.x + 12.0,
        inspector.y + inspector.h - 26.0,
        13.0,
        Color::from_rgba(145, 162, 190, 255),
    );
}

fn save_sprite_sheet_from_canvas(
    game: &mut Game,
    state: &mut EditorState,
    columns: u32,
    rows: u32,
) {
    let columns = columns.max(1).min(game.sprite_editor.width.max(1));
    let rows = rows.max(1).min(game.sprite_editor.height.max(1));
    let frame_width = (game.sprite_editor.width / columns).max(1);
    let frame_height = (game.sprite_editor.height / rows).max(1);
    let base = game
        .sprite_editor
        .last_path
        .as_ref()
        .and_then(|path| path.file_stem())
        .and_then(|value| value.to_str())
        .unwrap_or("SpriteSheetLab")
        .to_string();
    let name = format!("{base}_{columns}x{rows}");
    let result: io::Result<(PathBuf, PathBuf, PathBuf)> = (|| {
        let image_path = game.save_sprite_canvas(&name)?;
        let sheet_meta =
            SpriteSheetImporter::build_metadata(&image_path, frame_width, frame_height, 0, 0)?;
        let sheet_path = SpriteSheetImporter::write_sidecar(&image_path, &sheet_meta)?;
        let texture_ref = relative_project_path(game, &image_path);
        let frames = SpriteFrames2D::grid_slice(
            name.clone(),
            texture_ref.clone(),
            columns,
            rows,
            frame_width,
            frame_height,
            8.0,
        );
        let animation_folder = AssetTools::create_special_folder(&game.project_path, "animations")?;
        let frames_path =
            AssetTools::unique_path(&animation_folder, &format!("{name}.spriteframes"));
        let frames_value = serde_json::to_value(frames).map_err(io::Error::other)?;
        AssetTools::write_json(&frames_path, &frames_value)?;
        let sprite_manifest = game.create_sprite_import_asset(&name, &texture_ref)?;
        patch_sprite_import_links(
            game,
            &sprite_manifest,
            Some(&sheet_path),
            Some(&frames_path),
            (columns * rows) as usize,
        )?;
        Ok((sprite_manifest, sheet_path, frames_path))
    })();

    match result {
        Ok((sprite_manifest, _sheet_path, frames_path)) => {
            state.selected_asset_path = Some(relative_project_path(game, &sprite_manifest));
            state.content_source = "Sprites".to_string();
            state.content_type_filter = Some("Sprite".to_string());
            game.refresh_assets().ok();
            game.console.log(
                format!(
                    "Plancha {columns}x{rows} creada: {}",
                    relative_project_path(game, &frames_path)
                ),
                "SPRITE",
            );
        }
        Err(error) => game.console.log(
            format!("No se pudo crear plancha de sprites: {error}"),
            "ERROR",
        ),
    }
}

fn draw_sprite_frame_overlay(
    draft: &SpriteAnimationClipDraft,
    active_frame: Option<usize>,
    origin_x: f32,
    origin_y: f32,
    pixel: f32,
) {
    for marker in &draft.timeline_preview().markers {
        let [x, y, w, h] = marker.source_rect;
        let rect = RectSpec {
            x: origin_x + x as f32 * pixel,
            y: origin_y + y as f32 * pixel,
            w: w as f32 * pixel,
            h: h as f32 * pixel,
        };
        let active = active_frame == Some(marker.frame_index);
        draw_rectangle_lines(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            if active { 2.5 } else { 1.0 },
            if active {
                Color::from_rgba(255, 214, 92, 255)
            } else {
                Color::from_rgba(100, 185, 255, 180)
            },
        );
        if active {
            draw_text(
                &marker.label,
                rect.x + 3.0,
                rect.y + 12.0,
                11.0,
                Color::from_rgba(255, 245, 190, 255),
            );
        }
    }
}

fn draw_sprite_animation_timeline(
    draft: &SpriteAnimationClipDraft,
    sample: Option<&SpriteAnimationPlaybackSample>,
    rect: RectSpec,
) {
    draw_rect(rect, Color::from_rgba(15, 18, 25, 235));
    draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 1.0, ui_line_soft());
    let preview = draft.timeline_preview();
    draw_text(
        &format!(
            "Animation Draft | {} frames | {:.2}s @ {:.0} fps",
            preview.frame_count, preview.total_duration, preview.fps
        ),
        rect.x + 10.0,
        rect.y + 20.0,
        13.0,
        ui_text(),
    );
    let bar = RectSpec {
        x: rect.x + 10.0,
        y: rect.y + 34.0,
        w: rect.w - 20.0,
        h: 24.0,
    };
    draw_rectangle(
        bar.x,
        bar.y,
        bar.w,
        bar.h,
        Color::from_rgba(25, 30, 40, 255),
    );
    for marker in &preview.markers {
        let mx = bar.x + bar.w * marker.normalized_start;
        let mw = (bar.w * (marker.normalized_end - marker.normalized_start)).max(2.0);
        let active = sample.is_some_and(|sample| sample.frame_index == marker.frame_index);
        draw_rectangle(
            mx,
            bar.y,
            mw - 1.0,
            bar.h,
            if active {
                Color::from_rgba(255, 204, 92, 255)
            } else {
                Color::from_rgba(72, 118, 176, 230)
            },
        );
        if mw > 22.0 {
            draw_text(
                &marker.label,
                mx + 4.0,
                bar.y + 16.0,
                10.0,
                Color::from_rgba(245, 248, 255, 255),
            );
        }
    }
    if let Some(sample) = sample {
        let px = bar.x + bar.w * sample.normalized_time;
        draw_line(
            px,
            bar.y - 4.0,
            px,
            bar.y + bar.h + 4.0,
            2.0,
            Color::from_rgba(255, 245, 190, 255),
        );
        draw_text(
            &format!(
                "Frame {} | rect {:?}",
                sample.frame_index + 1,
                sample.source_rect
            ),
            rect.x + 10.0,
            rect.y + 76.0,
            12.0,
            Color::from_rgba(185, 205, 232, 255),
        );
    }
    for warning in preview.warnings.iter().take(1) {
        draw_text(
            warning,
            rect.x + 10.0,
            rect.y + rect.h - 10.0,
            11.0,
            Color::from_rgba(255, 178, 128, 255),
        );
    }
}

fn sprite_editor_color(color: SpriteColor) -> Color {
    Color::from_rgba(color.r, color.g, color.b, color.a)
}

fn checker_color(x: u32, y: u32) -> Color {
    if (x + y).is_multiple_of(2) {
        Color::from_rgba(42, 47, 58, 255)
    } else {
        Color::from_rgba(30, 34, 42, 255)
    }
}

fn frame_health_label(health: crate::engine::diagnostics::FrameHealth) -> &'static str {
    match health {
        crate::engine::diagnostics::FrameHealth::Stable => "stable",
        crate::engine::diagnostics::FrameHealth::OverBudget => "over budget",
        crate::engine::diagnostics::FrameHealth::Saturated => "fixed saturated",
    }
}

fn frame_health_color(health: crate::engine::diagnostics::FrameHealth) -> Color {
    match health {
        crate::engine::diagnostics::FrameHealth::Stable => Color::from_rgba(80, 210, 150, 255),
        crate::engine::diagnostics::FrameHealth::OverBudget => Color::from_rgba(255, 198, 92, 255),
        crate::engine::diagnostics::FrameHealth::Saturated => Color::from_rgba(255, 110, 110, 255),
    }
}

fn draw_profiler_panel(game: &mut Game, rect: RectSpec) {
    draw_text(
        &format!("Profiler {}", crate::ENGINE_VERSION),
        rect.x + 14.0,
        rect.y + 20.0,
        18.0,
        WHITE,
    );
    let mut x = rect.x + 120.0;
    let stats = [
        format!("FPS {:.0}", game.diagnostics.fps),
        format!("Frame {:.2} ms", game.diagnostics.frame_time_ms),
        format!(
            "Health {}",
            frame_health_label(game.diagnostics.last_frame.health)
        ),
        format!("Entities {}", game.runtime_world.units.len()),
        format!("Mode {}", game.mode),
    ];
    for item in stats {
        draw_text(
            &item,
            x,
            rect.y + 20.0,
            15.0,
            Color::from_rgba(205, 212, 226, 255),
        );
        x += 126.0;
    }

    let health = &game.diagnostics.last_frame;
    let health_color = frame_health_color(health.health);
    let health_width = if rect.w > 940.0 {
        (rect.w - 342.0).max(300.0)
    } else {
        rect.w - 28.0
    };
    let health_rect = RectSpec {
        x: rect.x + 14.0,
        y: rect.y + 38.0,
        w: health_width,
        h: 50.0,
    };
    draw_gradient_rect(
        health_rect,
        Color::from_rgba(23, 31, 44, 255),
        Color::from_rgba(13, 17, 25, 255),
    );
    draw_rectangle_lines(
        health_rect.x,
        health_rect.y,
        health_rect.w,
        health_rect.h,
        1.0,
        ui_line_soft(),
    );
    draw_rect(
        RectSpec {
            x: health_rect.x,
            y: health_rect.y,
            w: 4.0,
            h: health_rect.h,
        },
        health_color,
    );
    draw_text(
        frame_health_label(health.health),
        health_rect.x + 12.0,
        health_rect.y + 18.0,
        15.0,
        health_color,
    );
    let summary = game.diagnostics.health_summary();
    draw_text(
        &ellipsize(&summary, 112),
        health_rect.x + 132.0,
        health_rect.y + 18.0,
        13.0,
        Color::from_rgba(218, 225, 238, 255),
    );
    let pacing = format!(
        "fixed {} x {:.2} ms | dropped {:.2} ms | alpha {:.2}",
        health.fixed_steps,
        health.fixed_delta_ms,
        health.dropped_time_ms,
        health.interpolation_alpha
    );
    draw_text(
        &ellipsize(&pacing, 74),
        health_rect.x + 132.0,
        health_rect.y + 38.0,
        12.0,
        Color::from_rgba(160, 178, 202, 255),
    );

    let mut y = rect.y + 106.0;
    draw_text(
        "Counters",
        rect.x + 14.0,
        y,
        16.0,
        Color::from_rgba(180, 220, 255, 255),
    );
    y += 20.0;
    for (name, value) in &game.profiler.counters {
        draw_text(
            &format!("{name}: {value}"),
            rect.x + 14.0,
            y,
            15.0,
            Color::from_rgba(220, 225, 238, 255),
        );
        y += 18.0;
    }

    let mut y2 = rect.y + 106.0;
    draw_text(
        "Gameplay",
        rect.x + 220.0,
        y2,
        16.0,
        Color::from_rgba(180, 220, 255, 255),
    );
    y2 += 20.0;
    for (name, value) in &game.gameplay_system.stats {
        draw_text(
            &format!("{name}: {value}"),
            rect.x + 220.0,
            y2,
            15.0,
            Color::from_rgba(220, 225, 238, 255),
        );
        y2 += 18.0;
    }

    let mut y3 = rect.y + 106.0;
    draw_text(
        "Physics",
        rect.x + 430.0,
        y3,
        16.0,
        Color::from_rgba(180, 220, 255, 255),
    );
    y3 += 20.0;
    for (name, value) in &game.physics_system.stats {
        draw_text(
            &format!("{name}: {value}"),
            rect.x + 430.0,
            y3,
            15.0,
            Color::from_rgba(220, 225, 238, 255),
        );
        y3 += 18.0;
    }

    let mut y4 = rect.y + 106.0;
    draw_text(
        "Systems",
        rect.x + 650.0,
        y4,
        16.0,
        Color::from_rgba(180, 220, 255, 255),
    );
    y4 += 20.0;
    if let Some((name, ms)) = game.profiler.slowest_system() {
        let total = game.profiler.systems_time_total_ms();
        draw_text(
            &format!("Peak: {name} {ms:.1} ms | Σ {total:.1} ms"),
            rect.x + 650.0,
            y4,
            13.0,
            Color::from_rgba(255, 210, 140, 255),
        );
        y4 += 17.0;
    }
    let actions = health.action_items();
    for action in actions.iter().take(2) {
        draw_text(
            &ellipsize(action, 44),
            rect.x + 650.0,
            y4,
            12.0,
            Color::from_rgba(255, 190, 145, 255),
        );
        y4 += 16.0;
    }
    for (name, value) in game.profiler.rows().iter().take(7) {
        draw_text(
            &format!("{name}: {value}"),
            rect.x + 650.0,
            y4,
            15.0,
            Color::from_rgba(220, 225, 238, 255),
        );
        y4 += 18.0;
    }

    if rect.w > 940.0 {
        draw_complex_game_checklist(
            game,
            RectSpec {
                x: rect.x + rect.w - 306.0,
                y: rect.y + 44.0,
                w: 292.0,
                h: rect.h - 54.0,
            },
        );
    }
}

#[derive(Debug, Clone)]
struct EditorChecklistItem {
    label: &'static str,
    ok: bool,
    detail: String,
}

fn draw_complex_game_checklist(game: &mut Game, rect: RectSpec) {
    draw_rect(rect, Color::from_rgba(16, 20, 28, 235));
    draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 1.0, ui_line_soft());
    draw_text(
        "Complex Game Checklist",
        rect.x + 10.0,
        rect.y + 21.0,
        15.0,
        Color::from_rgba(180, 220, 255, 255),
    );
    let items = complex_game_checklist(game);
    let ready = items.iter().filter(|item| item.ok).count();
    draw_text(
        &format!("{ready}/{} foundations ready", items.len()),
        rect.x + 10.0,
        rect.y + 40.0,
        12.0,
        ui_text_muted(),
    );
    let max_items = ((rect.h - 102.0) / 34.0).floor().clamp(1.0, 8.0) as usize;
    let mut y = rect.y + 63.0;
    for item in items.iter().take(max_items) {
        let color = if item.ok {
            Color::from_rgba(116, 220, 148, 255)
        } else {
            Color::from_rgba(255, 184, 104, 255)
        };
        draw_circle(rect.x + 14.0, y - 5.0, 4.0, color);
        draw_text(item.label, rect.x + 26.0, y, 12.0, ui_text());
        draw_text(
            &ellipsize(&item.detail, 33),
            rect.x + 26.0,
            y + 15.0,
            11.0,
            ui_text_muted(),
        );
        y += 34.0;
    }
    let button_y = rect.y + rect.h - 30.0;
    if button(
        rect.x + 10.0,
        button_y,
        rect.w - 20.0,
        22.0,
        "Prepare Foundations",
        false,
    ) {
        let report = game.prepare_complex_game_foundations();
        game.console.log(
            format!(
                "Checklist action: player +{}, systems +{}, ui {}",
                report.added_player_components.len(),
                report.added_system_components.len(),
                if report.created_ui_canvas {
                    "created"
                } else {
                    "ready"
                }
            ),
            "EDITOR",
        );
    }
}

fn complex_game_checklist(game: &Game) -> Vec<EditorChecklistItem> {
    let ui_elements: usize = ui_canvases_from_value(&game.ui_canvases)
        .iter()
        .map(|canvas| canvas.elements.len())
        .sum();
    let component_count = |component: &str| {
        game.runtime_world
            .units
            .iter()
            .filter(|entity| entity.get_component(component).is_some())
            .count()
    };
    let has_scripted_logic = game.runtime_world.units.iter().any(|entity| {
        entity.script.is_some()
            || !entity.scripts.is_empty()
            || entity.get_component("VisualScript").is_some()
    });
    let gameplay_components = [
        "Health",
        "Inventory",
        "Ability",
        "QuestLog",
        "EconomyWallet",
        "ProductionQueue",
    ]
    .iter()
    .map(|component| component_count(component))
    .sum::<usize>();
    let runtime_scale_components = [
        "RuntimeBudget2D",
        "WorldPartition2D",
        "ObjectPool2D",
        "SpawnDirector2D",
        "SaveShard2D",
    ]
    .iter()
    .map(|component| component_count(component))
    .sum::<usize>();
    vec![
        EditorChecklistItem {
            label: "Playable Actor",
            ok: component_count("CharacterController2D") > 0
                || game
                    .runtime_world
                    .units
                    .iter()
                    .any(|entity| entity.tag == "Player"),
            detail: format!(
                "{} controllers, {} Player tags",
                component_count("CharacterController2D"),
                game.runtime_world
                    .units
                    .iter()
                    .filter(|entity| entity.tag == "Player")
                    .count()
            ),
        },
        EditorChecklistItem {
            label: "Gameplay Loop",
            ok: gameplay_components >= 3,
            detail: format!("{gameplay_components} economy/combat/progression components"),
        },
        EditorChecklistItem {
            label: "AI / Navigation",
            ok: component_count("AIController") > 0 || component_count("NavAgent") > 0,
            detail: format!(
                "{} AI, {} nav agents",
                component_count("AIController"),
                component_count("NavAgent")
            ),
        },
        EditorChecklistItem {
            label: "Runtime UI",
            ok: ui_elements > 0 || component_count("UIElement") > 0,
            detail: format!(
                "{ui_elements} canvas widgets, {} legacy UI",
                component_count("UIElement")
            ),
        },
        EditorChecklistItem {
            label: "Save Path",
            ok: component_count("Saveable") > 0,
            detail: format!("{} saveable entities", component_count("Saveable")),
        },
        EditorChecklistItem {
            label: "Scripted Logic",
            ok: has_scripted_logic,
            detail: format!(
                "{} open visual graphs",
                game.programming.opened_graphs.len()
            ),
        },
        EditorChecklistItem {
            label: "Input Map",
            ok: game.input_map.actions.len() >= 6,
            detail: format!("{} actions configured", game.input_map.actions.len()),
        },
        EditorChecklistItem {
            label: "Runtime Scale",
            ok: runtime_scale_components >= 3,
            detail: format!("{runtime_scale_components}/5 scale systems"),
        },
    ]
}

const COMMAND_PALETTE_ITEMS: &[(&str, &str)] = &[
    ("Spawn game object", "spawn_object"),
    ("Spawn sprite entity", "spawn_sprite"),
    ("Spawn advanced unit", "spawn_unit"),
    ("Spawn enemy AI", "spawn_enemy"),
    ("Spawn resource node", "spawn_resource"),
    ("Spawn RTS command center", "spawn_rts_base"),
    ("Queue worker on selected building", "queue_worker"),
    ("Place Barracks construction site", "place_barracks"),
    ("Create TopDown starter scene", "starter_topdown"),
    ("Create Platformer starter scene", "starter_platformer"),
    ("Create RTS skirmish scene", "rts_skirmish"),
    ("Prepare complex game foundations", "prepare_complex"),
    ("Toggle Play Mode snapshot", "toggle_play"),
    ("Open Play Window", "play_window"),
    ("Open Launcher", "open_launcher"),
    ("Open detached script window", "script_window"),
    ("Open preferences", "preferences"),
    ("Open script in VS Code", "open_vscode"),
    ("Add Health to selected", "add_health"),
    ("Add NavAgent to selected", "add_nav"),
    ("Add VisualScript to selected", "add_visual"),
    ("Attach Graph LogAndMove", "attach_graph_log"),
    ("Attach Graph PlayerVitalMovement", "attach_graph_player"),
    ("Attach Graph HealthCombat", "attach_graph_combat"),
    ("Attach Graph HealthPickup", "attach_graph_health"),
    ("Attach Graph RTSOrder", "attach_graph_rts"),
    (
        "Attach Graph InventoryEconomyLoop",
        "attach_graph_inventory",
    ),
    ("Attach Graph QuestAbilityLoop", "attach_graph_quest"),
    (
        "Attach Graph RTSProductionEconomy",
        "attach_graph_rts_economy",
    ),
    ("Open Blueprint Picker", "blueprint_picker"),
    ("Create visual graph asset", "create_graph"),
    (
        "Create inventory economy graph asset",
        "create_graph_inventory",
    ),
    ("Create quest ability graph asset", "create_graph_quest"),
    (
        "Create RTS production graph asset",
        "create_graph_rts_economy",
    ),
    ("Create sound cue asset", "asset_sound"),
    ("Create material asset", "asset_material"),
    ("Create UI Canvas HUD", "ui_canvas_hud"),
    ("Create UI Canvas label", "ui_canvas_label"),
    ("Save selected as prefab", "save_prefab"),
    ("Create prefab variant", "variant_prefab"),
    ("Instantiate first prefab", "instantiate_prefab"),
    ("Apply selected prefab source", "apply_prefab"),
    ("Revert selected prefab", "revert_prefab"),
    ("Detach selected prefab", "detach_prefab"),
    ("Open Scene Browser", "scene_browser"),
    ("Open detached Sprite Editor", "sprite_editor"),
    ("Open Python automation tools", "python_tools"),
    ("Export project package", "export_project_zip"),
    ("Import project package", "import_project_zip"),
    ("Workspace world", "workspace_world"),
    ("Workspace scripting", "workspace_script"),
    ("Workspace prefab", "workspace_prefab"),
    ("Workspace profiling", "workspace_profile"),
    ("Duplicate selected", "duplicate"),
    ("Delete selected", "delete"),
    ("Save project", "save_project"),
    ("Save scene", "save"),
    ("New scene", "new_scene"),
    ("Recover autosave", "recover_autosave"),
    ("Validate project", "validate"),
    ("Refresh asset database", "refresh"),
    ("Align selection left", "align_left"),
    ("Align selection center X", "align_center_x"),
    ("Align selection top", "align_top"),
    ("Align selection center Y", "align_center_y"),
    ("Distribute selection horizontally", "distribute_x"),
    ("Distribute selection vertically", "distribute_y"),
    ("Group selected objects", "group_selection"),
    ("Ungroup selected objects", "ungroup_selection"),
    ("Lock or unlock selected layer", "toggle_layer_lock"),
    ("Show or hide selected layer", "toggle_layer_visibility"),
    ("Move selection to next layer", "selection_next_layer"),
    ("Run Python scene production report", "python_scene_report"),
    ("Show 0.9.3.4 foundation status", "foundation_0934"),
    ("Build manifest", "manifest"),
    ("Export runtime debug", "export_debug"),
    ("Export runtime release", "export_release"),
    ("Package debug", "package_debug"),
    ("Package release", "package_release"),
    ("Create RTS template files", "template_rts"),
    ("Create ActionRPG template files", "template_actionrpg"),
    ("Create Survival template files", "template_survival"),
    ("Cycle tilemap layer", "cycle_layer"),
    ("Clear console", "clear_console"),
];

fn draw_command_palette(game: &mut Game, state: &mut EditorState, sw: f32, sh: f32) {
    let panel_h = (sh - 50.0).clamp(560.0, 760.0);
    let panel = RectSpec {
        x: sw * 0.5 - 300.0,
        y: sh * 0.5 - panel_h * 0.5,
        w: 600.0,
        h: panel_h,
    };
    draw_rectangle(0.0, 0.0, sw, sh, Color::from_rgba(0, 0, 0, 145));
    draw_surface(panel, true);
    draw_panel_header(panel, "Command Palette", "instant engine actions");
    draw_text(
        "Escribe para buscar. Enter ejecuta. Esc cierra.",
        panel.x + 18.0,
        panel.y + 54.0,
        15.0,
        ui_text_muted(),
    );

    let search = RectSpec {
        x: panel.x + 14.0,
        y: panel.y + 66.0,
        w: panel.w - 28.0,
        h: 30.0,
    };
    draw_gradient_rect(
        search,
        Color::from_rgba(23, 31, 46, 255),
        Color::from_rgba(12, 16, 24, 255),
    );
    draw_rectangle_lines(search.x, search.y, search.w, search.h, 1.0, ui_line());
    let query_label = if state.command_palette.query.is_empty() {
        "buscar comando, blueprint, inventario, economia..."
    } else {
        &state.command_palette.query
    };
    draw_text(
        &ellipsize(query_label, 68),
        search.x + 10.0,
        search.y + 21.0,
        15.0,
        if state.command_palette.query.is_empty() {
            ui_text_muted()
        } else {
            ui_text()
        },
    );

    let commands = filtered_palette_commands(&state.command_palette.query);
    let mut y = panel.y + 116.0;
    let max_rows = ((panel.h - 158.0) / 22.0).floor().max(1.0) as usize;
    let first_visible = state
        .command_palette
        .selected_index
        .saturating_add(1)
        .saturating_sub(max_rows);
    for (index, (label, command)) in commands
        .into_iter()
        .enumerate()
        .skip(first_visible)
        .take(max_rows)
    {
        let row = RectSpec {
            x: panel.x + 14.0,
            y: y - 18.0,
            w: panel.w - 28.0,
            h: 23.0,
        };
        let hovered = contains_mouse(row);
        let selected = state.command_palette.selected_index == index;
        draw_rect(
            row,
            if hovered || selected {
                Color::from_rgba(41, 58, 82, 255)
            } else {
                Color::from_rgba(23, 30, 43, 255)
            },
        );
        draw_rectangle(
            row.x,
            row.y,
            3.0,
            row.h,
            if hovered || selected {
                ui_accent()
            } else {
                ui_line_soft()
            },
        );
        draw_text(label, row.x + 10.0, y, 16.0, ui_text());
        if hovered && is_mouse_button_pressed(MouseButton::Left) {
            run_palette_command(game, state, command);
            state.command_palette.record_execution(command);
            state.command_palette.close();
        }
        y += 22.0;
    }

    let types = advanced_component_types()
        .iter()
        .take(8)
        .copied()
        .collect::<Vec<_>>()
        .join(", ");
    draw_text(
        &ellipsize(&format!("Advanced components: {types}..."), 64),
        panel.x + 18.0,
        panel.y + panel.h - 18.0,
        14.0,
        ui_text_muted(),
    );
}

fn filtered_palette_commands(query: &str) -> Vec<(&'static str, &'static str)> {
    let searchable = COMMAND_PALETTE_ITEMS
        .iter()
        .map(|(label, command)| format!("{} {} {}", label, command, command.replace('_', " ")))
        .collect::<Vec<_>>();
    fuzzy_rank(query, &searchable, searchable.len())
        .into_iter()
        .map(|result| COMMAND_PALETTE_ITEMS[result.index])
        .collect()
}

fn run_palette_command(game: &mut Game, state: &mut EditorState, command: &str) {
    match command {
        "align_left" => align_selected_entities(game, AlignMode2D::Left),
        "align_center_x" => align_selected_entities(game, AlignMode2D::CenterX),
        "align_top" => align_selected_entities(game, AlignMode2D::Top),
        "align_center_y" => align_selected_entities(game, AlignMode2D::CenterY),
        "distribute_x" => align_selected_entities(game, AlignMode2D::DistributeX),
        "distribute_y" => align_selected_entities(game, AlignMode2D::DistributeY),
        "group_selection" => group_selected_entities(game),
        "ungroup_selection" => ungroup_selected_entities(game),
        "toggle_layer_lock" => toggle_selected_layer_lock(game),
        "toggle_layer_visibility" => toggle_selected_layer_visibility(game),
        "selection_next_layer" => move_selection_to_next_layer(game),
        "python_scene_report" => {
            run_python_tool(game, state, "scene_report");
            state.show_console = true;
            state.bottom_tab = BottomTab::Console;
        }
        "toggle_smart_snap" => state.smart_snap = !state.smart_snap,
        "toggle_collision_overlay" => state.show_collisions = !state.show_collisions,
        "toggle_camera_frame" => state.show_camera_frame = !state.show_camera_frame,
        "foundation_0934" => {
            let plan = crate::engine::update_0934::Engine0934FoundationPlan::current();
            game.console.log(
                format!(
                    "MiniForge {} | {:?} | launch_allowed={} | {}",
                    plan.version, plan.release_state, plan.launch_allowed, plan.focus
                ),
                "0.9.3.4",
            );
            for capability in plan.capabilities {
                game.console.log(
                    format!("{}: {}", capability.area, capability.foundation),
                    "FOUNDATION",
                );
            }
            state.show_console = true;
            state.bottom_tab = BottomTab::Console;
        }
        "spawn_unit" => {
            let (x, y) = spawn_position(game);
            game.spawn_unit("PlayerUnit", x, y);
        }
        "spawn_object" => {
            let (x, y) = spawn_position(game);
            game.spawn_game_object("GameObject", x, y);
        }
        "spawn_sprite" => {
            let (x, y) = spawn_position(game);
            game.spawn_sprite_entity("Sprite", "sprite", x, y);
        }
        "spawn_enemy" => {
            let (x, y) = spawn_position(game);
            game.spawn_enemy(x + 2.0, y);
        }
        "spawn_resource" => {
            let (x, y) = spawn_position(game);
            game.spawn_resource(x, y + 2.0);
        }
        "spawn_rts_base" => {
            let (x, y) = spawn_position(game);
            game.spawn_rts_building("CommandCenter", x, y, 1);
        }
        "queue_worker" => queue_worker_selected(game),
        "place_barracks" => place_barracks(game),
        "starter_topdown" => {
            game.create_topdown_starter();
        }
        "starter_platformer" => {
            game.create_platformer_starter();
        }
        "rts_skirmish" => game.create_rts_skirmish(),
        "prepare_complex" => {
            game.prepare_complex_game_foundations();
            state.show_console = true;
            state.bottom_tab = BottomTab::Profiler;
            state.show_inspector = true;
        }
        "toggle_play" => toggle_play_mode(game, state),
        "add_health" => {
            if let Some(id) = selected_id(game) {
                add_component(game, id, "Health");
            }
        }
        "add_nav" => {
            if let Some(id) = selected_id(game) {
                add_component(game, id, "NavAgent");
            }
        }
        "add_visual" => {
            if let Some(id) = selected_id(game) {
                game.add_visual_script_template(id, "LogAndMove");
            }
        }
        "attach_graph_log" => {
            game.attach_program_template_to_selected("LogAndMove");
            state.show_console = true;
            state.bottom_tab = BottomTab::Programming;
        }
        "attach_graph_player" => {
            game.attach_program_template_to_selected("PlayerVitalMovement");
            state.show_console = true;
            state.bottom_tab = BottomTab::Programming;
        }
        "attach_graph_combat" => {
            game.attach_program_template_to_selected("HealthCombat");
            state.show_console = true;
            state.bottom_tab = BottomTab::Programming;
        }
        "attach_graph_health" => {
            game.attach_program_template_to_selected("HealthPickup");
            state.show_console = true;
            state.bottom_tab = BottomTab::Programming;
        }
        "attach_graph_rts" => {
            game.attach_program_template_to_selected("RTSOrder");
            state.show_console = true;
            state.bottom_tab = BottomTab::Programming;
        }
        "attach_graph_inventory" => {
            game.attach_program_template_to_selected("InventoryEconomyLoop");
            state.show_console = true;
            state.bottom_tab = BottomTab::Programming;
            state.script_window_open = true;
        }
        "attach_graph_quest" => {
            game.attach_program_template_to_selected("QuestAbilityLoop");
            state.show_console = true;
            state.bottom_tab = BottomTab::Programming;
            state.script_window_open = true;
        }
        "attach_graph_rts_economy" => {
            game.attach_program_template_to_selected("RTSProductionEconomy");
            state.show_console = true;
            state.bottom_tab = BottomTab::Programming;
            state.script_window_open = true;
        }
        "create_graph" => {
            if let Ok(path) = game.create_program_asset("LogAndMove") {
                set_open_file_editor_state(state, &path);
            }
            state.show_console = true;
            state.bottom_tab = BottomTab::Programming;
        }
        "create_graph_inventory" => {
            if let Ok(path) = game.create_program_asset("InventoryEconomyLoop") {
                set_open_file_editor_state(state, &path);
                state.script_window_open = true;
            }
            state.show_console = true;
            state.bottom_tab = BottomTab::Programming;
        }
        "create_graph_quest" => {
            if let Ok(path) = game.create_program_asset("QuestAbilityLoop") {
                set_open_file_editor_state(state, &path);
                state.script_window_open = true;
            }
            state.show_console = true;
            state.bottom_tab = BottomTab::Programming;
        }
        "create_graph_rts_economy" => {
            if let Ok(path) = game.create_program_asset("RTSProductionEconomy") {
                set_open_file_editor_state(state, &path);
                state.script_window_open = true;
            }
            state.show_console = true;
            state.bottom_tab = BottomTab::Programming;
        }
        "blueprint_picker" => {
            state.blueprint_picker_open = true;
            state.show_console = true;
            state.bottom_tab = BottomTab::Programming;
        }
        "asset_sound" => {
            game.create_sound_cue_asset("NewCue", "assets/audio/new.wav")
                .ok();
            state.show_console = true;
            state.bottom_tab = BottomTab::Assets;
        }
        "asset_material" => {
            game.create_material_asset("SpriteMaterial").ok();
            state.show_console = true;
            state.bottom_tab = BottomTab::Assets;
        }
        "ui_canvas_hud" => {
            game.ensure_default_ui_canvas_scene_data();
            game.console
                .log("Canvas HUD por defecto listo en la escena", "UI");
            state.show_inspector = true;
        }
        "ui_canvas_label" => {
            game.add_ui_canvas_scene_label("Nuevo label");
            game.console
                .log("Label de UI Canvas añadido al primer canvas", "UI");
            state.show_inspector = true;
        }
        "save_prefab" => {
            game.save_selected_as_prefab().ok();
            state.show_console = true;
            state.bottom_tab = BottomTab::Prefabs;
        }
        "variant_prefab" => {
            game.create_selected_prefab_variant().ok();
            state.show_console = true;
            state.bottom_tab = BottomTab::Prefabs;
        }
        "instantiate_prefab" => {
            let (x, y) = spawn_position(game);
            game.instantiate_first_prefab(x, y).ok();
            state.show_console = true;
            state.bottom_tab = BottomTab::Prefabs;
        }
        "apply_prefab" => {
            game.apply_selected_to_prefab_source().ok();
            state.show_console = true;
            state.bottom_tab = BottomTab::Prefabs;
        }
        "revert_prefab" => {
            game.revert_selected_prefab_instance().ok();
            state.show_console = true;
            state.bottom_tab = BottomTab::Prefabs;
        }
        "detach_prefab" => {
            game.detach_selected_prefab_instance();
            state.show_console = true;
            state.bottom_tab = BottomTab::Prefabs;
        }
        "workspace_world" => game.set_workspace_mode(WorkspaceMode::WorldBuilding),
        "workspace_script" => {
            game.set_workspace_mode(WorkspaceMode::Scripting);
            state.show_console = true;
            state.bottom_tab = BottomTab::Programming;
        }
        "workspace_prefab" => {
            game.set_workspace_mode(WorkspaceMode::PrefabEditing);
            state.show_console = true;
            state.bottom_tab = BottomTab::Prefabs;
        }
        "workspace_profile" => {
            game.set_workspace_mode(WorkspaceMode::Profiling);
            state.show_console = true;
            state.bottom_tab = BottomTab::Profiler;
        }
        "duplicate" => duplicate_selected(game),
        "delete" => delete_selected(game),
        "save_project" => save_project(game),
        "save" => save_scene(game),
        "new_scene" => create_new_scene(game),
        "command_palette" => {
            state.command_palette.open();
        }
        "toggle_browser" => {
            state.show_console = !state.show_console;
            state.bottom_tab = BottomTab::Assets;
        }
        "scene_browser" => {
            state.show_console = true;
            state.bottom_tab = BottomTab::Scenes;
        }
        "sprite_editor" => {
            state.sprite_window_open = true;
            state.bottom_tab = BottomTab::Sprites;
        }
        "python_tools" => state.python_tools_open = true,
        "toggle_hierarchy" => state.show_hierarchy = !state.show_hierarchy,
        "toggle_inspector" => state.show_inspector = !state.show_inspector,
        "script_window" => {
            state.script_window_open = true;
            state.show_console = true;
            state.bottom_tab = BottomTab::Programming;
        }
        "preferences" => {
            state.preferences_open = true;
            state.menu_bar.close();
            state.command_palette.close();
        }
        "open_vscode" => {
            open_current_file_in_external_editor(game, state);
        }
        "play_window" => {
            if game.mode == "PLAY" || state.external_play_child.is_some() {
                state.play_window_open = true;
            } else {
                launch_play_window(game, state);
            }
        }
        "open_launcher" => {
            state.launcher_overlay = Some(new_launcher_state());
            state.command_palette.close();
        }
        "validate" => {
            game.validate_project();
        }
        "refresh" => {
            refresh_assets(game);
            state.show_console = true;
            state.bottom_tab = BottomTab::Assets;
        }
        "manifest" => build_manifest(game),
        "export_debug" => export_runtime(game, ExportProfile::Debug),
        "export_release" => export_runtime(game, ExportProfile::Release),
        "export_project_zip" => export_project_zip(game),
        "import_project_zip" => import_project_zip(game),
        "package_debug" => {
            if let Err(e) = game.package_distributable(ExportProfile::Debug, "game") {
                game.console.log(format!("Package: {e}"), "ERROR");
            }
        }
        "package_release" => {
            if let Err(e) = game.package_distributable(ExportProfile::Release, "game") {
                game.console.log(format!("Package: {e}"), "ERROR");
            }
        }
        "recover_autosave" => {
            if let Err(e) = game.recover_from_autosave() {
                game.console.log(e, "ERROR");
            } else {
                state.show_console = true;
            }
        }
        "template_rts" => create_template(game, "RTS"),
        "template_actionrpg" => create_template(game, "ActionRPG"),
        "template_survival" => create_template(game, "Survival"),
        "cycle_layer" => {
            game.cycle_tilemap_layer();
        }
        "clear_console" => game.console.clear(),
        _ => {}
    }
}

fn draw_status_bar(game: &Game, state: &EditorState, rect: RectSpec) {
    draw_gradient_rect(
        rect,
        Color::from_rgba(23, 29, 40, 255),
        Color::from_rgba(13, 17, 25, 255),
    );
    draw_rectangle(rect.x, rect.y, rect.w, 1.0, ui_line_soft());
    let layer = &game.tilemap_layers.layers[game.tilemap_layers.active_layer].name;
    draw_text(
        &format!(
            "{} | FPS {:.0} | frame {:.2} ms avg {:.2} | health {} | {} | dock egui_dock | zoom {:.2} | camera {:.0},{:.0} | tool {} | layer {} | {}{}{}",
            crate::ENGINE_VERSION,
            game.diagnostics.fps,
            game.diagnostics.frame_time_ms,
            game.diagnostics.average_frame_time_ms,
            frame_health_label(game.diagnostics.last_frame.health),
            game.editor_workspace
                .performance_status(game.diagnostics.frame_time_ms),
            game.camera.zoom,
            game.camera.x,
            game.camera.y,
            state.toolbar.active_tool().label(),
            layer,
            if game.scene_dirty { "dirty" } else { "clean" },
            if game.mode == "PLAY" {
                format!(" | PLAY#{}", game.play_mode_manager.frame_count)
            } else {
                String::new()
            },
            if state.paused { " | paused" } else { "" }
        ),
        rect.x + 12.0,
        rect.y + 18.0,
        16.0,
        ui_text_muted(),
    );
    draw_status_pill(
        RectSpec {
            x: rect.x + rect.w - 246.0,
            y: rect.y + 4.0,
            w: 92.0,
            h: 18.0,
        },
        "UI 0.9.1.1",
        true,
    );
    draw_status_pill(
        RectSpec {
            x: rect.x + rect.w - 146.0,
            y: rect.y + 4.0,
            w: 126.0,
            h: 18.0,
        },
        game.mode.as_str(),
        game.mode == "PLAY",
    );
}

fn save_scene(game: &mut Game) {
    match game.save_scene() {
        Ok(()) => game.console.log("Escena guardada desde Rust", "SCENE"),
        Err(error) => game
            .console
            .log(format!("Error guardando escena: {error}"), "ERROR"),
    }
}

fn save_project(game: &mut Game) {
    match game.save_project() {
        Ok(()) => game.console.log("Proyecto guardado desde Rust", "PROJECT"),
        Err(error) => game
            .console
            .log(format!("Error guardando proyecto: {error}"), "ERROR"),
    }
}

fn create_new_scene(game: &mut Game) {
    let name = format!(
        "scene_{}",
        game.scene_manager
            .list_scenes()
            .map(|scenes| scenes.len() + 1)
            .unwrap_or(1)
    );
    match game.create_empty_scene(&name) {
        Ok(_) => game
            .console
            .log(format!("Escena creada y abierta vacia: {name}"), "SCENE"),
        Err(error) => game
            .console
            .log(format!("Error creando escena: {error}"), "ERROR"),
    }
}

fn refresh_assets(game: &mut Game) {
    match game.refresh_assets() {
        Ok(count) => game.console.log(
            format!("Asset database refrescada: {count} assets"),
            "ASSETS",
        ),
        Err(error) => game
            .console
            .log(format!("Error refrescando assets: {error}"), "ERROR"),
    }
}

fn build_manifest(game: &mut Game) {
    if let Err(error) = game.build_manifest() {
        game.console
            .log(format!("Error generando manifest: {error}"), "ERROR");
    }
}

fn export_runtime(game: &mut Game, profile: ExportProfile) {
    match game.export_runtime(profile) {
        Ok(report) => game.console.log(
            format!(
                "Export {}: {}",
                profile.label(),
                report.output_path.display()
            ),
            "BUILD",
        ),
        Err(error) => game
            .console
            .log(format!("Error exportando runtime: {error}"), "ERROR"),
    }
}

fn export_project_zip(game: &mut Game) {
    match game.export_project_package() {
        Ok(report) => game.console.log(
            format!(
                "Project package: {} ({} files, {} bytes)",
                report.archive_path.display(),
                report.files,
                report.bytes
            ),
            "PROJECT",
        ),
        Err(error) => game
            .console
            .report_error("PROJECT", "Export project package", error),
    }
}

fn import_project_zip(game: &mut Game) {
    let archive_path = game.project_paths.builds.join("import.mfpkg.zip");
    if !archive_path.exists() {
        game.console.log(
            format!("Import espera un paquete en {}", archive_path.display()),
            "WARNING",
        );
        return;
    }
    let destination_root = game
        .project_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("projects"));
    match game.import_project_package(&archive_path, destination_root) {
        Ok(report) => game.console.log(
            format!(
                "Imported package: {} ({} files)",
                report.project_path.display(),
                report.files
            ),
            "PROJECT",
        ),
        Err(error) => game
            .console
            .report_error("PROJECT", "Import project package", error),
    }
}

fn create_template(game: &mut Game, template_name: &str) {
    match game.create_project_template(template_name) {
        Ok(count) => game.console.log(
            format!("Template {template_name} listo: {count} archivos"),
            "PROJECT",
        ),
        Err(error) => game.console.log(
            format!("Error creando template {template_name}: {error}"),
            "ERROR",
        ),
    }
}

fn queue_worker_selected(game: &mut Game) {
    let Some(id) = selected_id(game) else {
        game.console
            .log("Selecciona un edificio con ProductionQueue.", "RTS");
        return;
    };
    let Some(entity) = game.get_entity_by_id_mut(id) else {
        return;
    };
    if RTSSystem::enqueue_production(
        entity,
        "Worker",
        "Worker",
        3.0,
        serde_json::json!({"Gold": 50.0}),
    ) {
        game.mark_scene_dirty("Queue Worker");
        game.console
            .log("Worker agregado a la cola de produccion", "RTS");
    } else {
        game.console
            .log("No se pudo agregar Worker: revisa recursos o cola.", "RTS");
    }
}

fn place_barracks(game: &mut Game) {
    let builder_ids = game.selected_units.clone();
    let (x, y) = spawn_position(game);
    if let Some(id) = game.try_place_rts_building(
        "Barracks",
        (x.round() as i32, y.round() as i32),
        1,
        builder_ids,
    ) {
        game.console
            .log(format!("Barracks construction site #{id} listo"), "RTS");
    } else {
        game.console
            .log("No hay espacio cercano para colocar Barracks.", "RTS");
    }
}

fn add_component(game: &mut Game, id: u64, component_type: &str) {
    if game.add_component_to_entity(id, component_type) {
        game.console
            .log(format!("{component_type} agregado a #{id}"), "EDITOR");
    } else {
        game.console.log(
            format!("No se pudo agregar {component_type} a #{id}"),
            "WARNING",
        );
    }
}

fn duplicate_selected(game: &mut Game) {
    let Some(id) = selected_id(game) else {
        return;
    };
    if let Some(new_id) = game.duplicate_entity(id) {
        game.console
            .log(format!("Entity #{id} duplicado como #{new_id}"), "EDITOR");
    }
}

fn delete_selected(game: &mut Game) {
    let Some(id) = selected_id(game) else {
        return;
    };
    if game.delete_entity(id) {
        game.console
            .log(format!("Entity #{id} eliminado"), "EDITOR");
    }
}

fn undo(game: &mut Game) {
    if let Some(label) = game.undo_editor_command() {
        game.console.log(format!("Undo: {label}"), "EDITOR");
    } else if let Some(label) = game.history.undo(&mut game.runtime_world.units) {
        game.clear_selection();
        game.sync_world();
        game.mark_scene_dirty("Undo");
        game.console.log(format!("Undo: {label}"), "EDITOR");
    }
}

fn redo(game: &mut Game) {
    if let Some(label) = game.redo_editor_command() {
        game.console.log(format!("Redo: {label}"), "EDITOR");
    } else if let Some(label) = game.history.redo(&mut game.runtime_world.units) {
        game.clear_selection();
        game.sync_world();
        game.mark_scene_dirty("Redo");
        game.console.log(format!("Redo: {label}"), "EDITOR");
    }
}

fn selected_id(game: &Game) -> Option<u64> {
    game.selected_units.first().copied()
}

fn entity_depth_in_units(entities: &[GameObject], entity_id: u64) -> usize {
    let mut depth = 0;
    let mut current = entities
        .iter()
        .find(|entity| entity.id == entity_id)
        .and_then(|entity| entity.parent_id);
    while let Some(parent_id) = current {
        depth += 1;
        if depth > entities.len() {
            break;
        }
        current = entities
            .iter()
            .find(|entity| entity.id == parent_id)
            .and_then(|entity| entity.parent_id);
    }
    depth
}

fn spawn_position(game: &Game) -> (f64, f64) {
    (
        game.camera.x / game.grid.tile_size as f64 + 5.0,
        game.camera.y / game.grid.tile_size as f64 + 4.0,
    )
}

fn entity_color(entity: &GameObject) -> Color {
    if entity.locked {
        return Color::from_rgba(115, 118, 128, 255);
    }
    if entity.tag == "Player" {
        return Color::from_rgba(85, 178, 255, 255);
    }
    if entity.tag == "Enemy" {
        return Color::from_rgba(255, 105, 105, 255);
    }
    if entity.get_component("ResourceNode").is_some() {
        return Color::from_rgba(245, 190, 78, 255);
    }
    if entity.get_component("UIElement").is_some() {
        return Color::from_rgba(160, 210, 255, 255);
    }
    Color::from_rgba(95, 215, 170, 255)
}

fn entity_type_accent(tag: &str) -> Color {
    match tag {
        "Player" => Color::from_rgba(85, 178, 255, 255),
        "Enemy" => Color::from_rgba(255, 105, 105, 255),
        "Resource" => Color::from_rgba(245, 190, 78, 255),
        "UI" => Color::from_rgba(160, 210, 255, 255),
        _ => Color::from_rgba(95, 215, 170, 255),
    }
}

fn tile_color(value: i32, layer_index: usize) -> Color {
    let palette = [
        Color::from_rgba(72, 126, 88, 210),
        Color::from_rgba(70, 116, 188, 210),
        Color::from_rgba(188, 150, 70, 210),
        Color::from_rgba(166, 82, 96, 210),
        Color::from_rgba(142, 96, 205, 210),
        Color::from_rgba(70, 170, 160, 210),
        Color::from_rgba(210, 210, 118, 210),
        Color::from_rgba(205, 118, 198, 210),
        Color::from_rgba(120, 154, 162, 210),
    ];
    let mut color = palette[value.unsigned_abs() as usize % palette.len()];
    color.r = (color.r + layer_index as f32 * 0.03).min(1.0);
    color.g = (color.g + layer_index as f32 * 0.02).min(1.0);
    color
}

fn world_to_screen(viewport: Viewport, wx: f32, wy: f32) -> (f32, f32) {
    (
        viewport.rect.x + wx * viewport.tile * viewport.zoom - viewport.camera_x * viewport.zoom,
        viewport.rect.y + wy * viewport.tile * viewport.zoom - viewport.camera_y * viewport.zoom,
    )
}

fn screen_to_world(viewport: Viewport, sx: f32, sy: f32) -> (f32, f32) {
    (
        (sx - viewport.rect.x + viewport.camera_x * viewport.zoom)
            / (viewport.tile * viewport.zoom),
        (sy - viewport.rect.y + viewport.camera_y * viewport.zoom)
            / (viewport.tile * viewport.zoom),
    )
}

fn draw_rect(rect: RectSpec, color: Color) {
    draw_rectangle(rect.x, rect.y, rect.w, rect.h, color);
}

fn ui_bg() -> Color {
    Color::from_rgba(9, 11, 15, 255)
}

fn ui_panel() -> Color {
    Color::from_rgba(17, 21, 29, 248)
}

fn ui_panel_alt() -> Color {
    Color::from_rgba(24, 29, 39, 248)
}

fn ui_line() -> Color {
    Color::from_rgba(67, 80, 101, 255)
}

fn ui_line_soft() -> Color {
    Color::from_rgba(42, 52, 68, 255)
}

fn ui_text() -> Color {
    Color::from_rgba(237, 243, 252, 255)
}

fn ui_text_muted() -> Color {
    Color::from_rgba(151, 166, 190, 255)
}

fn ui_accent() -> Color {
    Color::from_rgba(88, 166, 255, 255)
}

fn ui_accent_2() -> Color {
    Color::from_rgba(86, 214, 180, 255)
}

fn ui_warning() -> Color {
    Color::from_rgba(255, 202, 92, 255)
}

fn blend_color(a: Color, b: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    Color {
        r: a.r + (b.r - a.r) * t,
        g: a.g + (b.g - a.g) * t,
        b: a.b + (b.b - a.b) * t,
        a: a.a + (b.a - a.a) * t,
    }
}

fn draw_gradient_rect(rect: RectSpec, top: Color, bottom: Color) {
    let steps = rect.h.clamp(1.0, 96.0) as usize;
    if steps == 0 {
        return;
    }
    let step_h = rect.h / steps as f32;
    for index in 0..steps {
        let t = if steps <= 1 {
            0.0
        } else {
            index as f32 / (steps - 1) as f32
        };
        draw_rectangle(
            rect.x,
            rect.y + index as f32 * step_h,
            rect.w,
            step_h + 1.0,
            blend_color(top, bottom, t),
        );
    }
}

fn draw_editor_backdrop(sw: f32, sh: f32) {
    draw_gradient_rect(
        RectSpec {
            x: 0.0,
            y: 0.0,
            w: sw,
            h: sh,
        },
        Color::from_rgba(13, 17, 24, 255),
        Color::from_rgba(6, 8, 12, 255),
    );
    draw_rectangle(0.0, 0.0, sw, 2.0, Color::from_rgba(107, 213, 255, 80));
    draw_rectangle(0.0, 2.0, sw, 1.0, Color::from_rgba(86, 214, 180, 42));
}

fn draw_surface(rect: RectSpec, active: bool) {
    draw_rectangle(
        rect.x,
        rect.y + 2.0,
        rect.w,
        rect.h,
        Color::from_rgba(0, 0, 0, 48),
    );
    draw_gradient_rect(
        rect,
        if active {
            Color::from_rgba(28, 36, 50, 252)
        } else {
            ui_panel_alt()
        },
        ui_panel(),
    );
    draw_rectangle(
        rect.x,
        rect.y,
        rect.w,
        1.0,
        Color::from_rgba(255, 255, 255, 30),
    );
    draw_rectangle(
        rect.x + 1.0,
        rect.y + rect.h - 2.0,
        rect.w - 2.0,
        1.0,
        Color::from_rgba(0, 0, 0, 60),
    );
    draw_rectangle_lines(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        1.0,
        if active {
            Color::from_rgba(84, 116, 150, 255)
        } else {
            ui_line_soft()
        },
    );
}

fn draw_panel_header(rect: RectSpec, title: &str, subtitle: &str) {
    draw_gradient_rect(
        RectSpec {
            x: rect.x,
            y: rect.y,
            w: rect.w,
            h: 42.0_f32.min(rect.h),
        },
        Color::from_rgba(38, 49, 67, 245),
        Color::from_rgba(22, 28, 39, 235),
    );
    draw_rectangle(rect.x, rect.y, 4.0, rect.h.min(42.0), ui_accent());
    draw_rectangle(
        rect.x + 4.0,
        rect.y + rect.h.min(42.0) - 1.0,
        rect.w - 4.0,
        1.0,
        Color::from_rgba(86, 214, 180, 75),
    );
    draw_text_fit(
        title,
        RectSpec {
            x: rect.x + 14.0,
            y: rect.y + 6.0,
            w: (rect.w * 0.42).max(92.0),
            h: 28.0,
        },
        19,
        ui_text(),
        TextAlign::Left,
    );
    if !subtitle.is_empty() {
        draw_text_fit(
            subtitle,
            RectSpec {
                x: rect.x + (rect.w * 0.42).max(126.0),
                y: rect.y + 8.0,
                w: (rect.w * 0.56 - 18.0).max(44.0),
                h: 24.0,
            },
            12,
            ui_text_muted(),
            TextAlign::Right,
        );
    }
}

fn draw_status_pill(rect: RectSpec, label: &str, active: bool) {
    draw_gradient_rect(
        rect,
        if active {
            Color::from_rgba(42, 84, 122, 255)
        } else {
            Color::from_rgba(31, 38, 51, 255)
        },
        if active {
            Color::from_rgba(25, 54, 84, 255)
        } else {
            Color::from_rgba(22, 27, 37, 255)
        },
    );
    draw_rectangle_lines(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        1.0,
        if active { ui_accent() } else { ui_line_soft() },
    );
    draw_text(
        &ellipsize(label, (rect.w / 7.0).max(3.0) as usize),
        rect.x + 8.0,
        rect.y + rect.h * 0.68,
        12.0,
        if active { WHITE } else { ui_text_muted() },
    );
}

fn draw_macos_chrome(rect: RectSpec) {
    let y = rect.y + 16.0;
    for (index, color) in [
        Color::from_rgba(255, 95, 87, 255),
        Color::from_rgba(255, 189, 46, 255),
        Color::from_rgba(40, 200, 64, 255),
    ]
    .iter()
    .enumerate()
    {
        let x = rect.x + 18.0 + index as f32 * 20.0;
        draw_circle(x, y, 6.0, *color);
        draw_circle_lines(x, y, 6.0, 1.0, Color::from_rgba(0, 0, 0, 80));
    }
}

fn contains(rect: RectSpec, x: f32, y: f32) -> bool {
    UiButton::hit_test(
        (
            f64::from(rect.x),
            f64::from(rect.y),
            f64::from(rect.w),
            f64::from(rect.h),
        ),
        f64::from(x),
        f64::from(y),
        true,
        true,
    )
}

fn contains_mouse(rect: RectSpec) -> bool {
    let (mx, my) = mouse_position();
    contains(rect, mx, my)
}

fn draw_text_fit(text: &str, rect: RectSpec, font_size: u16, color: Color, align: TextAlign) {
    if rect.w <= 1.0 || rect.h <= 1.0 || text.is_empty() {
        return;
    }
    let mut size = font_size.max(8);
    while size > 8 && measure_text(text, None, size, 1.0).width > rect.w {
        size -= 1;
    }
    let avg_char_w = (size as f32 * 0.55).max(1.0);
    let max_chars = (rect.w / avg_char_w).floor().max(1.0) as usize;
    let shown = ellipsize(text, max_chars);
    let measure = measure_text(&shown, None, size, 1.0);
    let x = match align {
        TextAlign::Left => rect.x,
        TextAlign::Center => rect.x + ((rect.w - measure.width) * 0.5).max(0.0),
        TextAlign::Right => rect.x + (rect.w - measure.width).max(0.0),
    };
    let y = rect.y + rect.h * 0.5 + measure.height * 0.34;
    draw_text(&shown, x, y, size as f32, color);
}

fn command_modifier_down() -> bool {
    is_key_down(KeyCode::LeftControl)
        || is_key_down(KeyCode::RightControl)
        || is_key_down(KeyCode::LeftSuper)
        || is_key_down(KeyCode::RightSuper)
}

fn button(x: f32, y: f32, w: f32, h: f32, label: &str, active: bool) -> bool {
    let rect = RectSpec { x, y, w, h };
    let (mouse_x, mouse_y) = mouse_position();
    let hovered = UiButton::hit_test(
        (f64::from(x), f64::from(y), f64::from(w), f64::from(h)),
        f64::from(mouse_x),
        f64::from(mouse_y),
        true,
        true,
    );
    let pressed = hovered && is_mouse_button_down(MouseButton::Left);
    let visual_state = UiButton::resolve_visual_state(true, true, hovered, pressed);
    let fill = if active {
        Color::from_rgba(38, 91, 134, 255)
    } else if visual_state == crate::engine::ui::ButtonVisualState::Pressed {
        Color::from_rgba(30, 79, 118, 255)
    } else if hovered {
        Color::from_rgba(39, 48, 63, 255)
    } else {
        Color::from_rgba(25, 31, 42, 255)
    };
    draw_gradient_rect(
        rect,
        if hovered || active {
            blend_color(fill, Color::from_rgba(255, 255, 255, 255), 0.08)
        } else {
            fill
        },
        blend_color(fill, Color::from_rgba(0, 0, 0, 255), 0.16),
    );
    draw_rectangle(
        x + 1.0,
        y + 1.0,
        w - 2.0,
        1.0,
        Color::from_rgba(255, 255, 255, 22),
    );
    draw_rectangle_lines(
        x,
        y,
        w,
        h,
        1.0,
        if hovered || active {
            Color::from_rgba(111, 190, 244, 255)
        } else {
            Color::from_rgba(54, 65, 84, 255)
        },
    );
    if active {
        draw_rectangle(x + 3.0, y + h - 3.0, w - 6.0, 2.0, ui_accent_2());
    }
    draw_text_fit(
        label,
        RectSpec {
            x: x + 5.0,
            y: y + 1.0,
            w: (w - 10.0).max(1.0),
            h: (h - 2.0).max(1.0),
        },
        14,
        ui_text(),
        TextAlign::Center,
    );
    hovered && is_mouse_button_pressed(MouseButton::Left)
}

#[allow(
    clippy::too_many_arguments,
    reason = "compact immediate-mode UI primitive"
)]
fn icon_button(
    font: Option<&Font>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    icon: EditorIcon,
    label: &str,
    active: bool,
) -> bool {
    let clicked = button(x, y, w, h, "", active);
    if let Some(font) = font {
        draw_text_ex(
            icon.glyph(),
            x + 6.0,
            y + h - 6.0,
            TextParams {
                font: Some(font),
                font_size: 15,
                color: if active { WHITE } else { ui_text() },
                ..Default::default()
            },
        );
    }
    draw_text_fit(
        label,
        RectSpec {
            x: x + if font.is_some() { 22.0 } else { 5.0 },
            y: y + 1.0,
            w: (w - if font.is_some() { 27.0 } else { 10.0 }).max(1.0),
            h: (h - 2.0).max(1.0),
        },
        14,
        ui_text(),
        TextAlign::Center,
    );
    clicked
}

fn ellipsize(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    if max_chars <= 3 {
        return ".".repeat(max_chars);
    }
    let mut value = text.chars().take(max_chars - 3).collect::<String>();
    value.push_str("...");
    value
}

pub fn parse_exported_build_path_from_env() -> Result<PathBuf, String> {
    let mut args = std::env::args().skip(1);
    let mut resolved: Option<PathBuf> = None;
    while let Some(arg) = args.next() {
        if arg == "--help" || arg == "-h" {
            println!(
                "miniforge_runtime: ejecuta un build exportado sin UI de editor.\n\
                 Uso: miniforge_runtime --build <carpeta> | miniforge_runtime <carpeta>\n\
                 La carpeta debe contener runtime_manifest.json (salida de Export Runtime)."
            );
            std::process::exit(0);
        }
        if arg == "--build" {
            if let Some(p) = args.next() {
                resolved = Some(PathBuf::from(p));
            }
        } else if !arg.starts_with('-') && resolved.is_none() {
            resolved = Some(PathBuf::from(arg));
        }
    }
    resolved.ok_or_else(|| {
        "Indica la carpeta del build exportado: miniforge_runtime --build <ruta>".into()
    })
}

pub async fn run_exported_runtime_player() {
    let build_root = match parse_exported_build_path_from_env() {
        Ok(p) => p,
        Err(msg) => {
            eprintln!("{msg}");
            return;
        }
    };
    if !build_root.is_dir() {
        eprintln!("La ruta no es una carpeta: {}", build_root.display());
        return;
    }
    match RuntimeManifestLoader::load(&build_root) {
        Ok(m) => {
            if !m.validated_missing.is_empty() {
                eprintln!(
                    "MiniForge Runtime: {} referencias de assets no encontradas en disco:",
                    m.validated_missing.len()
                );
                for rel in m.validated_missing.iter().take(32) {
                    eprintln!("  missing: {rel}");
                }
            }
        }
        Err(e) => {
            eprintln!(
                "Advertencia: no se pudo validar runtime_manifest.json ({e}). Se intenta cargar el proyecto igualmente."
            );
        }
    }
    if let Err(error) = AssetTools::ensure_project_folders(&build_root) {
        eprintln!("Estructura de proyecto incompleta: {error}");
        return;
    }
    CrashReporter::install(CrashReporterConfig::for_project(
        &build_root,
        "MiniForge Runtime",
    ));
    let safe_mode = if safe_mode_requested_from_environment() {
        SafeModeSettings::for_recovery("solicitado para runtime exportado")
    } else {
        SafeModeSettings::default()
    };
    let mut game = match Game::from_project_with_safe_mode(&build_root, true, safe_mode) {
        Ok(game) => game,
        Err(error) => {
            eprintln!("No se pudo cargar el proyecto exportado: {error}");
            return;
        }
    };
    game.selected_units.clear();
    game.console.log(
        format!("Runtime player: {} (Esc para salir)", build_root.display()),
        "RUNTIME",
    );
    let mut nav = EditorState {
        zoom_target: game.camera.zoom,
        ..EditorState::default()
    };
    loop {
        if is_key_pressed(KeyCode::Escape) {
            break;
        }
        let dt = get_frame_time() as f64;
        let sw = screen_width();
        let sh = screen_height();
        handle_camera_input(&mut game, dt, &mut nav);
        handle_character_input(&mut game);
        game.run_headless_once(dt);
        clear_background(Color::from_rgba(18, 20, 26, 255));
        let viewport = Viewport {
            rect: RectSpec {
                x: 0.0,
                y: 0.0,
                w: sw,
                h: sh,
            },
            tile: game.grid.tile_size as f32,
            zoom: game.camera.zoom as f32,
            camera_x: game.camera.x as f32,
            camera_y: game.camera.y as f32,
        };
        draw_world_player(&game, viewport);
        draw_text(
            "Runtime player — WASD mover jugador, Space salto, Shift correr, X dash, Q/E zoom, Esc salir",
            14.0,
            18.0,
            16.0,
            Color::from_rgba(200, 210, 230, 255),
        );
        next_frame().await;
    }
}

fn safe_mode_requested_from_environment() -> bool {
    std::env::args().any(|argument| argument == "--safe-mode")
        || std::env::var("MINIFORGE_SAFE_MODE")
            .ok()
            .is_some_and(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
}
