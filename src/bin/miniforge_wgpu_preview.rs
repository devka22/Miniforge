use std::error::Error;
use std::sync::Arc;
use std::time::Instant;

use miniforge::render::backend::{RenderBackend, SpriteDrawCommand, WgpuBackend};
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
}

impl Default for PreviewApp {
    fn default() -> Self {
        Self {
            window: None,
            backend: None,
            started_at: Instant::now(),
            frames: 0,
            autotest_frames: std::env::var("MINIFORGE_WGPU_AUTOTEST_FRAMES")
                .ok()
                .and_then(|value| value.parse().ok()),
        }
    }
}

impl PreviewApp {
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

        let api = backend
            .caps
            .as_ref()
            .map(|caps| format!("{:?} · {}", caps.api, caps.device_name))
            .unwrap_or_else(|| "wgpu".to_string());
        window.set_title(&format!("MiniForge wgpu Surface Preview · {api}"));
        window.set_visible(true);
        window.focus_window();
        self.window = Some(window);
        self.backend = Some(backend);
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

        backend.begin_frame().map_err(|error| error.to_string())?;

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
        backend.end_frame().map_err(|error| error.to_string())?;

        self.frames += 1;
        if self.autotest_frames.is_some_and(|target| {
            backend.submitted_frames >= target.max(1)
                || self.frames >= target.max(1).saturating_mul(120)
        }) {
            println!(
                "MINIFORGE_WGPU_SURFACE_{} frames={} presented={} skipped={} reconfigured={} api={:?}",
                if backend.submitted_frames >= self.autotest_frames.unwrap_or(1).max(1) {
                    "OK"
                } else {
                    "OCCLUDED"
                },
                self.frames,
                backend.submitted_frames,
                backend.skipped_surface_frames,
                backend.surface_reconfigurations,
                backend.caps.as_ref().map(|caps| caps.api)
            );
            event_loop.exit();
        } else {
            window.request_redraw();
        }
        Ok(())
    }
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
                if let Some(backend) = self.backend.as_mut() {
                    if let Err(error) = backend.resize(size.width, size.height) {
                        eprintln!("MINIFORGE_WGPU_SURFACE_ERROR {error}");
                        event_loop.exit();
                    }
                }
            }
            WindowEvent::KeyboardInput { event, .. }
                if event.state == ElementState::Pressed
                    && matches!(event.logical_key, Key::Named(NamedKey::Escape)) =>
            {
                event_loop.exit();
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
    let mut app = PreviewApp::default();
    event_loop.run_app(&mut app)?;
    Ok(())
}
