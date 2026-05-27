use rapier2d::prelude::{ActiveEvents, ColliderBuilder, Real, Vector};

use crate::entities::game_object::GameObject;

#[derive(Debug, Clone, PartialEq)]
pub struct RapierSceneReport {
    pub backend: String,
    pub gravity: (f32, f32),
    pub colliders: usize,
    pub dynamic_bodies: usize,
    pub static_bodies: usize,
    pub kinematic_bodies: usize,
    pub sensors: usize,
    pub invalid_colliders: Vec<String>,
    pub broadphase_aabb_area: f32,
}

impl RapierSceneReport {
    pub fn status_line(&self) -> String {
        format!(
            "{} ready | bodies D/S/K {}/{}/{} | colliders {} | sensors {} | invalid {}",
            self.backend,
            self.dynamic_bodies,
            self.static_bodies,
            self.kinematic_bodies,
            self.colliders,
            self.sensors,
            self.invalid_colliders.len()
        )
    }
}

#[derive(Debug, Clone, Default)]
pub struct RapierPhysicsBridge;

impl RapierPhysicsBridge {
    pub fn inspect_scene(entities: &[GameObject], gravity: (f64, f64)) -> RapierSceneReport {
        let mut report = RapierSceneReport {
            backend: "Rapier2D bridge".to_string(),
            gravity: (gravity.0 as f32, gravity.1 as f32),
            colliders: 0,
            dynamic_bodies: 0,
            static_bodies: 0,
            kinematic_bodies: 0,
            sensors: 0,
            invalid_colliders: Vec::new(),
            broadphase_aabb_area: 0.0,
        };

        for entity in entities {
            if let Some(body) = entity.get_component("Rigidbody2D") {
                match body.get_string("body_type", "dynamic").as_str() {
                    "static" => report.static_bodies += 1,
                    "kinematic" => report.kinematic_bodies += 1,
                    _ => report.dynamic_bodies += 1,
                }
            }

            let Some(collider) = entity.get_component("Collider2D") else {
                continue;
            };
            if !collider.enabled {
                continue;
            }
            let Some(builder) = collider_builder_for(entity) else {
                report.invalid_colliders.push(entity.name.clone());
                continue;
            };
            let collider = builder
                .translation(Vector::new(entity.x as Real, entity.y as Real))
                .sensor(collider.get_bool("is_trigger", false))
                .active_events(ActiveEvents::COLLISION_EVENTS)
                .friction(collider.get_f64("friction", 0.25).max(0.0) as Real)
                .restitution(collider.get_f64("bounciness", 0.0).clamp(0.0, 1.0) as Real)
                .build();
            let aabb = collider.compute_aabb();
            let extents = aabb.extents();
            report.broadphase_aabb_area += extents.x * extents.y;
            report.colliders += 1;
            if collider.is_sensor() {
                report.sensors += 1;
            }
        }

        report
    }
}

fn collider_builder_for(entity: &GameObject) -> Option<ColliderBuilder> {
    let collider = entity.get_component("Collider2D")?;
    let shape = collider.get_string("shape", "rect").to_lowercase();
    match shape.as_str() {
        "circle" => Some(ColliderBuilder::ball(
            collider.get_f64("radius", 0.5).max(0.001) as Real,
        )),
        "polygon" => {
            let points = collider
                .get("points")
                .and_then(|value| value.as_array())
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|item| {
                            let pair = item.as_array()?;
                            Some(Vector::new(
                                pair.first()?.as_f64()? as Real,
                                pair.get(1)?.as_f64()? as Real,
                            ))
                        })
                        .collect::<Vec<Vector>>()
                })
                .unwrap_or_default();
            ColliderBuilder::convex_hull(&points).or_else(|| Some(default_box(entity, collider)))
        }
        _ => Some(default_box(entity, collider)),
    }
}

fn default_box(
    entity: &GameObject,
    collider: &crate::engine::component::Component,
) -> ColliderBuilder {
    ColliderBuilder::cuboid(
        (collider.get_f64("width", entity.width).max(0.001) * entity.scale_x * 0.5) as Real,
        (collider.get_f64("height", entity.height).max(0.001) * entity.scale_y * 0.5) as Real,
    )
}
