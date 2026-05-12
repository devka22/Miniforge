use std::collections::{BTreeMap, BTreeSet};

use crate::entities::game_object::GameObject;

#[derive(Debug, Clone)]
pub struct PhysicsSystem {
    pub gravity: (f64, f64),
    pub solver_iterations: usize,
    pub active_pairs: BTreeMap<(u64, u64), PairType>,
    pub layer_matrix: BTreeMap<(String, String), bool>,
    pub stats: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairType {
    Collision,
    Trigger,
}

#[derive(Debug, Clone, Copy)]
struct Contact {
    normal: (f64, f64),
    depth: f64,
}

#[derive(Debug, Clone, Copy)]
struct Bounds {
    cx: f64,
    cy: f64,
    half_w: f64,
    half_h: f64,
}

impl Default for PhysicsSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl PhysicsSystem {
    pub fn new() -> Self {
        Self {
            gravity: (0.0, 18.0),
            solver_iterations: 2,
            active_pairs: BTreeMap::new(),
            layer_matrix: BTreeMap::new(),
            stats: BTreeMap::from([
                ("bodies".to_string(), 0),
                ("colliders".to_string(), 0),
                ("pairs".to_string(), 0),
                ("contacts".to_string(), 0),
            ]),
        }
    }

    pub fn set_layer_collision(&mut self, first_layer: &str, second_layer: &str, enabled: bool) {
        self.layer_matrix
            .insert(layer_key(first_layer, second_layer), enabled);
    }

    pub fn layer_collision_enabled(&self, first: &GameObject, second: &GameObject) -> bool {
        self.layer_matrix
            .get(&layer_key(&first.layer, &second.layer))
            .copied()
            .unwrap_or(true)
    }

    pub fn update_entities(&self, entities: &mut [GameObject], dt: f64, mode: &str) {
        let mut system = self.clone();
        system.update_entities_mut(entities, dt, mode);
    }

    pub fn update_entities_mut(&mut self, entities: &mut [GameObject], dt: f64, mode: &str) {
        if mode == "EDITOR" {
            self.stats = BTreeMap::from([
                ("bodies".to_string(), 0),
                ("colliders".to_string(), 0),
                ("pairs".to_string(), 0),
                ("contacts".to_string(), 0),
            ]);
            return;
        }

        let dt = dt.clamp(0.0, 0.05);
        self.integrate_bodies(entities, dt);

        let colliders: Vec<usize> = entities
            .iter()
            .enumerate()
            .filter_map(|(index, entity)| {
                if entity.enabled
                    && entity.visible
                    && entity
                        .get_component("Collider2D")
                        .is_some_and(|collider| collider.enabled)
                {
                    Some(index)
                } else {
                    None
                }
            })
            .collect();

        let mut current_pairs = BTreeMap::new();
        for _ in 0..self.solver_iterations.max(1) {
            for left_pos in 0..colliders.len() {
                for right_pos in (left_pos + 1)..colliders.len() {
                    let first_index = colliders[left_pos];
                    let second_index = colliders[right_pos];
                    let (first, second) = two_entities_mut(entities, first_index, second_index);
                    if !self.layer_collision_enabled(first, second) {
                        continue;
                    }
                    let Some(contact) = compute_contact(first, second) else {
                        continue;
                    };
                    let trigger = first
                        .get_component("Collider2D")
                        .is_some_and(|collider| collider.get_bool("is_trigger", false))
                        || second
                            .get_component("Collider2D")
                            .is_some_and(|collider| collider.get_bool("is_trigger", false));
                    let pair_type = if trigger {
                        PairType::Trigger
                    } else {
                        PairType::Collision
                    };
                    current_pairs.insert(pair_key(first.id, second.id), pair_type);
                    if !trigger {
                        resolve_contact(first, second, contact);
                    }
                }
            }
        }

        self.dispatch_pair_events(&current_pairs);
        let bodies = entities
            .iter()
            .filter(|entity| entity.get_component("Rigidbody2D").is_some())
            .count();
        self.stats = BTreeMap::from([
            ("bodies".to_string(), bodies),
            ("colliders".to_string(), colliders.len()),
            ("pairs".to_string(), current_pairs.len()),
            ("contacts".to_string(), current_pairs.len()),
        ]);
    }

    fn integrate_bodies(&self, entities: &mut [GameObject], dt: f64) {
        for entity in entities {
            let (
                mut vx,
                mut vy,
                mut angular_velocity,
                use_gravity,
                gravity_scale,
                drag,
                angular_drag,
                freeze_x,
                freeze_y,
                freeze_rotation,
            ) = {
                let Some(body) = entity.get_component("Rigidbody2D") else {
                    continue;
                };
                if !body.enabled || !body.is_dynamic_body() {
                    continue;
                }
                (
                    body.get_f64("velocity_x", 0.0),
                    body.get_f64("velocity_y", 0.0),
                    body.get_f64("angular_velocity", 0.0),
                    body.get_bool("use_gravity", true),
                    body.get_f64("gravity_scale", 1.0),
                    body.get_f64("drag", 0.05),
                    body.get_f64("angular_drag", 0.05),
                    body.get_bool("freeze_x", false),
                    body.get_bool("freeze_y", false),
                    body.get_bool("freeze_rotation", false),
                )
            };

            if use_gravity {
                vx += self.gravity.0 * gravity_scale * dt;
                vy += self.gravity.1 * gravity_scale * dt;
            }
            let damping = (1.0 - drag.max(0.0) * dt).max(0.0);
            vx *= damping;
            vy *= damping;

            if !freeze_x {
                entity.x += vx * dt;
            }
            if !freeze_y {
                entity.y += vy * dt;
            }
            if !freeze_rotation {
                let angular_damping = (1.0 - angular_drag.max(0.0) * dt).max(0.0);
                angular_velocity *= angular_damping;
                entity.rotation += angular_velocity * dt;
            }

            if let Some(body) = entity.get_component_mut("Rigidbody2D") {
                body.set_f64("velocity_x", vx);
                body.set_f64("velocity_y", vy);
                body.set_f64("angular_velocity", angular_velocity);
            }
            entity.sync_to_components();
        }
    }

