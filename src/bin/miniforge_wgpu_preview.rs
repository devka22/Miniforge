use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use miniforge::engine::ui_runtime::UiRuntime;
use miniforge::render::backend::{
    RenderBackend, SpriteBlendMode, SpriteDrawCommand, SpriteDrawOptions, TextDrawCommand,
    TextWrapMode, WgpuBackend,
};
use miniforge::render::runtime_scene_2d::{
    RenderTargetUpdateMode2D, RuntimeScene2DStats, RuntimeTexture2D, draw_engine_runtime_scene_2d,
    draw_engine_runtime_world_to_render_target_2d, entity_normal_map_path, entity_sprite_path,
    entity_ui_sprite_path, runtime_post_process_2d, runtime_render_target_cameras,
    scene_ui_sprite_paths, ui_document_sprite_paths,
};
use miniforge::runtime::engine_runtime::EngineRuntime;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

#[cfg(target_os = "macos")]
use winit::platform::macos::{ActivationPolicy, EventLoopBuilderExtMacOS};

const INITIAL_WIDTH: u32 = 960;
const INITIAL_HEIGHT: u32 = 540;

struct PreviewApp {
    window: Option<Arc<Window>>,
    backend: Option<WgpuBackend>,
    started_at: Instant,
    frames: u64,
    autotest_frames: Option<u64>,
    simulate_device_loss_frame: Option<u64>,
    runtime: Option<EngineRuntime>,
    project_path: Option<PathBuf>,
    texture_ids: BTreeMap<String, RuntimeTexture2D>,
    rendered_once_targets: BTreeSet<u64>,
    observed_device_loss_recoveries: u64,
    last_scene_stats: RuntimeScene2DStats,
    previous_frame_at: Instant,
    input_left: bool,
    input_right: bool,
    input_up: bool,
    input_down: bool,
    input_run: bool,
    cursor_position: Option<(f64, f64)>,
    ui_runtime: UiRuntime,
}

impl PreviewApp {
    fn new(project_path: Option<PathBuf>) -> Result<Self, Box<dyn Error>> {
        let mut runtime = project_path.as_ref().map(EngineRuntime::new).transpose()?;
        if let Some(runtime) = runtime.as_mut() {
            runtime.run_headless_once(0.0);
        }
        let now = Instant::now();
        Ok(Self {
            window: None,
            backend: None,
            started_at: now,
            frames: 0,
            autotest_frames: std::env::var("MINIFORGE_WGPU_AUTOTEST_FRAMES")
                .ok()
                .and_then(|value| value.parse().ok()),
            simulate_device_loss_frame: std::env::var("MINIFORGE_WGPU_SIMULATE_DEVICE_LOSS_FRAME")
                .ok()
                .and_then(|value| value.parse().ok()),
            runtime,
            project_path,
            texture_ids: BTreeMap::new(),
            rendered_once_targets: BTreeSet::new(),
            observed_device_loss_recoveries: 0,
            last_scene_stats: RuntimeScene2DStats::default(),
            previous_frame_at: now,
            input_left: false,
            input_right: false,
            input_up: false,
            input_down: false,
            input_run: false,
            cursor_position: None,
            ui_runtime: UiRuntime::default(),
        })
    }

