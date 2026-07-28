use std::collections::BTreeMap;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use miniforge::render::backend::{RenderBackend, SpriteDrawCommand, WgpuBackend};
use miniforge::render::runtime_scene_2d::{
    RuntimeTexture2D, draw_engine_runtime_scene_2d, entity_sprite_path,
};
use miniforge::runtime::engine_runtime::EngineRuntime;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, WindowEvent};
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
    previous_frame_at: Instant,
    input_left: bool,
    input_right: bool,
    input_up: bool,
    input_down: bool,
    input_run: bool,
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
            previous_frame_at: now,
            input_left: false,
            input_right: false,
            input_up: false,
            input_down: false,
            input_run: false,
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
        let paths = runtime
            .runtime_world
            .units
            .iter()
            .filter_map(entity_sprite_path)
            .map(ToString::to_string)
            .collect::<std::collections::BTreeSet<_>>();
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
                "MINIFORGE_WGPU_SURFACE_{} frames={} presented={} skipped={} reconfigured={} surface_loss_recoveries={} device_loss_recoveries={} logical_draws={} gpu_draws={} binds={} vertex_bytes={} entities={} textures={} api={:?}",
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
                backend.last_frame_diagnostics().vertex_bytes_uploaded,
                self.runtime
                    .as_ref()
                    .map(|runtime| runtime.runtime_world.units.len())
                    .unwrap_or(0),
                self.texture_ids.len(),
                backend.caps.as_ref().map(|caps| caps.api)
            );
            event_loop.exit();
        } else {
            window.request_redraw();
        }
        Ok(())
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