    fn dispatch_pair_events(&mut self, current_pairs: &BTreeMap<(u64, u64), PairType>) {
        let previous: BTreeSet<(u64, u64)> = self.active_pairs.keys().copied().collect();
        let current: BTreeSet<(u64, u64)> = current_pairs.keys().copied().collect();

        let entered = current.difference(&previous).count();
        let exited = previous.difference(&current).count();
        let stayed = current.intersection(&previous).count();
        self.stats.insert("entered".to_string(), entered);
        self.stats.insert("exited".to_string(), exited);
        self.stats.insert("stayed".to_string(), stayed);
        self.active_pairs = current_pairs.clone();
    }
}

fn compute_contact(first: &GameObject, second: &GameObject) -> Option<Contact> {
    let first_bounds = collider_bounds(first)?;
    let second_bounds = collider_bounds(second)?;
    let dx = first_bounds.cx - second_bounds.cx;
    let dy = first_bounds.cy - second_bounds.cy;
    let overlap_x = first_bounds.half_w + second_bounds.half_w - dx.abs();
    let overlap_y = first_bounds.half_h + second_bounds.half_h - dy.abs();
    if overlap_x <= 0.0 || overlap_y <= 0.0 {
        return None;
    }
    if overlap_x < overlap_y {
        Some(Contact {
            normal: (if dx >= 0.0 { 1.0 } else { -1.0 }, 0.0),
            depth: overlap_x,
        })
    } else {
        Some(Contact {
            normal: (0.0, if dy >= 0.0 { 1.0 } else { -1.0 }),
            depth: overlap_y,
        })
    }
}

fn collider_bounds(entity: &GameObject) -> Option<Bounds> {
    let collider = entity.get_component("Collider2D")?;
    let mut width = collider.get_f64("width", 1.0);
    let mut height = collider.get_f64("height", 1.0);
    if collider.get_string("shape", "rect") == "circle" {
        let radius = collider.get_f64("radius", 0.5);
        width = radius * 2.0;
        height = radius * 2.0;
    }
    Some(Bounds {
        cx: entity.x + collider.get_f64("offset_x", 0.0),
        cy: entity.y + collider.get_f64("offset_y", 0.0),
        half_w: (width * 0.5).max(0.05),
        half_h: (height * 0.5).max(0.05),
    })
}

fn resolve_contact(first: &mut GameObject, second: &mut GameObject, contact: Contact) {
    let first_dynamic = first
        .get_component("Rigidbody2D")
        .is_some_and(|body| body.is_dynamic_body());
    let second_dynamic = second
        .get_component("Rigidbody2D")
        .is_some_and(|body| body.is_dynamic_body());
    if !first_dynamic && !second_dynamic {
        return;
    }

    let depth = contact.depth + 0.001;
    let (nx, ny) = contact.normal;
    let first_share = if first_dynamic && second_dynamic {
        0.5
    } else {
        1.0
    };
    let second_share = if first_dynamic && second_dynamic {
        0.5
    } else {
        1.0
    };

    if first_dynamic {
        first.x += nx * depth * first_share;
        first.y += ny * depth * first_share;
        apply_collision_velocity(first, nx, ny);
        first.sync_to_components();
    }
    if second_dynamic {
        second.x -= nx * depth * second_share;
        second.y -= ny * depth * second_share;
        apply_collision_velocity(second, -nx, -ny);
        second.sync_to_components();
    }
}

fn apply_collision_velocity(entity: &mut GameObject, nx: f64, ny: f64) {
    let Some(body) = entity.get_component_mut("Rigidbody2D") else {
        return;
    };
    let mut vx = body.get_f64("velocity_x", 0.0);
    let mut vy = body.get_f64("velocity_y", 0.0);
    let velocity_dot = vx * nx + vy * ny;
    if velocity_dot < 0.0 {
        let bounciness = body.get_f64("bounciness", 0.0);
        vx -= (1.0 + bounciness) * velocity_dot * nx;
        vy -= (1.0 + bounciness) * velocity_dot * ny;
    }

    let friction = body.get_f64("friction", 0.25).clamp(0.0, 1.0);
    if nx.abs() > 0.0 {
        vy *= 1.0 - friction;
    } else {
        vx *= 1.0 - friction;
    }
    if (vx * vx + vy * vy).sqrt() < 0.001 {
        vx = 0.0;
        vy = 0.0;
    }
    body.set_f64("velocity_x", vx);
    body.set_f64("velocity_y", vy);
}

fn two_entities_mut(
    entities: &mut [GameObject],
    first_index: usize,
    second_index: usize,
) -> (&mut GameObject, &mut GameObject) {
    debug_assert!(first_index != second_index);
    if first_index < second_index {
        let (left, right) = entities.split_at_mut(second_index);
        (&mut left[first_index], &mut right[0])
    } else {
        let (left, right) = entities.split_at_mut(first_index);
        (&mut right[0], &mut left[second_index])
    }
}

fn layer_key(first: &str, second: &str) -> (String, String) {
    if first <= second {
        (first.to_string(), second.to_string())
    } else {
        (second.to_string(), first.to_string())
    }
}

fn pair_key(first: u64, second: u64) -> (u64, u64) {
    if first <= second {
        (first, second)
    } else {
        (second, first)
    }
}
