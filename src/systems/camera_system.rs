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
        if let Some(entity) = entities
            .iter()
            .find(|entity| entity.get_component("CameraFollow").is_some())
        {
            camera.x = entity.x;
            camera.y = entity.y;
            camera.clamp_to_bounds();
        }
    }

    pub fn pan(camera: &mut Camera, direction: (f64, f64), dt: f64) {
        let speed = Self::default().speed;
        camera.move_by(direction.0 * speed * dt, direction.1 * speed * dt);
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
        camera.set_zoom(camera.zoom + amount * 0.8 * dt);
    }
}
