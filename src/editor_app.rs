use std::env;
use std::path::{Path, PathBuf};

use macroquad::prelude::*;

use crate::core::game::Game;
use crate::engine::asset_database::AssetRecord;
use crate::engine::asset_tools::AssetTools;
use crate::engine::component::{advanced_component_category, advanced_component_types};
use crate::engine::content_drag::{DragPayload, DropOutcome};
use crate::engine::editor_command::{EditorCommandKind, EditorSnapshot};
use crate::engine::editor_workspace::WorkspaceMode;
use crate::engine::engine_programming::{VisualGraphNodeView, VisualGraphView};
use crate::engine::inspector_editor::{InspectorEditor, InspectorField};
use crate::engine::runtime_exporter::ExportProfile;
use crate::engine::runtime_manifest_loader::RuntimeManifestLoader;
use crate::engine::scene_view_tools::SceneViewTools;
use crate::engine::tile_brush::TileBrushMode;
use crate::engine::ui_canvas::{UiCanvasElement, layout_element_pixels, ui_canvases_from_value};
use crate::entities::game_object::GameObject;
use crate::systems::command_system::CommandSystem;
use crate::systems::rts_system::RTSSystem;
use serde_json::{Value, json};

#[derive(Debug, Default, Clone)]
struct Args {
    project: Option<PathBuf>,
    runtime: bool,
    no_launcher: bool,
    headless_once: bool,
}

