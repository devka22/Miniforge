use std::env;
use std::path::PathBuf;

use macroquad::prelude::*;
use miniforge::core::game::Game;
use miniforge::engine::asset_tools::AssetTools;
use miniforge::engine::component::{advanced_component_category, advanced_component_types};
use miniforge::engine::editor_workspace::WorkspaceMode;
use miniforge::entities::game_object::GameObject;
use miniforge::systems::command_system::CommandSystem;
use miniforge::systems::rts_system::RTSSystem;

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
    Paint,
}

impl EditorTool {
    fn label(self) -> &'static str {
        match self {
            Self::Select => "Select",
            Self::Move => "Move",
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

#[derive(Debug, Clone)]
struct EditorState {
    paused: bool,
    show_console: bool,
    show_grid: bool,
    show_hierarchy: bool,
    show_inspector: bool,
    command_palette: bool,
    tool: EditorTool,
    bottom_tab: BottomTab,
    tile_brush: i32,
    drag_entity: Option<u64>,
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
            tool: EditorTool::Select,
            bottom_tab: BottomTab::Console,
            tile_brush: 1,
            drag_entity: None,
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

fn window_conf() -> Conf {
    Conf {
        window_title: "MiniForge 0.6.0 Beta - Rust Editor".to_string(),
        window_width: 1360,
        window_height: 820,
        high_dpi: true,
        sample_count: 4,
        window_resizable: true,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
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
            miniforge::version_label(),
            game.project_path.display()
        );
        return;
    }

    game.refresh_assets().ok();
    game.console.log("Ventana Rust iniciada", "ENGINE");
    game.console
        .log("Ctrl+P abre comandos. 1/2/3 cambia herramienta.", "EDITOR");
    let mut state = EditorState {
        tile_brush: game.tile_brush.max(1),
        ..Default::default()
    };

    loop {
        let dt = get_frame_time() as f64;
        let sw = screen_width();
        let sh = screen_height();

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
            draw_inspector(&mut game, layout.right);
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
    let top_h = 74.0;
    let status_h = 26.0;
    let console_h = if show_console {
        (sh * 0.24).clamp(138.0, 210.0)
    } else {
        0.0
    };
    let left_w = if show_left {
        (sw * 0.19).clamp(220.0, 286.0)
    } else {
        0.0
    };
    let right_w = if show_right {
        (sw * 0.23).clamp(286.0, 340.0)
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

fn handle_shortcuts(game: &mut Game, state: &mut EditorState) -> bool {
    let command = command_modifier_down();

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
        set_tool(game, state, EditorTool::Paint);
    }
    if is_key_pressed(KeyCode::Tab) {
        state.show_console = true;
        state.bottom_tab = state.bottom_tab.next();
    }
    if is_key_pressed(KeyCode::L) && state.tool == EditorTool::Paint {
        game.cycle_tilemap_layer();
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
        if is_key_pressed(KeyCode::R) {
            refresh_assets(game);
        }
        if is_key_pressed(KeyCode::G) && game.create_program_asset("LogAndMove").is_ok() {
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
    if !contains(viewport.rect, mx, my) || state.command_palette {
        state.drag_entity = None;
        return;
    }

    let world = screen_to_world(viewport, mx, my);
    match state.tool {
        EditorTool::Paint => {
            if is_mouse_button_down(MouseButton::Left) {
                paint_at(game, state, world);
            }
        }
        EditorTool::Move => {
            if is_mouse_button_pressed(MouseButton::Left) {
                state.drag_entity = find_entity_at(game, world);
                if let Some(id) = state.drag_entity {
                    game.select_entity(id);
                    game.console.log(format!("Moviendo entity #{id}"), "EDITOR");
                }
            }
            if is_mouse_button_down(MouseButton::Left) {
                if let Some(id) = state.drag_entity
                    && let Some(entity) = game.get_entity_by_id_mut(id)
                {
                    entity.x = (world.0 * 4.0).round() as f64 / 4.0;
                    entity.y = (world.1 * 4.0).round() as f64 / 4.0;
                    entity.path.clear();
                    entity.sync_to_components();
                    game.mark_scene_dirty("Move Entity");
                }
            } else {
                if state.drag_entity.is_some() {
                    game.sync_world();
                }
                state.drag_entity = None;
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

fn paint_at(game: &mut Game, state: &EditorState, world: (f32, f32)) {
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
            if game.paint_tile(x as usize, y as usize, state.tile_brush) {
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
    if game.mode == "PLAY" {
        game.play_mode_manager
            .exit_play_mode(&mut game.units, &mut game.mode);
        game.console.log("Editor Mode", "ENGINE");
    } else {
        game.play_mode_manager
            .enter_play_mode(&game.units, &mut game.mode);
        game.console.log("Play Mode", "ENGINE");
    }
    game.sync_world();
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
    draw_text(
        &format!(
            "{} | {} | {}",
            miniforge::version_label(),
            game.mode,
            game.scene_summary()
        ),
        rect.x + 212.0,
        rect.y + 23.0,
        15.0,
        Color::from_rgba(200, 206, 220, 255),
    );

    let mut x = rect.x + 16.0;
    let y = rect.y + 39.0;
    let play_label = if game.mode == "PLAY" { "Stop" } else { "Play" };
    if button(x, y, 58.0, 24.0, play_label, game.mode == "PLAY") {
        toggle_play_mode(game);
    }
    x += 64.0;
    if button(x, y, 58.0, 24.0, "Save", false) {
        save_scene(game);
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
    if button(x, y, 72.0, 24.0, "Manifest", false) {
        build_manifest(game);
    }
    x += 78.0;
    if button(x, y, 66.0, 24.0, "+Graph", false) && game.create_program_asset("LogAndMove").is_ok()
    {
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

    for tool in [EditorTool::Select, EditorTool::Move, EditorTool::Paint] {
        let width = match tool {
            EditorTool::Select => 62.0,
            EditorTool::Move => 54.0,
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

    let mut y = rect.y + 76.0;
    let max_rows = ((rect.h - 90.0) / 25.0).max(0.0) as usize;
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

fn draw_inspector(game: &mut Game, rect: RectSpec) {
    if rect.w <= 0.0 {
        return;
    }
    draw_rect(rect, Color::from_rgba(25, 29, 38, 255));
    draw_text("Inspector", rect.x + 14.0, rect.y + 26.0, 22.0, WHITE);
    let Some(id) = selected_id(game) else {
        draw_text(
            "No hay entidad seleccionada.",
            rect.x + 14.0,
            rect.y + 58.0,
            17.0,
            Color::from_rgba(170, 178, 196, 255),
        );
        return;
    };
    let Some(entity) = game.get_entity_by_id(id).cloned() else {
        return;
    };

    let lines = [
        format!("Name: {}", entity.name),
        format!("Type: {}", entity.entity_type),
        format!("ID: {}", entity.id),
        format!("Position: {:.2}, {:.2}", entity.x, entity.y),
        format!("Rotation: {:.1}", entity.rotation),
        format!("Scale: {:.2}, {:.2}", entity.scale_x, entity.scale_y),
        format!("Tag: {}", entity.tag),
        format!("Layer: {}", entity.layer),
        format!("Components: {}", entity.components.len()),
    ];
    let mut y = rect.y + 58.0;
    for line in lines {
        draw_text(
            &ellipsize(&line, 34),
            rect.x + 14.0,
            y,
            16.0,
            Color::from_rgba(220, 225, 238, 255),
        );
        y += 21.0;
    }

    y += 8.0;
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

    draw_text("Add / Presets", rect.x + 14.0, y, 18.0, WHITE);
    y += 24.0;
    let buttons = [
        ("Health", "Health"),
        ("Body", "Rigidbody2D"),
        ("Inventory", "Inventory"),
        ("Nav", "NavAgent"),
        ("AI", "AIController"),
        ("Light", "Light2D"),
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
    y += 34.0;
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
    for component in entity
        .components
        .iter()
        .take(((rect.h - y) / 20.0).max(0.0) as usize)
    {
        let category = advanced_component_category(&component.component_type).unwrap_or("Core");
        draw_text(
            &format!("- {} [{}]", component.component_type, category),
            rect.x + 18.0,
            y,
            15.0,
            Color::from_rgba(170, 210, 255, 255),
        );
        y += 20.0;
    }
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

    draw_paint_cursor(game, state, viewport);
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
    draw_rectangle_lines(
        sx,
        sy,
        tile * size,
        tile * size,
        2.0,
        Color::from_rgba(255, 235, 120, 255),
    );
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
    let text = format!(
        "{} | Tool {} | Brush {} | Tile {} | Layer {} | Scene {}",
        game.editor_workspace.active_mode.label(),
        state.tool.label(),
        game.brush_size,
        state.tile_brush,
        layer,
        if game.scene_dirty { "dirty" } else { "clean" }
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
        BottomTab::Assets => draw_assets_panel(game, content),
        BottomTab::Programming => draw_programming_panel(game, content),
        BottomTab::Prefabs => draw_prefab_panel(game, content),
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

fn draw_assets_panel(game: &mut Game, rect: RectSpec) {
    let mut counts = std::collections::BTreeMap::<String, usize>::new();
    for asset in game.asset_database.assets.values() {
        *counts.entry(asset.asset_type.clone()).or_default() += 1;
    }
    draw_text(
        &format!(
            "Content Browser ({}) | legacy py {}",
            game.asset_database.assets.len(),
            game.legacy_python_asset_count()
        ),
        rect.x + 14.0,
        rect.y + 20.0,
        18.0,
        WHITE,
    );
    if button(rect.x + 302.0, rect.y + 4.0, 70.0, 22.0, "Refresh", false) {
        refresh_assets(game);
    }
    if button(rect.x + 380.0, rect.y + 4.0, 76.0, 22.0, "Manifest", false) {
        build_manifest(game);
    }
    if button(rect.x + 464.0, rect.y + 4.0, 86.0, 22.0, "+ Graph", false) {
        game.create_program_asset("LogAndMove").ok();
    }
    if button(rect.x + 558.0, rect.y + 4.0, 92.0, 22.0, "+ Prefab", false) {
        game.save_selected_as_prefab().ok();
    }

    let mut cx = rect.x + 14.0;
    for (asset_type, count) in counts.iter().take(8) {
        let text = format!("{asset_type} {count}");
        let width = measure_text(&text, None, 13, 1.0).width + 16.0;
        draw_rectangle(
            cx,
            rect.y + 30.0,
            width,
            19.0,
            Color::from_rgba(38, 44, 56, 255),
        );
        draw_text(
            &text,
            cx + 8.0,
            rect.y + 44.0,
            13.0,
            Color::from_rgba(184, 198, 222, 255),
        );
        cx += width + 6.0;
    }

    let mut y = rect.y + 72.0;
    let max_rows = ((rect.h - 74.0) / 22.0).max(0.0) as usize;
    for asset in game.asset_database.assets.values().take(max_rows) {
        let color = match asset.asset_type.as_str() {
            "Sprite" => Color::from_rgba(130, 220, 255, 255),
            "Audio" => Color::from_rgba(255, 185, 120, 255),
            "Prefab" => Color::from_rgba(180, 145, 255, 255),
            "VisualGraph" => Color::from_rgba(110, 230, 205, 255),
            "LegacyScript" => Color::from_rgba(255, 198, 120, 255),
            "Data" => Color::from_rgba(160, 235, 180, 255),
            _ => Color::from_rgba(210, 216, 230, 255),
        };
        let warning = if asset.compatibility.is_empty() {
            ""
        } else {
            " !"
        };
        draw_text(
            &ellipsize(
                &format!(
                    "{}{} | {} | {} KB | {}",
                    asset.asset_type,
                    warning,
                    asset.name,
                    (asset.size_bytes as f64 / 1024.0).ceil() as u64,
                    asset.relative_path
                ),
                150,
            ),
            rect.x + 14.0,
            y,
            16.0,
            color,
        );
        if !asset.compatibility.is_empty() {
            draw_text(
                &ellipsize(&asset.compatibility.join(" / "), 70),
                rect.x + rect.w - 430.0,
                y,
                13.0,
                Color::from_rgba(255, 206, 130, 255),
            );
        }
        y += 20.0;
    }
}

fn draw_programming_panel(game: &mut Game, rect: RectSpec) {
    draw_text("Programming", rect.x + 14.0, rect.y + 20.0, 18.0, WHITE);
    draw_text(
        &game.programming.summary(),
        rect.x + 128.0,
        rect.y + 20.0,
        15.0,
        Color::from_rgba(185, 202, 224, 255),
    );
    if button(rect.x + 430.0, rect.y + 4.0, 92.0, 22.0, "New Graph", false) {
        game.create_program_asset("LogAndMove").ok();
    }
    if button(rect.x + 530.0, rect.y + 4.0, 96.0, 22.0, "Attach", false) {
        game.attach_program_template_to_selected("LogAndMove");
    }
    if button(
        rect.x + 634.0,
        rect.y + 4.0,
        102.0,
        22.0,
        "RTS Order",
        false,
    ) {
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
            rect.x + 430.0,
            y2,
            14.0,
            Color::from_rgba(212, 220, 235, 255),
        );
        y2 += 20.0;
    }
    if game.programming.last_warnings.is_empty() {
        draw_text(
            "Graph validator clean",
            rect.x + 760.0,
            rect.y + 70.0,
            14.0,
            Color::from_rgba(135, 230, 165, 255),
        );
    } else {
        let mut wy = rect.y + 70.0;
        for warning in game.programming.last_warnings.iter().take(4) {
            draw_text(
                &ellipsize(warning, 46),
                rect.x + 760.0,
                wy,
                14.0,
                Color::from_rgba(255, 206, 130, 255),
            );
            wy += 18.0;
        }
    }
}

fn draw_prefab_panel(game: &mut Game, rect: RectSpec) {
    draw_text("Prefabs", rect.x + 14.0, rect.y + 20.0, 18.0, WHITE);
    draw_text(
        &game.advanced_prefabs.status_line(),
        rect.x + 110.0,
        rect.y + 20.0,
        15.0,
        Color::from_rgba(205, 214, 230, 255),
    );
    if button(rect.x + 380.0, rect.y + 4.0, 96.0, 22.0, "Save Sel", false) {
        game.save_selected_as_prefab().ok();
    }
    if button(rect.x + 484.0, rect.y + 4.0, 78.0, 22.0, "Variant", false) {
        game.create_selected_prefab_variant().ok();
    }
    if button(rect.x + 570.0, rect.y + 4.0, 96.0, 22.0, "Instance", false) {
        let (x, y) = spawn_position(game);
        game.instantiate_first_prefab(x, y).ok();
    }

    let mut y = rect.y + 50.0;
    for prefab in game
        .asset_database
        .assets
        .values()
        .filter(|asset| asset.asset_type == "Prefab")
        .take(((rect.h - 54.0) / 22.0).max(0.0) as usize)
    {
        draw_text(
            &ellipsize(
                &format!(
                    "{} | {} KB | {}",
                    prefab.name,
                    (prefab.size_bytes as f64 / 1024.0).ceil() as u64,
                    prefab.relative_path
                ),
                96,
            ),
            rect.x + 14.0,
            y,
            15.0,
            Color::from_rgba(185, 155, 255, 255),
        );
        y += 22.0;
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
            rect.x + rect.w - 470.0,
            rect.y + 52.0,
            14.0,
            Color::from_rgba(205, 214, 232, 255),
        );
    }
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
            game.create_program_asset("LogAndMove").ok();
            state.show_console = true;
            state.bottom_tab = BottomTab::Programming;
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
        "save" => save_scene(game),
        "validate" => {
            game.validate_project();
        }
        "refresh" => {
            refresh_assets(game);
            state.show_console = true;
            state.bottom_tab = BottomTab::Assets;
        }
        "manifest" => build_manifest(game),
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
            "FPS {:.0} | frame {:.2} ms avg {:.2} | {} | zoom {:.2} | camera {:.0},{:.0} | tool {} | layer {} | {}{}",
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

fn create_new_scene(game: &mut Game) {
    let name = format!(
        "scene_{}",
        game.scene_manager
            .list_scenes()
            .map(|scenes| scenes.len() + 1)
            .unwrap_or(1)
    );
    match game.scene_manager.create_new_scene(&name) {
        Ok(_) => game.console.log(format!("Escena creada: {name}"), "SCENE"),
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
    if let Some(label) = game.history.undo(&mut game.units) {
        game.clear_selection();
        game.sync_world();
        game.mark_scene_dirty("Undo");
        game.console.log(format!("Undo: {label}"), "EDITOR");
    }
}

fn redo(game: &mut Game) {
    if let Some(label) = game.history.redo(&mut game.units) {
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
        Color::from_rgba(70, 135, 210, 255)
    } else if hovered {
        Color::from_rgba(62, 68, 84, 255)
    } else {
        Color::from_rgba(43, 48, 60, 255)
    };
    draw_rectangle(x, y, w, h, color);
    draw_rectangle_lines(x, y, w, h, 1.0, Color::from_rgba(92, 99, 118, 255));
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