    fn create_renderer(&mut self, event_loop: &ActiveEventLoop) -> Result<(), String> {
        let attributes = Window::default_attributes()
            .with_title("MiniForge wgpu Surface Preview")
            .with_inner_size(LogicalSize::new(INITIAL_WIDTH, INITIAL_HEIGHT))
            .with_min_inner_size(LogicalSize::new(480, 270));
        let window = Arc::new(
            event_loop
                .create_window(attributes)
                .map_err(|error| format!("window creation failed: {error}"))?,
        );
        let size = window.inner_size();
        let mut backend = WgpuBackend::new(true, cfg!(target_os = "macos"));
        backend
            .resize(size.width, size.height)
            .map_err(|error| error.to_string())?;
        backend
            .init_with_surface(window.clone())
            .map_err(|error| error.to_string())?;
        backend.set_clear_color([0.025, 0.032, 0.048, 1.0]);
        backend
            .upload_texture_rgba8(
                1,
                4,
                4,
                &[
                    72, 201, 145, 255, 72, 201, 145, 255, 21, 91, 72, 255, 21, 91, 72, 255, 72,
                    201, 145, 255, 72, 201, 145, 255, 21, 91, 72, 255, 21, 91, 72, 255, 21, 91, 72,
                    255, 21, 91, 72, 255, 72, 201, 145, 255, 72, 201, 145, 255, 21, 91, 72, 255,
                    21, 91, 72, 255, 72, 201, 145, 255, 72, 201, 145, 255,
                ],
            )
            .map_err(|error| error.to_string())?;
        self.load_project_textures(&mut backend)?;
        self.rendered_once_targets.clear();
        self.observed_device_loss_recoveries = backend.device_loss_recoveries;

        let api = backend
            .caps
            .as_ref()
            .map(|caps| format!("{:?} · {}", caps.api, caps.device_name))
            .unwrap_or_else(|| "wgpu".to_string());
        let content = self
            .runtime
            .as_ref()
            .map(|runtime| {
                let name = self
                    .project_path
                    .as_deref()
                    .and_then(Path::file_name)
                    .and_then(|name| name.to_str())
                    .unwrap_or("Project");
                format!("{name} · {} entities", runtime.runtime_world.units.len())
            })
            .unwrap_or_else(|| "renderer diagnostic".to_string());
        window.set_title(&format!("MiniForge wgpu · {content} · {api}"));
        window.set_visible(true);
        window.focus_window();
        self.window = Some(window);
        self.backend = Some(backend);
        Ok(())
    }

    fn load_project_textures(&mut self, backend: &mut WgpuBackend) -> Result<(), String> {
        let Some(runtime) = self.runtime.as_ref() else {
            return Ok(());
        };
        for camera in runtime_render_target_cameras(&runtime.runtime_world)? {
            backend
                .create_render_target_2d(camera.descriptor.clone())
                .map_err(|error| error.to_string())?;
            self.texture_ids.insert(
                camera.binding_key,
                RuntimeTexture2D {
                    texture_id: camera.texture_id,
                    width: camera.descriptor.width,
                    height: camera.descriptor.height,
                },
            );
        }
        let mut paths = runtime
            .runtime_world
            .units
            .iter()
            .flat_map(|entity| {
                [
                    entity_sprite_path(entity),
                    entity_normal_map_path(entity),
                    entity_ui_sprite_path(entity),
                ]
                .into_iter()
                .flatten()
            })
            .map(ToString::to_string)
            .collect::<std::collections::BTreeSet<_>>();
        paths.extend(scene_ui_sprite_paths(&runtime.ui_canvases));
        paths.extend(ui_document_sprite_paths(&runtime.ui_documents));
        for relative_path in paths {
            let path = if Path::new(&relative_path).is_absolute() {
                PathBuf::from(&relative_path)
            } else {
                runtime.project_path.join(&relative_path)
            };
            let Ok(image) = image::open(&path) else {
                continue;
            };
            let rgba = image.to_rgba8();
            let texture_id = 10 + self.texture_ids.len() as u64;
            backend
                .upload_texture_rgba8(texture_id, rgba.width(), rgba.height(), rgba.as_raw())
                .map_err(|error| error.to_string())?;
            self.texture_ids.insert(
                relative_path,
                RuntimeTexture2D {
                    texture_id,
                    width: rgba.width(),
                    height: rgba.height(),
                },
            );
        }
        Ok(())
    }