#[derive(Debug, Clone, Copy)]
struct RectSpec {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
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
enum EditorTool {
    Select,
    Move,
    Rotate,
    Scale,
    Paint,
}

impl EditorTool {
    fn label(self) -> &'static str {
        match self {
            Self::Select => "Select",
            Self::Move => "Move",
            Self::Rotate => "Rotate",
            Self::Scale => "Scale",
            Self::Paint => "Paint",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BottomTab {
    Console,
    Assets,
    Programming,
    Prefabs,
    Profiler,
}

impl BottomTab {
    fn label(self) -> &'static str {
        match self {
            Self::Console => "Console",
            Self::Assets => "Browser",
            Self::Programming => "Graph",
            Self::Prefabs => "Prefabs",
            Self::Profiler => "Profiler",
        }
    }

    fn next(self) -> Self {
        match self {
            Self::Console => Self::Assets,
            Self::Assets => Self::Programming,
            Self::Programming => Self::Prefabs,
            Self::Prefabs => Self::Profiler,
            Self::Profiler => Self::Console,
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
}

#[derive(Debug, Clone)]
struct TextEditState {
    entity_id: u64,
    target: String,
    key: String,
    buffer: String,
}

#[derive(Debug, Clone)]
struct EditorState {
    paused: bool,
    show_console: bool,
    show_grid: bool,
    show_hierarchy: bool,
    show_inspector: bool,
    command_palette: bool,
    top_menu: Option<TopMenu>,
    tool: EditorTool,
    bottom_tab: BottomTab,
    tile_brush: i32,
    tile_brush_mode: TileBrushMode,
    snap_to_grid: bool,
    drag_entity: Option<u64>,
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
    graph_drag_node: Option<String>,
    graph_drag_offset: (f32, f32),
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            paused: false,
            show_console: true,
            show_grid: true,
            show_hierarchy: true,
            show_inspector: true,
            command_palette: false,
            top_menu: None,
            tool: EditorTool::Select,
            bottom_tab: BottomTab::Console,
            tile_brush: 1,
            tile_brush_mode: TileBrushMode::Pencil,
            snap_to_grid: true,
            drag_entity: None,
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
            graph_drag_node: None,
            graph_drag_offset: (0.0, 0.0),
        }
    }
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
            "--headless-once" => parsed.headless_once = true,
            "--help" | "-h" => {
                println!("MiniForge editor/runtime launcher");
                println!("  --project <path>  Project path to open");
                println!("  --runtime         Start in play/runtime mode");
                println!("  --no-launcher     Open project directly");
                println!("  --headless-once   Run one frame and exit, useful for CI");
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

pub async fn run_editor_async() {
    let args = parse_args();
    let project_path = args
        .project
        .clone()
        .unwrap_or_else(|| PathBuf::from("projects").join("DefaultProject"));

    if let Err(error) = AssetTools::ensure_project_folders(&project_path) {
        eprintln!("No se pudo preparar el proyecto: {error}");
        return;
    }

    let mut game = match Game::from_project(&project_path, args.runtime) {
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
        ..Default::default()
    };

    loop {
        let dt = get_frame_time() as f64;
        let sw = screen_width();
        let sh = screen_height();

        handle_text_edit_input(&mut game, &mut state);
        if handle_shortcuts(&mut game, &mut state) {
            break;
        }

        handle_camera_input(&mut game, dt, &state);

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

        handle_scene_mouse(&mut game, &mut state, viewport);

        if !state.paused {
            game.run_headless_once(dt);
        } else {
            game.diagnostics.update(dt);
        }

        clear_background(Color::from_rgba(18, 20, 26, 255));
        draw_scene(&game, &state, viewport);
        if state.show_hierarchy {
            draw_hierarchy(&mut game, layout.left);
        }
        if state.show_inspector {
            draw_inspector(&mut game, &mut state, layout.right);
        }
        draw_top_bar(&mut game, &mut state, layout.top);
        draw_status_bar(&game, &state, layout.status);
        if state.show_console {
            draw_bottom_panel(&mut game, &mut state, layout.console);
        }
        if state.command_palette {
            draw_command_palette(&mut game, &mut state, sw, sh);
        }

        next_frame().await;
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
    let top_h = 92.0;
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
    if state.content_search_active {
        handle_content_search_input(state);
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
        state.code_editor_active = false;
        return;
    }
    if command_modifier_down() && is_key_pressed(KeyCode::S) {
        if let Err(error) = game.save_open_file() {
            game.console
                .log(format!("Error guardando archivo abierto: {error}"), "ERROR");
        }
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
            state.code_cursor_column -= 1;
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
            state.code_cursor_column += 1;
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

    if state.active_text_field.is_some() || state.code_editor_active || state.content_search_active
    {
        return false;
    }

    if is_key_pressed(KeyCode::Escape) {
        if state.command_palette {
            state.command_palette = false;
            return false;
        }
        return true;
    }
    if command && is_key_pressed(KeyCode::P) {
        state.command_palette = !state.command_palette;
        return false;
    }
    if state.command_palette {
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
        toggle_play_mode(game);
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
    if is_key_pressed(KeyCode::Tab) {
        state.show_console = true;
        state.bottom_tab = state.bottom_tab.next();
    }
    if is_key_pressed(KeyCode::L) && state.tool == EditorTool::Paint {
        game.cycle_tilemap_layer();
    }
    if is_key_pressed(KeyCode::B) && state.tool == EditorTool::Paint && !command {
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
        if is_key_pressed(KeyCode::G)
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

fn handle_camera_input(game: &mut Game, dt: f64, state: &EditorState) {
    if state.command_palette {
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
        game.camera.set_zoom(game.camera.zoom - 0.8 * dt);
    }
    if is_key_down(KeyCode::E) {
        game.camera.set_zoom(game.camera.zoom + 0.8 * dt);
    }
}

fn handle_scene_mouse(game: &mut Game, state: &mut EditorState, viewport: Viewport) {
    let (mx, my) = mouse_position();
    if let Some(payload) = state.drag_payload.clone() {
        if is_mouse_button_released(MouseButton::Left) {
            if contains(viewport.rect, mx, my) {
                let world = screen_to_world(viewport, mx, my);
                match game.drop_asset_to_scene(&payload, world.0 as f64, world.1 as f64) {
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

    if !contains(viewport.rect, mx, my) || state.command_palette {
        if is_mouse_button_released(MouseButton::Left) {
            finish_gizmo_drag(game, state);
        }
        return;
    }

    let world = screen_to_world(viewport, mx, my);
    match state.tool {
        EditorTool::Paint => {
            if is_mouse_button_pressed(MouseButton::Left) {
                state.paint_start = world_to_cell(game, world);
                state.last_painted_cell = None;
            }
            if is_mouse_button_down(MouseButton::Left) {
                paint_at(game, state, world);
            }
            if is_mouse_button_released(MouseButton::Left) {
                if state.tile_brush_mode == TileBrushMode::Rectangle
                    && let (Some(start), Some(end)) =
                        (state.paint_start, world_to_cell(game, world))
                    && game.paint_tile_brush(TileBrushMode::Rectangle, start, end, state.tile_brush)
                {
                    game.console.log("Rect brush aplicado", "TILEMAP");
                }
                state.paint_start = None;
                state.last_painted_cell = None;
            }
        }
        EditorTool::Move | EditorTool::Rotate | EditorTool::Scale => {
            if is_mouse_button_pressed(MouseButton::Left) {
                state.drag_entity = find_entity_at(game, world);
                if let Some(id) = state.drag_entity {
                    game.select_entity(id);
                    state.drag_entity_before = Some(game.capture_editor_snapshot());
                    game.console.log(format!("Moviendo entity #{id}"), "EDITOR");
                }
            }
            if is_mouse_button_down(MouseButton::Left) {
                apply_gizmo_drag(game, state, world);
            } else {
                finish_gizmo_drag(game, state);
            }
        }
        EditorTool::Select => {
            if is_mouse_button_pressed(MouseButton::Left) {
                select_at(game, world);
            }
            if is_mouse_button_pressed(MouseButton::Right) {
                command_selected_move(game, world);
            }
        }
    }
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
    if state.tile_brush_mode == TileBrushMode::Rectangle {
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

fn apply_gizmo_drag(game: &mut Game, state: &EditorState, world: (f32, f32)) {
    let Some(id) = state.drag_entity else {
        return;
    };
    let snap = state.snap_to_grid;
    let tile_size = game.grid.tile_size as f64;
    let camera_zoom = game.camera.zoom;
    if let Some(entity) = game.get_entity_by_id_mut(id) {
        match state.tool {
            EditorTool::Move => {
                let tools = SceneViewTools {
                    grid_snapping: snap,
                    snap_size: if snap { 0.25 } else { 0.0001 },
                    tile_size,
                    camera_zoom,
                };
                tools.set_world_position(entity, world.0 as f64, world.1 as f64);
            }
            EditorTool::Rotate => {
                let dx = world.0 as f64 - entity.x;
                let dy = world.1 as f64 - entity.y;
                entity.rotation = dy.atan2(dx).to_degrees();
                entity.sync_to_components();
            }
            EditorTool::Scale => {
                let dx = world.0 as f64 - entity.x;
                let dy = world.1 as f64 - entity.y;
                let scale = (dx.hypot(dy) * 0.5).clamp(0.1, 12.0);
                entity.scale_x = scale;
                entity.scale_y = scale;
                entity.sync_to_components();
            }
            EditorTool::Select | EditorTool::Paint => {}
        }
        game.mark_scene_dirty(state.tool.label());
    }
}

fn finish_gizmo_drag(game: &mut Game, state: &mut EditorState) {
    let Some(id) = state.drag_entity.take() else {
        state.drag_entity_before = None;
        return;
    };
    if let Some(before) = state.drag_entity_before.take() {
        game.sync_world();
        game.push_editor_command(
            format!("{} Entity", state.tool.label()),
            EditorCommandKind::MoveEntity { entity_id: id },
            before,
        );
    }
}

fn log_drop_outcome(game: &mut Game, asset_name: &str, outcome: DropOutcome) {
    match outcome {
        DropOutcome::SpawnedEntity(id) => game
            .console
            .log(format!("{asset_name} instanciado como #{id}"), "ASSETS"),
        DropOutcome::AppliedToEntity(id) => game
            .console
            .log(format!("{asset_name} aplicado a #{id}"), "ASSETS"),
        DropOutcome::Unsupported(reason) => game
            .console
            .log(format!("{asset_name}: {reason}"), "WARNING"),
    }
}

fn select_at(game: &mut Game, world: (f32, f32)) {
    game.clear_selection();
    if let Some(id) = find_entity_at(game, world) {
        game.select_entity(id);
        game.console
            .log(format!("Seleccionado entity #{id}"), "EDITOR");
    }
}

fn find_entity_at(game: &Game, world: (f32, f32)) -> Option<u64> {
    game.units
        .iter()
        .rev()
        .find(|entity| {
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

fn toggle_play_mode(game: &mut Game) {
    game.toggle_play_mode();
}

fn set_tool(game: &mut Game, state: &mut EditorState, tool: EditorTool) {
    state.tool = tool;
    game.active_tool = tool.label().to_string();
    game.console
        .log(format!("Herramienta activa: {}", tool.label()), "EDITOR");
}

fn draw_top_bar(game: &mut Game, state: &mut EditorState, rect: RectSpec) {
    draw_rect(rect, Color::from_rgba(24, 27, 34, 255));
    draw_macos_chrome(rect);
    draw_rectangle(
        rect.x,
        rect.y + rect.h - 1.0,
        rect.w,
        1.0,
        Color::from_rgba(70, 78, 96, 255),
    );
    draw_text("MiniForge", rect.x + 82.0, rect.y + 25.0, 24.0, WHITE);
    draw_top_menus(game, state, rect);
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
        Color::from_rgba(200, 206, 220, 255),
    );

    let mut x = rect.x + 16.0;
    let y = rect.y + 56.0;
    let play_label = if game.mode == "PLAY" { "Stop" } else { "Play" };
    if button(x, y, 58.0, 24.0, play_label, game.mode == "PLAY") {
        toggle_play_mode(game);
    }
    x += 64.0;
    if button(x, y, 58.0, 24.0, "Save", false) {
        save_project(game);
    }
    x += 66.0;
    if button(x, y, 54.0, 24.0, "+Obj", false) {
        let (sx, sy) = spawn_position(game);
        let id = game.spawn_game_object("GameObject", sx, sy);
        game.console
            .log(format!("GameObject creado #{id}"), "SCENE");
    }
    x += 60.0;
    if button(x, y, 56.0, 24.0, "+Unit", false) {
        let (sx, sy) = spawn_position(game);
        let id = game.spawn_unit("PlayerUnit", sx, sy);
        game.console
            .log(format!("Unidad avanzada creada #{id}"), "SCENE");
    }
    x += 62.0;
    if button(x, y, 66.0, 24.0, "+Enemy", false) {
        let (sx, sy) = spawn_position(game);
        let id = game.spawn_enemy(sx + 2.0, sy);
        game.console.log(format!("Enemy AI creado #{id}"), "SCENE");
    }
    x += 72.0;
    if button(x, y, 58.0, 24.0, "+Gold", false) {
        let (sx, sy) = spawn_position(game);
        let id = game.spawn_resource(sx, sy + 2.0);
        game.console
            .log(format!("ResourceNode creado #{id}"), "SCENE");
    }
    x += 66.0;
    if button(x, y, 58.0, 24.0, "+Base", false) {
        let (sx, sy) = spawn_position(game);
        let id = game.spawn_rts_building("CommandCenter", sx, sy, 1);
        game.console.log(format!("Base RTS creada #{id}"), "RTS");
    }
    x += 66.0;
    if button(x, y, 62.0, 24.0, "Top2D", false) {
        game.create_topdown_starter();
    }
    x += 70.0;
    if button(x, y, 62.0, 24.0, "Plat2D", false) {
        game.create_platformer_starter();
    }
    x += 70.0;
    if button(x, y, 78.0, 24.0, "RTS Demo", false) {
        game.create_rts_skirmish();
    }
    x += 86.0;
    if button(x, y, 72.0, 24.0, "Validate", false) {
        game.validate_project();
    }
    x += 78.0;
    if button(x, y, 66.0, 24.0, "Assets", false) {
        refresh_assets(game);
        state.show_console = true;
        state.bottom_tab = BottomTab::Assets;
    }
    x += 72.0;
    if button(x, y, 68.0, 24.0, "+Sprite", false) {
        let (sx, sy) = spawn_position(game);
        game.spawn_sprite_entity("Sprite", "sprite", sx, sy);
    }
    x += 76.0;
    if button(x, y, 72.0, 24.0, "Manifest", false) {
        build_manifest(game);
    }
    x += 78.0;
    if button(x, y, 66.0, 24.0, "+Graph", false)
        && let Ok(path) = game.create_program_asset("LogAndMove")
    {
        set_open_file_editor_state(state, &path);
        state.show_console = true;
        state.bottom_tab = BottomTab::Programming;
    }
    x += 72.0;
    if button(x, y, 76.0, 24.0, "Prefab", false) {
        if let Err(error) = game.save_selected_as_prefab() {
            game.console
                .log(format!("Error guardando prefab: {error}"), "ERROR");
        }
        state.show_console = true;
        state.bottom_tab = BottomTab::Prefabs;
    }
    x += 86.0;

    for tool in [
        EditorTool::Select,
        EditorTool::Move,
        EditorTool::Rotate,
        EditorTool::Scale,
        EditorTool::Paint,
    ] {
        let width = match tool {
            EditorTool::Select => 62.0,
            EditorTool::Move => 54.0,
            EditorTool::Rotate => 62.0,
            EditorTool::Scale => 58.0,
            EditorTool::Paint => 58.0,
        };
        if button(x, y, width, 24.0, tool.label(), state.tool == tool) {
            set_tool(game, state, tool);
        }
        x += width + 6.0;
    }
    if button(x, y, 30.0, 24.0, "-", false) {
        state.tile_brush = (state.tile_brush - 1).max(0);
        game.tile_brush = state.tile_brush;
    }
    x += 34.0;
    if button(x, y, 30.0, 24.0, "+", false) {
        state.tile_brush = (state.tile_brush + 1).min(9);
        game.tile_brush = state.tile_brush;
    }
    x += 38.0;
    if button(x, y, 54.0, 24.0, "Layer", false) {
        game.cycle_tilemap_layer();
    }
    x += 60.0;
    if button(
        x,
        y,
        74.0,
        24.0,
        state.tile_brush_mode.label(),
        state.tool == EditorTool::Paint,
    ) {
        state.tile_brush_mode = state.tile_brush_mode.next();
    }
    x += 80.0;
    if button(x, y, 54.0, 24.0, "Snap", state.snap_to_grid) {
        state.snap_to_grid = !state.snap_to_grid;
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
        Color::from_rgba(150, 162, 184, 255),
    );
}

fn draw_top_menus(game: &mut Game, state: &mut EditorState, rect: RectSpec) {
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
            state.top_menu == Some(menu),
        ) {
            state.top_menu = if state.top_menu == Some(menu) {
                None
            } else {
                Some(menu)
            };
        }
        x += width + 5.0;
    }
    if let Some(menu) = state.top_menu {
        draw_menu_popover(game, state, menu, rect.x + 190.0, rect.y + 32.0);
    }
}

fn draw_menu_popover(game: &mut Game, state: &mut EditorState, menu: TopMenu, x: f32, y: f32) {
    let items: &[(&str, &str)] = match menu {
        TopMenu::File => &[
            ("Save Project", "save_project"),
            ("Save Scene", "save"),
            ("New Scene", "new_scene"),
            ("Recover Autosave", "recover_autosave"),
            ("Export Debug", "export_debug"),
            ("Export Release", "export_release"),
            ("Package Debug", "package_debug"),
            ("Package Release", "package_release"),
            ("Refresh Assets", "refresh"),
        ],
        TopMenu::Create => &[
            ("GameObject", "spawn_object"),
            ("Unit", "spawn_unit"),
            ("Sprite Entity", "spawn_sprite"),
            ("UI Canvas HUD", "ui_canvas_hud"),
            ("UI Canvas Label", "ui_canvas_label"),
            ("Sound Cue", "asset_sound"),
            ("Material", "asset_material"),
        ],
        TopMenu::View => &[
            ("Command Palette", "command_palette"),
            ("Toggle Browser", "toggle_browser"),
            ("Toggle Hierarchy", "toggle_hierarchy"),
            ("Toggle Inspector", "toggle_inspector"),
        ],
        TopMenu::Project => &[
            ("Validate", "validate"),
            ("Build Manifest", "manifest"),
            ("TopDown Starter", "starter_topdown"),
            ("Platformer Starter", "starter_platformer"),
        ],
        TopMenu::Rts => &[
            ("RTS Skirmish", "rts_skirmish"),
            ("Command Center", "spawn_rts_base"),
            ("Queue Worker", "queue_worker"),
            ("Place Barracks", "place_barracks"),
        ],
    };
    let width = 184.0;
    let height = items.len() as f32 * 24.0 + 12.0;
    draw_rect(
        RectSpec {
            x,
            y,
            w: width,
            h: height,
        },
        Color::from_rgba(28, 32, 42, 248),
    );
    draw_rectangle_lines(x, y, width, height, 1.0, Color::from_rgba(88, 98, 120, 255));
    let mut row_y = y + 26.0;
    for (label, command) in items {
        let row = RectSpec {
            x: x + 6.0,
            y: row_y - 18.0,
            w: width - 12.0,
            h: 22.0,
        };
        let hovered = contains_mouse(row);
        draw_rect(
            row,
            if hovered {
                Color::from_rgba(58, 67, 86, 255)
            } else {
                Color::from_rgba(35, 40, 52, 255)
            },
        );
        draw_text(
            label,
            row.x + 8.0,
            row_y,
            14.0,
            Color::from_rgba(232, 236, 245, 255),
        );
        if hovered && is_mouse_button_pressed(MouseButton::Left) {
            run_palette_command(game, state, command);
            state.top_menu = None;
        }
        row_y += 24.0;
    }
}

fn draw_hierarchy(game: &mut Game, rect: RectSpec) {
    if rect.w <= 0.0 {
        return;
    }
    draw_rect(rect, Color::from_rgba(22, 25, 33, 255));
    draw_text("Hierarchy", rect.x + 14.0, rect.y + 25.0, 21.0, WHITE);
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
        Color::from_rgba(150, 158, 174, 255),
    );
    draw_text(
        "F5 Play snapshot | F11 pausa sim | F2/F3 Hierarchy/Inspector",
        rect.x + 14.0,
        rect.y + 66.0,
        12.0,
        Color::from_rgba(120, 132, 158, 255),
    );
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

    let mut y = rect.y + 86.0;
    let max_rows = ((rect.h - 102.0) / 25.0).max(0.0) as usize;
    let rows: Vec<(u64, String, String, bool)> = game
        .units
        .iter()
        .take(max_rows)
        .map(|entity| {
            (
                entity.id,
                entity.name.clone(),
                entity.tag.clone(),
                game.selected_units.contains(&entity.id),
            )
        })
        .collect();

    for (id, name, tag, selected) in rows {
        let row = RectSpec {
            x: rect.x + 8.0,
            y: y - 17.0,
            w: rect.w - 16.0,
            h: 22.0,
        };
        let hovered = contains_mouse(row);
        let color = if selected {
            Color::from_rgba(65, 116, 184, 255)
        } else if hovered {
            Color::from_rgba(45, 52, 66, 255)
        } else {
            Color::from_rgba(34, 39, 50, 255)
        };
        draw_rect(row, color);
        draw_text(
            &ellipsize(&format!("{name}  #{id}"), 25),
            rect.x + 16.0,
            y,
            16.0,
            Color::from_rgba(232, 236, 245, 255),
        );
        draw_text(
            &ellipsize(&tag, 11),
            rect.x + rect.w - 80.0,
            y,
            13.0,
            Color::from_rgba(165, 176, 198, 255),
        );
        if hovered && is_mouse_button_pressed(MouseButton::Left) {
            game.select_entity(id);
            game.console
                .log(format!("Seleccionado desde Hierarchy #{id}"), "EDITOR");
        }
        y += 25.0;
    }

    draw_rectangle_lines(
        rect.x + rect.w - 1.0,
        rect.y,
        1.0,
        rect.h,
        1.0,
        Color::from_rgba(62, 68, 84, 255),
    );
}

fn draw_inspector_scene_ui_canvas(game: &mut Game, rect: RectSpec) {
    let mut y = rect.y + 58.0;
    draw_text(
        "Sin entidad seleccionada — UI Canvas",
        rect.x + 14.0,
        y,
        16.0,
        Color::from_rgba(200, 210, 230, 255),
    );
    y += 26.0;
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
                    draw_text(label, px + 8.0, py + eh * 0.62, 14.0, WHITE);
                }
                UiCanvasElement::Label {
                    text, font_size, ..
                } => {
                    draw_text(text, px, py + *font_size, *font_size, WHITE);
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
    draw_rect(rect, Color::from_rgba(25, 29, 38, 255));
    draw_text("Inspector", rect.x + 14.0, rect.y + 26.0, 22.0, WHITE);
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
        Color::from_rgba(220, 225, 238, 255),
    );
    y += 24.0;
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
    y += 34.0;

    draw_text("Transform", rect.x + 14.0, y, 18.0, WHITE);
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
        Color::from_rgba(205, 216, 236, 255),
    );
    if button(rect.x + 190.0, y, 80.0, 23.0, "Layer", false) {
        cycle_identity(game, id, "layer");
    }
    y += 34.0;

    draw_text("Prefab / Graph", rect.x + 14.0, y, 18.0, WHITE);
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
            Color::from_rgba(178, 205, 240, 255),
        );
        y += 22.0;
    }

    draw_text("Add / Remove Component", rect.x + 14.0, y, 18.0, WHITE);
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
    y += 38.0;

    draw_text("Components", rect.x + 14.0, y, 18.0, WHITE);
    y += 24.0;
    for component in entity.components.iter().take(10) {
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
        draw_rect(header, Color::from_rgba(35, 41, 54, 255));
        draw_text(
            &ellipsize(&format!("{} [{}]", component.component_type, category), 28),
            rect.x + 18.0,
            y,
            15.0,
            Color::from_rgba(170, 210, 255, 255),
        );
        if button(rect.x + rect.w - 52.0, y - 16.0, 38.0, 20.0, "-", false)
            && let Err(error) = game.remove_component_from_entity(id, &component.component_type)
        {
            game.console.log(error, "WARNING");
        }
        y += 27.0;
        for (key, value) in component.data.iter().take(4) {
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

fn draw_scene(game: &Game, state: &EditorState, viewport: Viewport) {
    draw_rect(viewport.rect, Color::from_rgba(20, 23, 30, 255));
    draw_tiles(game, viewport);
    if state.show_grid {
        draw_grid(game, viewport);
    }

    for entity in &game.units {
        if !entity.enabled || !entity.visible {
            continue;
        }
        draw_entity(entity, viewport, game.selected_units.contains(&entity.id));
    }

    draw_scene_gizmos(game, state, viewport);
    draw_paint_cursor(game, state, viewport);
    draw_drag_preview(state);
    draw_ui_elements(game);
    draw_scene_hud(game, state, viewport);
    draw_rectangle_lines(
        viewport.rect.x,
        viewport.rect.y,
        viewport.rect.w,
        viewport.rect.h,
        1.0,
        Color::from_rgba(73, 78, 94, 255),
    );
}

fn draw_world_player(game: &Game, viewport: Viewport) {
    draw_rect(viewport.rect, Color::from_rgba(20, 23, 30, 255));
    draw_tiles(game, viewport);
    for entity in &game.units {
        if !entity.enabled || !entity.visible {
            continue;
        }
        draw_entity(entity, viewport, false);
    }
    draw_ui_elements(game);
}

fn draw_scene_gizmos(game: &Game, state: &EditorState, viewport: Viewport) {
    for id in &game.selected_units {
        let Some(entity) = game.get_entity_by_id(*id) else {
            continue;
        };
        let bounds = SceneViewTools::bounding_box(entity);
        let (min_x, min_y) = world_to_screen(viewport, bounds.min_x as f32, bounds.min_y as f32);
        let (max_x, max_y) = world_to_screen(viewport, bounds.max_x as f32, bounds.max_y as f32);
        let w = max_x - min_x;
        let h = max_y - min_y;
        draw_rectangle_lines(min_x, min_y, w, h, 2.0, Color::from_rgba(94, 196, 255, 255));
        let (cx, cy) = world_to_screen(viewport, entity.x as f32, entity.y as f32);
        if matches!(
            state.tool,
            EditorTool::Move | EditorTool::Rotate | EditorTool::Scale
        ) {
            draw_line(
                cx,
                cy,
                cx + 46.0,
                cy,
                3.0,
                Color::from_rgba(255, 105, 105, 255),
            );
            draw_line(
                cx,
                cy,
                cx,
                cy - 46.0,
                3.0,
                Color::from_rgba(120, 225, 145, 255),
            );
            draw_circle(cx, cy, 5.0, Color::from_rgba(245, 245, 255, 255));
        }
        if state.tool == EditorTool::Rotate {
            draw_circle_lines(
                cx,
                cy,
                w.max(h).max(28.0) * 0.65,
                2.0,
                Color::from_rgba(255, 212, 120, 255),
            );
        }
        if state.tool == EditorTool::Scale {
            draw_rectangle(
                max_x - 6.0,
                max_y - 6.0,
                12.0,
                12.0,
                Color::from_rgba(185, 145, 255, 255),
            );
        }
    }
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
        &ellipsize(&format!("Drop {}", payload.name), 24),
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
    if state.tool != EditorTool::Paint {
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
        TileBrushMode::Collision => Color::from_rgba(255, 170, 90, 255),
    };
    draw_rectangle_lines(sx, sy, tile * size, tile * size, 2.0, color);
    if state.tile_brush_mode == TileBrushMode::Rectangle
        && let Some(start) = state.paint_start
        && let Some(end) = world_to_cell(game, world)
    {
        let min_x = start.0.min(end.0) as f32;
        let min_y = start.1.min(end.1) as f32;
        let max_x = start.0.max(end.0) as f32 + 1.0;
        let max_y = start.1.max(end.1) as f32 + 1.0;
        let (rx, ry) = world_to_screen(viewport, min_x, min_y);
        let (rw, rh) = world_to_screen(viewport, max_x, max_y);
        draw_rectangle_lines(rx, ry, rw - rx, rh - ry, 2.0, color);
    }
}

fn draw_ui_elements(game: &Game) {
    for entity in &game.units {
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
        draw_text(
            &ellipsize(&text, 24),
            x + 8.0,
            y + h * 0.65,
            18.0,
            Color::from_rgba(25, 28, 35, 255),
        );
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
        "{} | Tool {} | Brush {} {} | Tile {} | Layer {} | Snap {} | Scene {}{}",
        game.editor_workspace.active_mode.label(),
        state.tool.label(),
        game.brush_size,
        state.tile_brush_mode.label(),
        state.tile_brush,
        layer,
        if state.snap_to_grid { "on" } else { "off" },
        if game.scene_dirty { "dirty" } else { "clean" },
        play_bit
    );
    let w = measure_text(&text, None, 16, 1.0).width + 22.0;
    draw_rectangle(
        viewport.rect.x + 12.0,
        viewport.rect.y + 12.0,
        w,
        30.0,
        Color::from_rgba(14, 17, 22, 205),
    );
    draw_text(
        &text,
        viewport.rect.x + 23.0,
        viewport.rect.y + 32.0,
        16.0,
        Color::from_rgba(226, 232, 244, 255),
    );
}

fn draw_bottom_panel(game: &mut Game, state: &mut EditorState, rect: RectSpec) {
    if rect.h <= 0.0 {
        return;
    }
    draw_rect(rect, Color::from_rgba(18, 20, 26, 248));
    let mut x = rect.x + 12.0;
    for tab in [
        BottomTab::Console,
        BottomTab::Assets,
        BottomTab::Programming,
        BottomTab::Prefabs,
        BottomTab::Profiler,
    ] {
        let width = match tab {
            BottomTab::Console => 76.0,
            BottomTab::Assets => 70.0,
            BottomTab::Programming => 70.0,
            BottomTab::Prefabs => 78.0,
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
        BottomTab::Profiler => draw_profiler_panel(game, content),
    }
}

fn draw_console(game: &mut Game, rect: RectSpec) {
    draw_text("Console", rect.x + 14.0, rect.y + 20.0, 18.0, WHITE);
    if button(rect.x + 92.0, rect.y + 4.0, 54.0, 22.0, "Clear", false) {
        game.console.clear();
    }
    let lines = ((rect.h - 34.0) / 19.0).max(0.0) as usize;
    let start = game.console.entries.len().saturating_sub(lines);
    let mut y = rect.y + 42.0;
    for (channel, message) in game.console.entries.iter().skip(start) {
        let color = match channel.as_str() {
            "ERROR" => Color::from_rgba(255, 110, 110, 255),
            "WARNING" => Color::from_rgba(255, 210, 100, 255),
            "SCRIPT" => Color::from_rgba(130, 220, 255, 255),
            "SCENE" => Color::from_rgba(150, 255, 180, 255),
            "VALIDATOR" => Color::from_rgba(180, 230, 255, 255),
            "TILEMAP" => Color::from_rgba(255, 225, 120, 255),
            _ => Color::from_rgba(210, 216, 230, 255),
        };
        draw_text(
            &ellipsize(&format!("[{channel}] {message}"), 155),
            rect.x + 14.0,
            y,
            16.0,
            color,
        );
        y += 19.0;
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
        if let Ok(path) = game.create_rhai_script_asset("NewGameplayScript") {
            set_open_file_editor_state(state, &path);
        }
        state.bottom_tab = BottomTab::Programming;
    }
    if button(rect.x + 96.0, rect.y + 31.0, 78.0, 22.0, "Import", false) {
        game.console.log(
            "Import: usa create/import asset desde Browser o arrastra archivos al proyecto.",
            "ASSETS",
        );
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
    draw_rect(browser_panel, Color::from_rgba(18, 22, 30, 238));
    draw_rectangle_lines(
        browser_panel.x,
        browser_panel.y,
        browser_panel.w,
        browser_panel.h,
        1.0,
        Color::from_rgba(62, 74, 96, 255),
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
    let search_lower = state.content_search.to_ascii_lowercase();
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
        .filter(|asset| {
            search_lower.is_empty()
                || asset.name.to_ascii_lowercase().contains(&search_lower)
                || asset
                    .relative_path
                    .to_ascii_lowercase()
                    .contains(&search_lower)
                || asset
                    .asset_type
                    .to_ascii_lowercase()
                    .contains(&search_lower)
        })
        .cloned()
        .collect::<Vec<_>>();
    assets.sort_by(|a, b| {
        a.asset_type
            .cmp(&b.asset_type)
            .then_with(|| a.relative_path.cmp(&b.relative_path))
    });

    let asset_view = RectSpec {
        x: browser_panel.x + 10.0,
        y: browser_panel.y + 74.0,
        w: browser_panel.w - 20.0,
        h: browser_panel.h - 86.0,
    };
    draw_asset_grid(game, state, asset_view, &assets);
    draw_rect(preview_panel, Color::from_rgba(21, 25, 34, 238));
    draw_rectangle_lines(
        preview_panel.x,
        preview_panel.y,
        preview_panel.w,
        preview_panel.h,
        1.0,
        Color::from_rgba(62, 74, 96, 255),
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

fn draw_panel_chrome(rect: RectSpec, title: &str, subtitle: &str) {
    draw_rect(rect, Color::from_rgba(14, 17, 23, 248));
    draw_rectangle(
        rect.x,
        rect.y,
        rect.w,
        4.0,
        Color::from_rgba(76, 151, 255, 255),
    );
    draw_text(title, rect.x + 14.0, rect.y + 22.0, 18.0, WHITE);
    draw_text(
        subtitle,
        rect.x + 156.0,
        rect.y + 22.0,
        13.0,
        Color::from_rgba(156, 176, 205, 255),
    );
}

fn draw_sources_panel(game: &Game, state: &mut EditorState, rect: RectSpec) {
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
        "Sources",
        rect.x + 12.0,
        rect.y + 23.0,
        16.0,
        Color::from_rgba(220, 230, 242, 255),
    );
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
                Color::from_rgba(45, 73, 112, 255)
            } else if hovered {
                Color::from_rgba(36, 44, 58, 255)
            } else {
                Color::from_rgba(24, 29, 38, 255)
            },
        );
        draw_text(
            source_icon(icon),
            row.x + 8.0,
            y,
            14.0,
            Color::from_rgba(126, 205, 255, 255),
        );
        draw_text(
            label,
            row.x + 30.0,
            y,
            14.0,
            Color::from_rgba(226, 235, 244, 255),
        );
        draw_text(
            &count.to_string(),
            row.x + row.w - 28.0,
            y,
            12.0,
            Color::from_rgba(150, 166, 192, 255),
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
    draw_rect(
        rect,
        if state.content_search_active {
            Color::from_rgba(30, 39, 55, 255)
        } else {
            Color::from_rgba(12, 15, 21, 255)
        },
    );
    draw_rectangle_lines(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        1.0,
        if state.content_search_active {
            Color::from_rgba(76, 151, 255, 255)
        } else {
            Color::from_rgba(66, 76, 94, 255)
        },
    );
    draw_text(
        "Search",
        rect.x + 10.0,
        rect.y + 18.0,
        13.0,
        Color::from_rgba(122, 142, 172, 255),
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
            Color::from_rgba(88, 104, 132, 255)
        } else {
            Color::from_rgba(226, 235, 244, 255)
        },
    );
    if contains_mouse(rect) && is_mouse_button_pressed(MouseButton::Left) {
        state.content_search_active = true;
    }
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
        let color = asset_type_color(&asset.asset_type);
        draw_rectangle(card.x + 12.0, card.y + 10.0, card.w - 24.0, 48.0, color);
        draw_rectangle(
            card.x + 16.0,
            card.y + 14.0,
            card.w - 32.0,
            40.0,
            Color::from_rgba(12, 15, 20, 75),
        );
        draw_text(
            asset_icon(&asset.asset_type),
            card.x + 47.0,
            card.y + 43.0,
            22.0,
            Color::from_rgba(240, 246, 255, 255),
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
        draw_text(
            &ellipsize(&asset.name, 15),
            card.x + 10.0,
            card.y + 76.0,
            13.0,
            Color::from_rgba(232, 238, 248, 255),
        );
        draw_text(
            &ellipsize(&asset.asset_type, 14),
            card.x + 10.0,
            card.y + 94.0,
            11.0,
            Color::from_rgba(145, 162, 190, 255),
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
        "RhaiScript" | "VisualGraph" | "Data" | "Material" | "Shader" | "AudioEvent" => {
            open_project_file_in_editor(game, state, asset.relative_path.clone());
        }
        _ if asset.relative_path.ends_with(".rhai")
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

fn source_icon(kind: &str) -> &'static str {
    match kind {
        "assets" => "A",
        "image" => "I",
        "sound" => "S",
        "prefab" => "P",
        "code" => "{ }",
        "graph" => "G",
        "scene" => "L",
        "gear" => "*",
        _ => "C",
    }
}

fn asset_icon(asset_type: &str) -> &'static str {
    match asset_type {
        "Sprite" => "IMG",
        "Audio" | "AudioEvent" => "AUD",
        "Prefab" | "UI" => "PFB",
        "VisualGraph" => "NOD",
        "RhaiScript" => "RHA",
        "Scene" => "LVL",
        "Material" => "MAT",
        "Shader" => "SHD",
        _ => "DAT",
    }
}

fn asset_type_color(asset_type: &str) -> Color {
    match asset_type {
        "Sprite" => Color::from_rgba(52, 128, 176, 255),
        "Audio" | "AudioEvent" => Color::from_rgba(184, 114, 58, 255),
        "Prefab" | "UI" => Color::from_rgba(122, 94, 190, 255),
        "VisualGraph" => Color::from_rgba(54, 150, 137, 255),
        "RhaiScript" => Color::from_rgba(68, 118, 190, 255),
        "Scene" => Color::from_rgba(98, 150, 82, 255),
        "Material" | "Shader" => Color::from_rgba(137, 99, 184, 255),
        _ => Color::from_rgba(82, 96, 118, 255),
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
        game.asset_database.rebuild_dependency_graph().ok();
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
        "Sprite" => {
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
        }
        "Audio" => {
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
        "Material" | "Shader" => {
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
    draw_text("Programming", rect.x + 14.0, rect.y + 20.0, 18.0, WHITE);
    draw_text(
        &game.programming.summary(),
        rect.x + 128.0,
        rect.y + 20.0,
        15.0,
        Color::from_rgba(185, 202, 224, 255),
    );
    if button(rect.x + 430.0, rect.y + 4.0, 92.0, 22.0, "New Graph", false)
        && let Ok(path) = game.create_program_asset("LogAndMove")
    {
        set_open_file_editor_state(state, &path);
    }
    if button(rect.x + 530.0, rect.y + 4.0, 92.0, 22.0, "+ Script", false)
        && let Ok(path) = game.create_rhai_script_asset("PlayerController")
    {
        set_open_file_editor_state(state, &path);
    }
    if button(rect.x + 630.0, rect.y + 4.0, 74.0, 22.0, "Attach", false) {
        game.attach_program_template_to_selected("LogAndMove");
    }
    if button(rect.x + 712.0, rect.y + 4.0, 88.0, 22.0, "RTS Order", false) {
        game.attach_program_template_to_selected("RTSOrder");
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
    let templates = game.programming.templates.clone();
    for template in templates.iter().take(5) {
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
        draw_text(
            &template.name,
            row.x + 8.0,
            y,
            15.0,
            Color::from_rgba(226, 235, 244, 255),
        );
        draw_text(
            &ellipsize(&template.description, 72),
            row.x + 130.0,
            y,
            13.0,
            Color::from_rgba(150, 164, 186, 255),
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
        let label = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("asset");
        let active = current.as_ref() == Some(path);
        let width = (measure_text(label, None, 12, 1.0).width + 28.0).clamp(82.0, 154.0);
        if x + width > max_x {
            break;
        }
        if button(x, rect.y + 3.0, width, 20.0, &ellipsize(label, 18), active)
            && open_project_file_in_editor(game, state, path)
        {
            game.console
                .log(format!("Pestana activa: {label}"), "EDITOR");
        }
        x += width + 4.0;
    }
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
    draw_rect(rect, Color::from_rgba(15, 18, 25, 245));
    draw_rectangle_lines(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        1.0,
        Color::from_rgba(62, 74, 96, 255),
    );
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
        Color::from_rgba(232, 238, 248, 255),
    );
    let button_y = rect.y + 5.0;
    if button(
        rect.x + rect.w - 330.0,
        button_y,
        72.0,
        21.0,
        "+ Log",
        false,
    ) {
        game.add_node_to_open_graph("Log").ok();
    }
    if button(
        rect.x + rect.w - 252.0,
        button_y,
        76.0,
        21.0,
        "+ Move",
        false,
    ) {
        game.add_node_to_open_graph("Move").ok();
    }
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
    draw_graph_canvas_background(canvas);
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

    for connection in &view.connections {
        let Some(from) = view.nodes.iter().find(|node| node.id == connection.from) else {
            continue;
        };
        let Some(to) = view.nodes.iter().find(|node| node.id == connection.to) else {
            continue;
        };
        let start = graph_output_pin_pos(canvas, from);
        let end = graph_input_pin_pos(canvas, to);
        draw_graph_wire(start, end, Color::from_rgba(112, 184, 255, 255));
    }
    if let Some(from_id) = &state.graph_connect_from
        && let Some(from) = view.nodes.iter().find(|node| &node.id == from_id)
    {
        draw_graph_wire(
            graph_output_pin_pos(canvas, from),
            mouse_position(),
            Color::from_rgba(255, 210, 115, 255),
        );
    }

    if is_mouse_button_released(MouseButton::Left) {
        state.graph_drag_node = None;
    }
    for node in &view.nodes {
        let node_rect = graph_node_rect(canvas, node);
        let input_pin = pin_rect(graph_input_pin_pos(canvas, node));
        let output_pin = pin_rect(graph_output_pin_pos(canvas, node));
        let hovered = contains_mouse(node_rect);
        let selected = state.graph_selected_node.as_deref() == Some(node.id.as_str());
        draw_graph_node(node_rect, node, selected, hovered);
        draw_pin(input_pin, Color::from_rgba(92, 182, 255, 255));
        draw_pin(output_pin, Color::from_rgba(255, 186, 92, 255));

        if contains_mouse(output_pin) && is_mouse_button_pressed(MouseButton::Left) {
            state.graph_connect_from = Some(node.id.clone());
            state.graph_selected_node = Some(node.id.clone());
        } else if contains_mouse(input_pin) && is_mouse_button_pressed(MouseButton::Left) {
            if let Some(from) = state.graph_connect_from.clone()
                && game
                    .connect_open_graph_nodes(&from, &node.id)
                    .unwrap_or(false)
            {
                state.graph_connect_from = None;
            }
            state.graph_selected_node = Some(node.id.clone());
        } else if hovered && is_mouse_button_pressed(MouseButton::Left) {
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
        let x = (mouse.0 - state.graph_drag_offset.0 - canvas.x).max(8.0);
        let y = (mouse.1 - state.graph_drag_offset.1 - canvas.y).max(8.0);
        game.move_open_graph_node(&node_id, x as f64, y as f64).ok();
    }

    draw_graph_details(game, state, details, &view);
}

fn draw_graph_canvas_background(rect: RectSpec) {
    draw_rect(rect, Color::from_rgba(10, 13, 19, 255));
    let grid = 24.0;
    let mut x = rect.x;
    while x < rect.x + rect.w {
        draw_line(
            x,
            rect.y,
            x,
            rect.y + rect.h,
            1.0,
            Color::from_rgba(27, 33, 44, 255),
        );
        x += grid;
    }
    let mut y = rect.y;
    while y < rect.y + rect.h {
        draw_line(
            rect.x,
            y,
            rect.x + rect.w,
            y,
            1.0,
            Color::from_rgba(27, 33, 44, 255),
        );
        y += grid;
    }
}

fn graph_node_rect(canvas: RectSpec, node: &VisualGraphNodeView) -> RectSpec {
    RectSpec {
        x: canvas.x + node.x as f32,
        y: canvas.y + node.y as f32,
        w: 148.0,
        h: 74.0,
    }
}

fn graph_input_pin_pos(canvas: RectSpec, node: &VisualGraphNodeView) -> (f32, f32) {
    let rect = graph_node_rect(canvas, node);
    (rect.x - 1.0, rect.y + 42.0)
}

fn graph_output_pin_pos(canvas: RectSpec, node: &VisualGraphNodeView) -> (f32, f32) {
    let rect = graph_node_rect(canvas, node);
    (rect.x + rect.w + 1.0, rect.y + 42.0)
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

fn draw_graph_wire(start: (f32, f32), end: (f32, f32), color: Color) {
    let mid_x = (start.0 + end.0) * 0.5;
    let points = [start, (mid_x, start.1), (mid_x, end.1), end];
    for pair in points.windows(2) {
        draw_line(pair[0].0, pair[0].1, pair[1].0, pair[1].1, 2.0, color);
    }
}

fn draw_graph_node(rect: RectSpec, node: &VisualGraphNodeView, selected: bool, hovered: bool) {
    let color = graph_node_color(&node.node_type);
    draw_rect(
        rect,
        if selected {
            Color::from_rgba(38, 52, 72, 255)
        } else if hovered {
            Color::from_rgba(32, 41, 56, 255)
        } else {
            Color::from_rgba(24, 30, 42, 255)
        },
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
        Color::from_rgba(246, 250, 255, 255),
    );
    draw_text(
        &ellipsize(&node.id, 18),
        rect.x + 10.0,
        rect.y + 42.0,
        12.0,
        Color::from_rgba(178, 193, 216, 255),
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
        "EventStart" | "EventUpdate" | "EventClick" | "EventTrigger" => {
            Color::from_rgba(155, 66, 72, 255)
        }
        "Move" => Color::from_rgba(55, 128, 190, 255),
        "Log" => Color::from_rgba(70, 138, 104, 255),
        "SetVariable" | "SetEnabled" => Color::from_rgba(132, 102, 182, 255),
        "Heal" | "Damage" => Color::from_rgba(178, 118, 52, 255),
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
    let selected = state
        .graph_selected_node
        .as_ref()
        .and_then(|id| view.nodes.iter().find(|node| &node.id == id));
    if let Some(node) = selected {
        draw_text(
            &format!("Node: {}", node.title),
            rect.x + 10.0,
            rect.y + 75.0,
            13.0,
            Color::from_rgba(126, 205, 255, 255),
        );
        draw_text(
            &format!("id {}", node.id),
            rect.x + 10.0,
            rect.y + 96.0,
            12.0,
            Color::from_rgba(190, 202, 222, 255),
        );
        draw_text(
            &format!("next {}", node.next.as_deref().unwrap_or("none")),
            rect.x + 10.0,
            rect.y + 115.0,
            12.0,
            Color::from_rgba(190, 202, 222, 255),
        );
    } else {
        draw_text(
            "Select a node or drag from output pin.",
            rect.x + 10.0,
            rect.y + 76.0,
            12.0,
            Color::from_rgba(132, 150, 176, 255),
        );
    }
    let mut y = rect.y + 150.0;
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
    draw_rect(
        rect,
        if state.code_editor_active {
            Color::from_rgba(25, 32, 42, 245)
        } else {
            Color::from_rgba(21, 25, 34, 235)
        },
    );
    draw_rectangle_lines(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        1.0,
        Color::from_rgba(62, 74, 96, 255),
    );
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
    draw_text(
        &ellipsize(&format!("Editor: {title}{dirty}"), 60),
        rect.x + 10.0,
        rect.y + 20.0,
        16.0,
        Color::from_rgba(232, 238, 248, 255),
    );
    let button_y = rect.y + 5.0;
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

    let code_area = RectSpec {
        x: rect.x + 8.0,
        y: rect.y + 34.0,
        w: rect.w - 16.0,
        h: rect.h - 62.0,
    };
    draw_rect(code_area, Color::from_rgba(13, 16, 23, 255));
    if contains_mouse(code_area) && is_mouse_button_pressed(MouseButton::Left) {
        state.code_editor_active = true;
        let row = ((mouse_position().1 - code_area.y) / 17.0).max(0.0) as usize;
        state.code_cursor_line =
            (state.code_scroll_line + row).min(game.script_editor.lines.len().saturating_sub(1));
        state.code_cursor_column = 0;
    }

    if game.script_editor.document.path.is_none() {
        draw_text(
            "Sin archivo abierto.",
            code_area.x + 12.0,
            code_area.y + 28.0,
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
        draw_text(
            &ellipsize(line, 112),
            code_area.x + 48.0,
            y,
            13.0,
            Color::from_rgba(214, 224, 238, 255),
        );
        if index == state.code_cursor_line && state.code_editor_active {
            let cursor_x = code_area.x + 48.0 + (state.code_cursor_column as f32 * 7.2);
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

fn draw_profiler_panel(game: &Game, rect: RectSpec) {
    draw_text("Profiler", rect.x + 14.0, rect.y + 20.0, 18.0, WHITE);
    let mut x = rect.x + 120.0;
    let stats = [
        format!("FPS {:.0}", game.diagnostics.fps),
        format!("Frame {:.2} ms", game.diagnostics.frame_time_ms),
        format!("Entities {}", game.units.len()),
        format!("Mode {}", game.mode),
        format!(
            "Budget {}",
            game.editor_workspace
                .performance_status(game.diagnostics.frame_time_ms)
        ),
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

    let mut y = rect.y + 46.0;
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

    let mut y2 = rect.y + 46.0;
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

    let mut y3 = rect.y + 46.0;
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

    let mut y4 = rect.y + 46.0;
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
}

fn draw_command_palette(game: &mut Game, state: &mut EditorState, sw: f32, sh: f32) {
    let panel_h = (sh - 50.0).clamp(560.0, 760.0);
    let panel = RectSpec {
        x: sw * 0.5 - 300.0,
        y: sh * 0.5 - panel_h * 0.5,
        w: 600.0,
        h: panel_h,
    };
    draw_rectangle(0.0, 0.0, sw, sh, Color::from_rgba(0, 0, 0, 105));
    draw_rect(panel, Color::from_rgba(28, 32, 42, 248));
    draw_rectangle_lines(
        panel.x,
        panel.y,
        panel.w,
        panel.h,
        1.0,
        Color::from_rgba(88, 98, 120, 255),
    );
    draw_text(
        "Command Palette",
        panel.x + 18.0,
        panel.y + 30.0,
        24.0,
        WHITE,
    );
    draw_text(
        "Click para ejecutar. Esc cierra.",
        panel.x + 18.0,
        panel.y + 54.0,
        15.0,
        Color::from_rgba(165, 176, 196, 255),
    );

    let commands = [
        ("Spawn advanced unit", "spawn_unit"),
        ("Spawn enemy AI", "spawn_enemy"),
        ("Spawn resource node", "spawn_resource"),
        ("Spawn RTS command center", "spawn_rts_base"),
        ("Queue worker on selected building", "queue_worker"),
        ("Place Barracks construction site", "place_barracks"),
        ("Create TopDown starter scene", "starter_topdown"),
        ("Create Platformer starter scene", "starter_platformer"),
        ("Create RTS skirmish scene", "rts_skirmish"),
        ("Toggle Play Mode (snapshot)", "toggle_play"),
        ("Add Health to selected", "add_health"),
        ("Add NavAgent to selected", "add_nav"),
        ("Add VisualScript to selected", "add_visual"),
        ("Attach Graph LogAndMove", "attach_graph_log"),
        ("Attach Graph HealthPickup", "attach_graph_health"),
        ("Attach Graph RTSOrder", "attach_graph_rts"),
        ("Create visual graph asset", "create_graph"),
        ("Save selected as prefab", "save_prefab"),
        ("Create prefab variant", "variant_prefab"),
        ("Instantiate first prefab", "instantiate_prefab"),
        ("Workspace world", "workspace_world"),
        ("Workspace scripting", "workspace_script"),
        ("Workspace prefab", "workspace_prefab"),
        ("Workspace profiling", "workspace_profile"),
        ("Duplicate selected", "duplicate"),
        ("Delete selected", "delete"),
        ("Save scene", "save"),
        ("Validate project", "validate"),
        ("Refresh asset database", "refresh"),
        ("Build manifest", "manifest"),
        ("Export runtime debug", "export_debug"),
        ("Export runtime release", "export_release"),
        ("Create RTS template files", "template_rts"),
        ("Create ActionRPG template files", "template_actionrpg"),
        ("Create Survival template files", "template_survival"),
        ("Cycle tilemap layer", "cycle_layer"),
        ("Clear console", "clear_console"),
    ];

    let mut y = panel.y + 78.0;
    for (label, command) in commands {
        let row = RectSpec {
            x: panel.x + 14.0,
            y: y - 18.0,
            w: panel.w - 28.0,
            h: 23.0,
        };
        let hovered = contains_mouse(row);
        draw_rect(
            row,
            if hovered {
                Color::from_rgba(58, 67, 86, 255)
            } else {
                Color::from_rgba(37, 43, 55, 255)
            },
        );
        draw_text(
            label,
            row.x + 10.0,
            y,
            16.0,
            Color::from_rgba(232, 236, 245, 255),
        );
        if hovered && is_mouse_button_pressed(MouseButton::Left) {
            run_palette_command(game, state, command);
            state.command_palette = false;
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
        Color::from_rgba(160, 172, 196, 255),
    );
}

fn run_palette_command(game: &mut Game, state: &mut EditorState, command: &str) {
    match command {
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
        "toggle_play" => toggle_play_mode(game),
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
        "create_graph" => {
            if let Ok(path) = game.create_program_asset("LogAndMove") {
                set_open_file_editor_state(state, &path);
            }
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
        "command_palette" => state.command_palette = true,
        "toggle_browser" => {
            state.show_console = !state.show_console;
            state.bottom_tab = BottomTab::Assets;
        }
        "toggle_hierarchy" => state.show_hierarchy = !state.show_hierarchy,
        "toggle_inspector" => state.show_inspector = !state.show_inspector,
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
    draw_rect(rect, Color::from_rgba(30, 34, 43, 255));
    let layer = &game.tilemap_layers.layers[game.tilemap_layers.active_layer].name;
    draw_text(
        &format!(
            "FPS {:.0} | frame {:.2} ms avg {:.2} | {} | zoom {:.2} | camera {:.0},{:.0} | tool {} | layer {} | {}{}{}",
            game.diagnostics.fps,
            game.diagnostics.frame_time_ms,
            game.diagnostics.average_frame_time_ms,
            game.editor_workspace
                .performance_status(game.diagnostics.frame_time_ms),
            game.camera.zoom,
            game.camera.x,
            game.camera.y,
            state.tool.label(),
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
        Color::from_rgba(205, 212, 226, 255),
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
    } else if let Some(label) = game.history.undo(&mut game.units) {
        game.clear_selection();
        game.sync_world();
        game.mark_scene_dirty("Undo");
        game.console.log(format!("Undo: {label}"), "EDITOR");
    }
}

fn redo(game: &mut Game) {
    if let Some(label) = game.redo_editor_command() {
        game.console.log(format!("Redo: {label}"), "EDITOR");
    } else if let Some(label) = game.history.redo(&mut game.units) {
        game.clear_selection();
        game.sync_world();
        game.mark_scene_dirty("Redo");
        game.console.log(format!("Redo: {label}"), "EDITOR");
    }
}

fn selected_id(game: &Game) -> Option<u64> {
    game.selected_units.first().copied()
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
    x >= rect.x && y >= rect.y && x <= rect.x + rect.w && y <= rect.y + rect.h
}

fn contains_mouse(rect: RectSpec) -> bool {
    let (mx, my) = mouse_position();
    contains(rect, mx, my)
}

fn command_modifier_down() -> bool {
    is_key_down(KeyCode::LeftControl)
        || is_key_down(KeyCode::RightControl)
        || is_key_down(KeyCode::LeftSuper)
        || is_key_down(KeyCode::RightSuper)
}

fn button(x: f32, y: f32, w: f32, h: f32, label: &str, active: bool) -> bool {
    let rect = RectSpec { x, y, w, h };
    let hovered = contains_mouse(rect);
    let color = if active {
        Color::from_rgba(68, 126, 196, 255)
    } else if hovered {
        Color::from_rgba(58, 66, 82, 255)
    } else {
        Color::from_rgba(38, 44, 56, 255)
    };
    draw_rectangle(x, y + 1.0, w, h, Color::from_rgba(0, 0, 0, 55));
    draw_rectangle(x, y, w, h, color);
    draw_rectangle(x, y, w, 1.0, Color::from_rgba(255, 255, 255, 24));
    draw_rectangle_lines(
        x,
        y,
        w,
        h,
        1.0,
        if hovered || active {
            Color::from_rgba(120, 150, 205, 255)
        } else {
            Color::from_rgba(78, 86, 104, 255)
        },
    );
    if active {
        draw_rectangle(
            x + 3.0,
            y + h - 3.0,
            w - 6.0,
            2.0,
            Color::from_rgba(150, 210, 255, 255),
        );
    }
    let font_size = 14;
    let shown = ellipsize(label, (w / 7.0).max(3.0) as usize);
    let measure = measure_text(&shown, None, font_size, 1.0);
    draw_text(
        &shown,
        x + ((w - measure.width) * 0.5).max(4.0),
        y + h * 0.5 + measure.height * 0.34,
        font_size as f32,
        WHITE,
    );
    hovered && is_mouse_button_pressed(MouseButton::Left)
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
    let mut game = match Game::from_project(&build_root, true) {
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
    loop {
        if is_key_pressed(KeyCode::Escape) {
            break;
        }
        let dt = get_frame_time() as f64;
        let sw = screen_width();
        let sh = screen_height();
        let nav = EditorState::default();
        handle_camera_input(&mut game, dt, &nav);
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
            "Runtime player — WASD mover cámara, Q/E zoom, Esc salir",
            14.0,
            18.0,
            16.0,
            Color::from_rgba(200, 210, 230, 255),
        );
        next_frame().await;
    }
}
