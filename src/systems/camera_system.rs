use crate::engine::camera::Camera;
use crate::entities::game_object::GameObject;

#[derive(Debug, Clone)]
pub struct CameraSystem {
    pub speed: f64,
    pub edge_margin: f64,
}

impl Default for CameraSystem {
    fn default() -> Self {
        Self {
            speed: 520.0,
            edge_margin: 18.0,
        }
    }
}

impl CameraSystem {
    pub fn follow(camera: &mut Camera, entities: &[GameObject]) {
        if let Some(entity) = entities.iter().find(|entity| {
            entity.enabled
                && entity.active
                && entity
                    .get_component("CameraFollow")
                    .is_some_and(|component| component.enabled)
        }) {
            camera.x = entity.x;
            camera.y = entity.y;
            camera.clamp_to_bounds();
        }
    }

    pub fn pan(camera: &mut Camera, direction: (f64, f64), dt: f64) {
        let speed = Self::default().speed;
        pan_at_speed(camera, direction, dt, speed);
    }

    pub fn pan_configured(&self, camera: &mut Camera, direction: (f64, f64), dt: f64) {
        pan_at_speed(camera, direction, dt, self.speed);
    }

    pub fn edge_pan(
        camera: &mut Camera,
        mouse: (f64, f64),
        viewport_size: (f64, f64),
        dt: f64,
    ) -> (f64, f64) {
        let system = Self::default();
        let mut dx = 0.0;
        let mut dy = 0.0;
        if mouse.0 < system.edge_margin {
            dx -= 1.0;
        }
        if mouse.0 > viewport_size.0 - system.edge_margin {
            dx += 1.0;
        }
        if mouse.1 < system.edge_margin {
            dy -= 1.0;
        }
        if mouse.1 > viewport_size.1 - system.edge_margin {
            dy += 1.0;
        }
        Self::pan(camera, (dx, dy), dt);
        (dx, dy)
    }

    pub fn zoom(camera: &mut Camera, amount: f64, dt: f64) {
        let dt = finite_dt(dt);
        camera.set_zoom(camera.zoom + amount * 0.8 * dt);
    }

    /// Zoom while keeping the world position below the cursor stable.
    pub fn zoom_towards(camera: &mut Camera, cursor_viewport: (f64, f64), amount: f64, dt: f64) {
        let old_zoom = camera.zoom.max(0.1);
        let world_before = (
            camera.x + cursor_viewport.0 / old_zoom,
            camera.y + cursor_viewport.1 / old_zoom,
        );
        Self::zoom(camera, amount, dt);
        camera.x = world_before.0 - cursor_viewport.0 / camera.zoom.max(0.1);
        camera.y = world_before.1 - cursor_viewport.1 / camera.zoom.max(0.1);
        camera.clamp_to_bounds();
    }
}

fn pan_at_speed(camera: &mut Camera, direction: (f64, f64), dt: f64, speed: f64) {
    let length = direction.0.hypot(direction.1);
    let direction = if length > 1.0 {
        (direction.0 / length, direction.1 / length)
    } else {
        direction
    };
    camera.move_by(
        direction.0 * speed.max(0.0) * finite_dt(dt),
        direction.1 * speed.max(0.0) * finite_dt(dt),
    );
}

fn finite_dt(dt: f64) -> f64 {
    if dt.is_finite() {
        dt.clamp(0.0, 0.1)
    } else {
        0.0
    }
}