    fn draw(&mut self, event_loop: &ActiveEventLoop) -> Result<(), String> {
        let Some(window) = self.window.as_ref() else {
            return Ok(());
        };
        let Some(backend) = self.backend.as_mut() else {
            return Ok(());
        };
        let size = window.inner_size();
        let width = size.width.max(1) as f32;
        let height = size.height.max(1) as f32;
        let elapsed = self.started_at.elapsed().as_secs_f32();
        let now = Instant::now();
        let dt = now
            .duration_since(self.previous_frame_at)
            .as_secs_f64()
            .clamp(0.0, 0.05);
        self.previous_frame_at = now;

        if self.simulate_device_loss_frame == Some(self.frames)
            && backend.device_loss_recoveries == 0
        {
            backend
                .force_device_loss_for_testing()
                .map_err(|error| error.to_string())?;
        }
        if backend.device_loss_recoveries != self.observed_device_loss_recoveries {
            self.rendered_once_targets.clear();
            self.observed_device_loss_recoveries = backend.device_loss_recoveries;
        }
        backend.begin_frame().map_err(|error| error.to_string())?;
        if let Some(runtime) = self.runtime.as_mut() {
            let movement = (
                self.input_right as i32 as f64 - self.input_left as i32 as f64,
                self.input_down as i32 as f64 - self.input_up as i32 as f64,
            );
            runtime.set_character_input_for_tag(
                "Player",
                movement,
                false,
                false,
                self.input_run,
                false,
            );
            runtime.run_headless_once(dt);
            let target_cameras = runtime_render_target_cameras(&runtime.runtime_world)?;
            for camera in target_cameras {
                let should_render = match camera.update_mode {
                    RenderTargetUpdateMode2D::Always => true,
                    RenderTargetUpdateMode2D::Once => {
                        !self.rendered_once_targets.contains(&camera.texture_id)
                    }
                    RenderTargetUpdateMode2D::Manual => false,
                };
                if !should_render {
                    continue;
                }
                draw_engine_runtime_world_to_render_target_2d(
                    backend,
                    runtime,
                    &self.texture_ids,
                    camera.texture_id,
                    camera.view,
                )
                .map_err(|error| error.to_string())?;
                if camera.update_mode == RenderTargetUpdateMode2D::Once {
                    self.rendered_once_targets.insert(camera.texture_id);
                }
            }
            let post_processing_enabled = runtime
                .engine_config
                .data
                .pointer("/rendering/post_processing")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(true);
            if post_processing_enabled
                && let Some(command) = runtime_post_process_2d(&runtime.runtime_world, elapsed)
            {
                backend
                    .set_post_process_2d(command)
                    .map_err(|error| error.to_string())?;
            }
            self.last_scene_stats =
                draw_engine_runtime_scene_2d(backend, runtime, &self.texture_ids, width, height)
                    .map_err(|error| error.to_string())?;
        } else {
            draw_renderer_diagnostic(backend, width, height, elapsed)?;
        }
        backend.end_frame().map_err(|error| error.to_string())?;

        self.frames += 1;
        if self.autotest_frames.is_some_and(|target| {
            backend.submitted_frames >= target.max(1)
                || self.frames >= target.max(1).saturating_mul(120)
        }) {
            println!(
                "MINIFORGE_WGPU_SURFACE_{} frames={} presented={} skipped={} reconfigured={} surface_loss_recoveries={} device_loss_recoveries={} logical_draws={} gpu_draws={} color_binds={} normal_binds={} render_target_passes={} post_process_passes={} post_process_effects={} pipelines={} vertex_bytes={} particle_emitters={} particle_capacity={} particle_spawned={} particle_dispatches={} entities={} textures={} normal_mapped={} lit={} ui_documents={} retained_ui_widgets={} retained_ui_quads={} api={:?}",
                if backend.submitted_frames >= self.autotest_frames.unwrap_or(1).max(1) {
                    "OK"
                } else {
                    "OCCLUDED"
                },
                self.frames,
                backend.submitted_frames,
                backend.skipped_surface_frames,
                backend.surface_reconfigurations,
                backend.surface_loss_recoveries,
                backend.device_loss_recoveries,
                backend.last_frame_diagnostics().logical_draw_calls,
                backend.last_frame_diagnostics().gpu_draw_calls,
                backend.last_frame_diagnostics().texture_bind_changes,
                backend.last_frame_diagnostics().normal_texture_bind_changes,
                backend.last_frame_diagnostics().render_target_passes,
                backend.last_frame_diagnostics().post_process_passes,
                backend.last_frame_diagnostics().post_process_effects,
                backend.last_frame_diagnostics().pipeline_changes,
                backend.last_frame_diagnostics().vertex_bytes_uploaded,
                backend.last_frame_diagnostics().queued_particle_systems,
                backend.last_frame_diagnostics().gpu_particle_capacity,
                backend.last_frame_diagnostics().gpu_particle_spawned,
                backend.last_frame_diagnostics().particle_compute_dispatches,
                self.runtime
                    .as_ref()
                    .map(|runtime| runtime.runtime_world.units.len())
                    .unwrap_or(0),
                self.texture_ids.len(),
                self.last_scene_stats.normal_mapped_entities,
                self.last_scene_stats.lit_entities,
                self.runtime
                    .as_ref()
                    .map(|runtime| runtime.ui_documents.len())
                    .unwrap_or(0),
                self.last_scene_stats.retained_ui_widgets,
                self.last_scene_stats.retained_ui_quads,
                backend.caps.as_ref().map(|caps| caps.api)
            );
            event_loop.exit();
        } else {
            window.request_redraw();
        }
        Ok(())
    }

    fn handle_ui_click(&mut self, pointer: (f64, f64)) {
        let viewport = self
            .window
            .as_ref()
            .map(|window| window.inner_size())
            .map(|size| (size.width.max(1) as f32, size.height.max(1) as f32))
            .unwrap_or((INITIAL_WIDTH as f32, INITIAL_HEIGHT as f32));
        let pointer_f32 = (pointer.0 as f32, pointer.1 as f32);
        let Some(runtime) = self.runtime.as_mut() else {
            return;
        };
        let mut handled = false;
        for document in runtime
            .ui_documents
            .iter_mut()
            .rev()
            .filter(|document| document.input_enabled)
        {
            let layout_viewport = document.layout_viewport(viewport);
            let layout_pointer = document.screen_to_layout(viewport, pointer_f32);
            let events = self.ui_runtime.update_miniforge_canvas_interaction(
                &document.canvas,
                layout_viewport,
                Some(layout_pointer),
                true,
            );
            let clicked = events
                .into_iter()
                .find(|event| event.kind == miniforge::engine::ui_runtime::UiEventKind::Click);
            if let Some(event) = clicked {
                self.ui_runtime.activate_miniforge_widget(
                    &mut document.canvas,
                    &event.element_id,
                    layout_viewport,
                    layout_pointer,
                );
                handled = true;
                break;
            }
        }
        if !handled {
            self.ui_runtime.update_entity_interaction(
                &mut runtime.runtime_world.units,
                pointer,
                true,
            );
        }
    }

    fn handle_ui_wheel(&mut self, pointer: (f64, f64), wheel_lines: f64) {
        let viewport = self
            .window
            .as_ref()
            .map(|window| window.inner_size())
            .map(|size| (size.width.max(1) as f32, size.height.max(1) as f32))
            .unwrap_or((INITIAL_WIDTH as f32, INITIAL_HEIGHT as f32));
        let Some(runtime) = self.runtime.as_mut() else {
            return;
        };
        let handled = runtime
            .ui_documents
            .iter_mut()
            .rev()
            .filter(|document| document.input_enabled)
            .any(|document| {
                let layout_viewport = document.layout_viewport(viewport);
                let layout_pointer =
                    document.screen_to_layout(viewport, (pointer.0 as f32, pointer.1 as f32));
                self.ui_runtime
                    .scroll_miniforge_canvas_under_pointer(
                        &mut document.canvas,
                        layout_viewport,
                        layout_pointer,
                        wheel_lines as f32,
                    )
                    .is_some()
            });
        if !handled {
            self.ui_runtime.scroll_entity_under_pointer(
                &mut runtime.runtime_world.units,
                pointer,
                wheel_lines,
            );
        }
    }

    fn update_movement_key(&mut self, key: &Key, pressed: bool) {
        match key {
            Key::Named(NamedKey::ArrowLeft) => self.input_left = pressed,
            Key::Named(NamedKey::ArrowRight) => self.input_right = pressed,
            Key::Named(NamedKey::ArrowUp) => self.input_up = pressed,
            Key::Named(NamedKey::ArrowDown) => self.input_down = pressed,
            Key::Character(character) if character.eq_ignore_ascii_case("a") => {
                self.input_left = pressed;
            }
            Key::Character(character) if character.eq_ignore_ascii_case("d") => {
                self.input_right = pressed;
            }
            Key::Character(character) if character.eq_ignore_ascii_case("w") => {
                self.input_up = pressed;
            }
            Key::Character(character) if character.eq_ignore_ascii_case("s") => {
                self.input_down = pressed;
            }
            _ => {}
        }
    }
}

fn draw_renderer_diagnostic(
    backend: &mut WgpuBackend,
    width: f32,
    height: f32,
    elapsed: f32,
) -> Result<(), String> {
    for column in 0..12 {
        for row in 0..7 {
            backend
                .draw_sprite(SpriteDrawCommand {
                    entity_id: (row * 12 + column) as u64,
                    texture_id: 0,
                    x: column as f32 * width / 12.0,
                    y: row as f32 * height / 7.0,
                    width: width / 12.0 - 1.0,
                    height: height / 7.0 - 1.0,
                    rotation: 0.0,
                    color: if (row + column) % 2 == 0 {
                        [0.055, 0.075, 0.105, 1.0]
                    } else {
                        [0.04, 0.055, 0.08, 1.0]
                    },
                })
                .map_err(|error| error.to_string())?;
        }
    }
    let pulse = 0.85 + elapsed.sin() * 0.1;
    backend
        .draw_text(TextDrawCommand {
            text_id: 900,
            text: "MiniForge wgpu · alpha + additive + multiply + screen".to_string(),
            font_family: String::new(),
            x: 20.0,
            y: 18.0,
            width: (width - 40.0).max(1.0),
            height: 44.0,
            font_size: 22.0,
            line_height: 28.0,
            color: [230, 242, 255, 255],
            wrap: TextWrapMode::Word,
            clip_rect: None,
        })
        .map_err(|error| error.to_string())?;
    backend
        .draw_sprite(SpriteDrawCommand {
            entity_id: 1_000,
            texture_id: 1,
            x: width * 0.5 - 90.0,
            y: height * 0.5 - 90.0,
            width: 180.0,
            height: 180.0,
            rotation: elapsed * 0.35,
            color: [pulse, 1.0, pulse, 1.0],
        })
        .map_err(|error| error.to_string())?;
    backend
        .draw_sprite(SpriteDrawCommand {
            entity_id: 1_001,
            texture_id: 0,
            x: width * 0.5 - 8.0,
            y: height * 0.5 - 8.0,
            width: 16.0,
            height: 16.0,
            rotation: -elapsed,
            color: [1.0, 0.82, 0.28, 1.0],
        })
        .map_err(|error| error.to_string())?;
    for (index, blend_mode) in [
        SpriteBlendMode::Alpha,
        SpriteBlendMode::Additive,
        SpriteBlendMode::Multiply,
        SpriteBlendMode::Screen,
        SpriteBlendMode::PremultipliedAlpha,
    ]
    .into_iter()
    .enumerate()
    {
        backend
            .draw_sprite_with_options(
                SpriteDrawCommand {
                    entity_id: 1_100 + index as u64,
                    texture_id: 0,
                    x: width * 0.5 - 150.0 + index as f32 * 60.0,
                    y: height - 58.0,
                    width: 52.0,
                    height: 38.0,
                    rotation: 0.0,
                    color: [0.25 + index as f32 * 0.08, 0.72, 0.95, 0.7],
                },
                SpriteDrawOptions {
                    blend_mode,
                    ..SpriteDrawOptions::default()
                },
            )
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

impl ApplicationHandler for PreviewApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            if let Err(error) = self.create_renderer(event_loop) {
                eprintln!("MINIFORGE_WGPU_SURFACE_ERROR {error}");
                event_loop.exit();
            } else if let Some(window) = self.window.as_ref() {
                window.request_redraw();
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if self.window.as_ref().map(|window| window.id()) != Some(window_id) {
            return;
        }
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(backend) = self.backend.as_mut()
                    && let Err(error) = backend.resize(size.width, size.height)
                {
                    eprintln!("MINIFORGE_WGPU_SURFACE_ERROR {error}");
                    event_loop.exit();
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let pressed = event.state == ElementState::Pressed;
                if pressed && matches!(event.logical_key, Key::Named(NamedKey::Escape)) {
                    event_loop.exit();
                } else {
                    self.update_movement_key(&event.logical_key, pressed);
                }
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.input_run = modifiers.state().shift_key();
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_position = Some((position.x, position.y));
            }
            WindowEvent::CursorLeft { .. } => {
                self.cursor_position = None;
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                if let Some(pointer) = self.cursor_position {
                    self.handle_ui_click(pointer);
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let wheel_lines = match delta {
                    MouseScrollDelta::LineDelta(_, y) => f64::from(y),
                    MouseScrollDelta::PixelDelta(position) => position.y / 32.0,
                };
                if let Some(pointer) = self.cursor_position {
                    self.handle_ui_wheel(pointer, wheel_lines);
                }
            }
            WindowEvent::RedrawRequested => {
                if let Err(error) = self.draw(event_loop) {
                    eprintln!("MINIFORGE_WGPU_SURFACE_ERROR {error}");
                    event_loop.exit();
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut event_loop_builder = EventLoop::builder();
    #[cfg(target_os = "macos")]
    event_loop_builder
        .with_activation_policy(ActivationPolicy::Regular)
        .with_activate_ignoring_other_apps(true);
    let event_loop = event_loop_builder.build()?;
    let project_path = std::env::args_os().nth(1).map(PathBuf::from);
    let mut app = PreviewApp::new(project_path)?;
    event_loop.run_app(&mut app)?;
    Ok(())
}
