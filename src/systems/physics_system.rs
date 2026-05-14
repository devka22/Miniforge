use std::collections::{BTreeMap, BTreeSet};

use crate::entities::game_object::GameObject;

type Vec2 = (f64, f64);
type RayShapeHit = (f64, Vec2, Vec2);

#[derive(Debug, Clone)]
pub struct PhysicsSystem {
    pub gravity: (f64, f64),
    pub solver_iterations: usize,
    pub active_pairs: BTreeMap<(u64, u64), PairType>,
    pub active_contacts: BTreeMap<(u64, u64), Contact>,
    pub layer_matrix: BTreeMap<(String, String), bool>,
    pub events: Vec<PhysicsEvent>,
    pub stats: BTreeMap<String, usize>,
    active_pair_names: BTreeMap<(u64, u64), (String, String)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairType {
    Collision,
    Trigger,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhysicsEventPhase {
    Enter,
    Stay,
    Exit,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PhysicsEvent {
    pub first_id: u64,
    pub second_id: u64,
    pub first_name: String,
    pub second_name: String,
    pub pair_type: PairType,
    pub phase: PhysicsEventPhase,
    pub normal: (f64, f64),
    pub depth: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Contact {
    pub normal: (f64, f64),
    pub depth: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RaycastHit {
    pub entity_id: u64,
    pub entity_name: String,
    pub point: (f64, f64),
    pub normal: (f64, f64),
    pub distance: f64,
    pub layer: String,
    pub is_trigger: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BodyKind {
    Dynamic,
    Static,
    Kinematic,
}

#[derive(Debug, Clone)]
enum ColliderShape {
    Circle {
        center: (f64, f64),
        radius: f64,
    },
    Polygon {
        center: (f64, f64),
        points: Vec<(f64, f64)>,
    },
}

impl ColliderShape {
    fn center(&self) -> (f64, f64) {
        match self {
            Self::Circle { center, .. } | Self::Polygon { center, .. } => *center,
        }
    }
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
            active_contacts: BTreeMap::new(),
            layer_matrix: BTreeMap::new(),
            events: Vec::new(),
            stats: default_stats(),
            active_pair_names: BTreeMap::new(),
        }
    }

    pub fn set_gravity(&mut self, x: f64, y: f64) {
        self.gravity = (x, y);
    }

    pub fn set_layer_collision(&mut self, first_layer: &str, second_layer: &str, enabled: bool) {
        self.layer_matrix
            .insert(layer_key(first_layer, second_layer), enabled);
    }

    pub fn layer_collision_enabled(&self, first: &GameObject, second: &GameObject) -> bool {
        if !self
            .layer_matrix
            .get(&layer_key(
                &collision_layer(first),
                &collision_layer(second),
            ))
            .copied()
            .unwrap_or(true)
        {
            return false;
        }
        collider_mask_allows(first, &collision_layer(second))
            && collider_mask_allows(second, &collision_layer(first))
    }

    pub fn update_entities(&self, entities: &mut [GameObject], dt: f64, mode: &str) {
        let mut system = self.clone();
        system.update_entities_mut(entities, dt, mode);
    }

    pub fn update_entities_mut(&mut self, entities: &mut [GameObject], dt: f64, mode: &str) {
        self.events.clear();
        if mode == "EDITOR" {
            self.stats = default_stats();
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
        let mut current_contacts = BTreeMap::new();
        let mut current_names = BTreeMap::new();
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
                    let trigger = collider_is_trigger(first) || collider_is_trigger(second);
                    let pair_type = if trigger {
                        PairType::Trigger
                    } else {
                        PairType::Collision
                    };
                    let key = pair_key(first.id, second.id);
                    current_pairs.insert(key, pair_type);
                    current_contacts.insert(key, contact);
                    current_names.insert(key, ordered_pair_names(first, second));
                    if !trigger {
                        resolve_contact(first, second, contact);
                    }
                }
            }
        }

        self.dispatch_pair_events(&current_pairs, &current_contacts, &current_names);
        let bodies = entities
            .iter()
            .filter(|entity| entity.get_component("Rigidbody2D").is_some())
            .count();
        let triggers = current_pairs
            .values()
            .filter(|pair| **pair == PairType::Trigger)
            .count();
        let collisions = current_pairs.len().saturating_sub(triggers);
        self.stats.extend(BTreeMap::from([
            ("bodies".to_string(), bodies),
            ("colliders".to_string(), colliders.len()),
            ("pairs".to_string(), current_pairs.len()),
            ("contacts".to_string(), current_pairs.len()),
            ("triggers".to_string(), triggers),
            ("collisions".to_string(), collisions),
        ]));
    }

    pub fn drain_events(&mut self) -> Vec<PhysicsEvent> {
        std::mem::take(&mut self.events)
    }

    pub fn raycast(
        &self,
        entities: &[GameObject],
        origin: (f64, f64),
        direction: (f64, f64),
        max_distance: f64,
    ) -> Option<RaycastHit> {
        self.raycast_filtered(entities, origin, direction, max_distance, false, None)
    }

    pub fn raycast_filtered(
        &self,
        entities: &[GameObject],
        origin: (f64, f64),
        direction: (f64, f64),
        max_distance: f64,
        include_triggers: bool,
        layers: Option<&[String]>,
    ) -> Option<RaycastHit> {
        self.raycast_all_filtered(
            entities,
            origin,
            direction,
            max_distance,
            include_triggers,
            layers,
        )
        .into_iter()
        .next()
    }

    pub fn raycast_all_filtered(
        &self,
        entities: &[GameObject],
        origin: (f64, f64),
        direction: (f64, f64),
        max_distance: f64,
        include_triggers: bool,
        layers: Option<&[String]>,
    ) -> Vec<RaycastHit> {
        let Some(dir) = normalize(direction) else {
            return Vec::new();
        };
        let max_distance = max_distance.max(0.0);
        let mut hits = Vec::new();
        for entity in entities {
            if !entity.enabled || !entity.visible {
                continue;
            }
            let Some(collider) = entity.get_component("Collider2D") else {
                continue;
            };
            if !collider.enabled {
                continue;
            }
            let is_trigger = collider.get_bool("is_trigger", false);
            if is_trigger && !include_triggers {
                continue;
            }
            let layer = collision_layer(entity);
            if layers.is_some_and(|items| !items.iter().any(|item| item == &layer)) {
                continue;
            }
            let Some(shape) = collider_shape(entity) else {
                continue;
            };
            if let Some((distance, point, normal)) =
                ray_shape_hit(origin, dir, max_distance, &shape)
            {
                hits.push(RaycastHit {
                    entity_id: entity.id,
                    entity_name: entity.name.clone(),
                    point,
                    normal,
                    distance,
                    layer,
                    is_trigger,
                });
            }
        }
        hits.sort_by(|a, b| {
            a.distance
                .partial_cmp(&b.distance)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        hits
    }

    fn integrate_bodies(&self, entities: &mut [GameObject], dt: f64) {
        for entity in entities {
            let (
                body_kind,
                mut vx,
                mut vy,
                mut angular_velocity,
                use_gravity,
                gravity_scale,
                body_gravity_x,
                body_gravity_y,
                drag,
                angular_drag,
                freeze_x,
                freeze_y,
                freeze_rotation,
            ) = {
                let Some(body) = entity.get_component("Rigidbody2D") else {
                    continue;
                };
                if !body.enabled || body.get_bool("sleeping", false) {
                    continue;
                }
                (
                    body_kind(body),
                    body.get_f64("velocity_x", 0.0),
                    body.get_f64("velocity_y", 0.0),
                    body.get_f64("angular_velocity", 0.0),
                    body.get_bool("use_gravity", true),
                    body.get_f64("gravity_scale", 1.0),
                    body.get("gravity_x").and_then(serde_json::Value::as_f64),
                    body.get("gravity_y").and_then(serde_json::Value::as_f64),
                    body.get_f64("drag", 0.05),
                    body.get_f64("angular_drag", 0.05),
                    body.get_bool("freeze_x", false),
                    body.get_bool("freeze_y", false),
                    body.get_bool("freeze_rotation", false),
                )
            };

            if body_kind == BodyKind::Static {
                continue;
            }
            if body_kind == BodyKind::Dynamic && use_gravity {
                vx += body_gravity_x.unwrap_or(self.gravity.0) * gravity_scale * dt;
                vy += body_gravity_y.unwrap_or(self.gravity.1) * gravity_scale * dt;
            }
            if body_kind == BodyKind::Dynamic {
                let damping = (1.0 - drag.max(0.0) * dt).max(0.0);
                vx *= damping;
                vy *= damping;
            }

            if !freeze_x {
                entity.x += vx * dt;
            }
            if !freeze_y {
                entity.y += vy * dt;
            }
            if !freeze_rotation {
                let angular_damping = if body_kind == BodyKind::Dynamic {
                    (1.0 - angular_drag.max(0.0) * dt).max(0.0)
                } else {
                    1.0
                };
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

    fn dispatch_pair_events(
        &mut self,
        current_pairs: &BTreeMap<(u64, u64), PairType>,
        current_contacts: &BTreeMap<(u64, u64), Contact>,
        current_names: &BTreeMap<(u64, u64), (String, String)>,
    ) {
        let previous: BTreeSet<(u64, u64)> = self.active_pairs.keys().copied().collect();
        let current: BTreeSet<(u64, u64)> = current_pairs.keys().copied().collect();

        let mut entered = 0;
        let mut exited = 0;
        let mut stayed = 0;
        for key in &current {
            let phase = if previous.contains(key) {
                stayed += 1;
                PhysicsEventPhase::Stay
            } else {
                entered += 1;
                PhysicsEventPhase::Enter
            };
            let contact = current_contacts.get(key).copied().unwrap_or(Contact {
                normal: (0.0, 0.0),
                depth: 0.0,
            });
            let names = current_names
                .get(key)
                .cloned()
                .unwrap_or_else(|| (String::new(), String::new()));
            self.events.push(PhysicsEvent {
                first_id: key.0,
                second_id: key.1,
                first_name: names.0,
                second_name: names.1,
                pair_type: current_pairs[key],
                phase,
                normal: contact.normal,
                depth: contact.depth,
            });
        }
        for key in previous.difference(&current) {
            exited += 1;
            let names = self
                .active_pair_names
                .get(key)
                .cloned()
                .unwrap_or_else(|| (String::new(), String::new()));
            let contact = self.active_contacts.get(key).copied().unwrap_or(Contact {
                normal: (0.0, 0.0),
                depth: 0.0,
            });
            self.events.push(PhysicsEvent {
                first_id: key.0,
                second_id: key.1,
                first_name: names.0,
                second_name: names.1,
                pair_type: self.active_pairs[key],
                phase: PhysicsEventPhase::Exit,
                normal: contact.normal,
                depth: contact.depth,
            });
        }
        self.stats.insert("entered".to_string(), entered);
        self.stats.insert("exited".to_string(), exited);
        self.stats.insert("stayed".to_string(), stayed);
        self.active_pairs = current_pairs.clone();
        self.active_contacts = current_contacts.clone();
        self.active_pair_names = current_names.clone();
    }
}

fn default_stats() -> BTreeMap<String, usize> {
    BTreeMap::from([
        ("bodies".to_string(), 0),
        ("colliders".to_string(), 0),
        ("pairs".to_string(), 0),
        ("contacts".to_string(), 0),
        ("triggers".to_string(), 0),
        ("collisions".to_string(), 0),
        ("entered".to_string(), 0),
        ("exited".to_string(), 0),
        ("stayed".to_string(), 0),
    ])
}

fn compute_contact(first: &GameObject, second: &GameObject) -> Option<Contact> {
    let first_shape = collider_shape(first)?;
    let second_shape = collider_shape(second)?;
    shape_contact(&first_shape, &second_shape)
}

fn collider_shape(entity: &GameObject) -> Option<ColliderShape> {
    let collider = entity.get_component("Collider2D")?;
    let center = (
        entity.x + collider.get_f64("offset_x", 0.0),
        entity.y + collider.get_f64("offset_y", 0.0),
    );
    let shape = collider.get_string("shape", "rect").to_lowercase();
    if shape == "circle" {
        return Some(ColliderShape::Circle {
            center,
            radius: collider.get_f64("radius", 0.5).max(0.001),
        });
    }

    let points = if shape == "polygon" {
        parse_polygon_points(collider.get("points")).unwrap_or_else(|| {
            rect_points(
                collider.get_f64("width", entity.width).max(0.001),
                collider.get_f64("height", entity.height).max(0.001),
            )
        })
    } else {
        rect_points(
            collider.get_f64("width", entity.width).max(0.001),
            collider.get_f64("height", entity.height).max(0.001),
        )
    };
    let rotation = entity.rotation.to_radians();
    Some(ColliderShape::Polygon {
        center,
        points: points
            .into_iter()
            .map(|point| {
                let scaled = (point.0 * entity.scale_x, point.1 * entity.scale_y);
                let rotated = rotate_point(scaled, rotation);
                (center.0 + rotated.0, center.1 + rotated.1)
            })
            .collect(),
    })
}

fn shape_contact(first: &ColliderShape, second: &ColliderShape) -> Option<Contact> {
    match (first, second) {
        (
            ColliderShape::Circle {
                center: first_center,
                radius: first_radius,
            },
            ColliderShape::Circle {
                center: second_center,
                radius: second_radius,
            },
        ) => circle_circle_contact(*first_center, *first_radius, *second_center, *second_radius),
        (ColliderShape::Polygon { points, .. }, ColliderShape::Polygon { points: other, .. }) => {
            sat_contact(first.center(), points, second.center(), other, &[])
        }
        (ColliderShape::Polygon { points, .. }, ColliderShape::Circle { center, radius }) => {
            let extra = closest_vertex_axis(points, *center)
                .into_iter()
                .collect::<Vec<_>>();
            sat_contact(
                first.center(),
                points,
                *center,
                &circle_proxy_points(*center, *radius),
                &extra,
            )
        }
        (ColliderShape::Circle { center, radius }, ColliderShape::Polygon { points, .. }) => {
            let extra = closest_vertex_axis(points, *center)
                .into_iter()
                .collect::<Vec<_>>();
            sat_contact(
                *center,
                &circle_proxy_points(*center, *radius),
                second.center(),
                points,
                &extra,
            )
        }
    }
}

fn circle_circle_contact(
    first_center: (f64, f64),
    first_radius: f64,
    second_center: (f64, f64),
    second_radius: f64,
) -> Option<Contact> {
    let delta = (
        first_center.0 - second_center.0,
        first_center.1 - second_center.1,
    );
    let distance = length(delta);
    let depth = first_radius + second_radius - distance;
    if depth <= 0.0 {
        return None;
    }
    Some(Contact {
        normal: if distance <= f64::EPSILON {
            (1.0, 0.0)
        } else {
            (delta.0 / distance, delta.1 / distance)
        },
        depth,
    })
}

fn sat_contact(
    first_center: (f64, f64),
    first_points: &[(f64, f64)],
    second_center: (f64, f64),
    second_points: &[(f64, f64)],
    extra_axes: &[(f64, f64)],
) -> Option<Contact> {
    let mut axes = polygon_axes(first_points);
    axes.extend(polygon_axes(second_points));
    axes.extend(extra_axes.iter().copied());
    if axes.is_empty() {
        return None;
    }

    let mut min_depth = f64::MAX;
    let mut best_axis = (1.0, 0.0);
    for axis in axes.into_iter().filter_map(normalize) {
        let first_projection = project_points(first_points, axis);
        let second_projection = project_points(second_points, axis);
        let overlap = projection_overlap(first_projection, second_projection);
        if overlap <= 0.0 {
            return None;
        }
        if overlap < min_depth {
            min_depth = overlap;
            best_axis = axis;
        }
    }

    let center_delta = (
        first_center.0 - second_center.0,
        first_center.1 - second_center.1,
    );
    if dot(center_delta, best_axis) < 0.0 {
        best_axis = (-best_axis.0, -best_axis.1);
    }
    Some(Contact {
        normal: best_axis,
        depth: min_depth,
    })
}

fn resolve_contact(first: &mut GameObject, second: &mut GameObject, contact: Contact) {
    let first_kind = rigidbody_kind(first);
    let second_kind = rigidbody_kind(second);
    let first_inv_mass = inverse_mass(first, first_kind);
    let second_inv_mass = inverse_mass(second, second_kind);
    let total_inv_mass = first_inv_mass + second_inv_mass;
    if total_inv_mass <= f64::EPSILON {
        return;
    }

    let depth = contact.depth + 0.001;
    let (nx, ny) = contact.normal;
    if first_kind == BodyKind::Dynamic {
        let share = first_inv_mass / total_inv_mass;
        first.x += nx * depth * share;
        first.y += ny * depth * share;
        apply_collision_velocity(first, nx, ny);
        first.sync_to_components();
    }
    if second_kind == BodyKind::Dynamic {
        let share = second_inv_mass / total_inv_mass;
        second.x -= nx * depth * share;
        second.y -= ny * depth * share;
        apply_collision_velocity(second, -nx, -ny);
        second.sync_to_components();
    }
}

fn apply_collision_velocity(entity: &mut GameObject, nx: f64, ny: f64) {
    let Some(body) = entity.get_component_mut("Rigidbody2D") else {
        return;
    };
    if body_kind(body) != BodyKind::Dynamic {
        return;
    }
    let mut vx = body.get_f64("velocity_x", 0.0);
    let mut vy = body.get_f64("velocity_y", 0.0);
    let velocity_dot = vx * nx + vy * ny;
    if velocity_dot < 0.0 {
        let bounciness = body.get_f64("bounciness", 0.0).clamp(0.0, 1.0);
        vx -= (1.0 + bounciness) * velocity_dot * nx;
        vy -= (1.0 + bounciness) * velocity_dot * ny;
    }

    let friction = body.get_f64("friction", 0.25).clamp(0.0, 1.0);
    let tangent_velocity = (vx - (vx * nx + vy * ny) * nx, vy - (vx * nx + vy * ny) * ny);
    vx -= tangent_velocity.0 * friction;
    vy -= tangent_velocity.1 * friction;
    if (vx * vx + vy * vy).sqrt() < 0.001 {
        vx = 0.0;
        vy = 0.0;
    }
    body.set_f64("velocity_x", vx);
    body.set_f64("velocity_y", vy);
}

fn ray_shape_hit(
    origin: Vec2,
    direction: Vec2,
    max_distance: f64,
    shape: &ColliderShape,
) -> Option<RayShapeHit> {
    match shape {
        ColliderShape::Circle { center, radius } => {
            ray_circle_hit(origin, direction, max_distance, *center, *radius)
        }
        ColliderShape::Polygon { points, .. } => {
            ray_polygon_hit(origin, direction, max_distance, points)
        }
    }
}

fn ray_circle_hit(
    origin: Vec2,
    direction: Vec2,
    max_distance: f64,
    center: Vec2,
    radius: f64,
) -> Option<RayShapeHit> {
    let oc = (origin.0 - center.0, origin.1 - center.1);
    let c = dot(oc, oc) - radius * radius;
    if c <= 0.0 {
        let normal = normalize((origin.0 - center.0, origin.1 - center.1))
            .unwrap_or((-direction.0, -direction.1));
        return Some((0.0, origin, normal));
    }
    let b = 2.0 * dot(oc, direction);
    let discriminant = b * b - 4.0 * c;
    if discriminant < 0.0 {
        return None;
    }
    let t = (-b - discriminant.sqrt()) * 0.5;
    if !(0.0..=max_distance).contains(&t) {
        return None;
    }
    let point = (origin.0 + direction.0 * t, origin.1 + direction.1 * t);
    let normal = normalize((point.0 - center.0, point.1 - center.1)).unwrap_or((0.0, -1.0));
    Some((t, point, normal))
}

fn ray_polygon_hit(
    origin: Vec2,
    direction: Vec2,
    max_distance: f64,
    points: &[Vec2],
) -> Option<RayShapeHit> {
    if point_in_polygon(origin, points) {
        return Some((0.0, origin, (-direction.0, -direction.1)));
    }
    let mut best: Option<RayShapeHit> = None;
    for index in 0..points.len() {
        let a = points[index];
        let b = points[(index + 1) % points.len()];
        if let Some(t) = ray_segment_intersection(origin, direction, a, b)
            && t <= max_distance
        {
            let point = (origin.0 + direction.0 * t, origin.1 + direction.1 * t);
            let edge = (b.0 - a.0, b.1 - a.1);
            let mut normal = normalize((-edge.1, edge.0)).unwrap_or((0.0, -1.0));
            if dot(normal, direction) > 0.0 {
                normal = (-normal.0, -normal.1);
            }
            if best.as_ref().is_none_or(|(best_t, _, _)| t < *best_t) {
                best = Some((t, point, normal));
            }
        }
    }
    best
}

fn ray_segment_intersection(
    origin: (f64, f64),
    direction: (f64, f64),
    a: (f64, f64),
    b: (f64, f64),
) -> Option<f64> {
    let segment = (b.0 - a.0, b.1 - a.1);
    let denom = cross(direction, segment);
    if denom.abs() <= 1e-9 {
        return None;
    }
    let to_segment = (a.0 - origin.0, a.1 - origin.1);
    let t = cross(to_segment, segment) / denom;
    let u = cross(to_segment, direction) / denom;
    if t >= 0.0 && (0.0..=1.0).contains(&u) {
        Some(t)
    } else {
        None
    }
}

fn parse_polygon_points(value: Option<&serde_json::Value>) -> Option<Vec<(f64, f64)>> {
    let points = value?
        .as_array()?
        .iter()
        .filter_map(|item| {
            let pair = item.as_array()?;
            Some((pair.first()?.as_f64()?, pair.get(1)?.as_f64()?))
        })
        .collect::<Vec<_>>();
    if points.len() >= 3 {
        Some(points)
    } else {
        None
    }
}

fn rect_points(width: f64, height: f64) -> Vec<(f64, f64)> {
    let hw = width * 0.5;
    let hh = height * 0.5;
    vec![(-hw, -hh), (hw, -hh), (hw, hh), (-hw, hh)]
}

fn circle_proxy_points(center: (f64, f64), radius: f64) -> Vec<(f64, f64)> {
    vec![
        (center.0 - radius, center.1),
        (center.0, center.1 - radius),
        (center.0 + radius, center.1),
        (center.0, center.1 + radius),
    ]
}

fn polygon_axes(points: &[(f64, f64)]) -> Vec<(f64, f64)> {
    if points.len() < 2 {
        return Vec::new();
    }
    let mut axes = Vec::new();
    for index in 0..points.len() {
        let a = points[index];
        let b = points[(index + 1) % points.len()];
        let edge = (b.0 - a.0, b.1 - a.1);
        if let Some(axis) = normalize((-edge.1, edge.0)) {
            axes.push(axis);
        }
    }
    axes
}

fn closest_vertex_axis(points: &[(f64, f64)], center: (f64, f64)) -> Option<(f64, f64)> {
    points
        .iter()
        .min_by(|a, b| {
            let da = length((a.0 - center.0, a.1 - center.1));
            let db = length((b.0 - center.0, b.1 - center.1));
            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
        })
        .and_then(|point| normalize((point.0 - center.0, point.1 - center.1)))
}

fn project_points(points: &[(f64, f64)], axis: (f64, f64)) -> (f64, f64) {
    let mut min = f64::MAX;
    let mut max = f64::MIN;
    for point in points {
        let projected = dot(*point, axis);
        min = min.min(projected);
        max = max.max(projected);
    }
    (min, max)
}

fn projection_overlap(first: (f64, f64), second: (f64, f64)) -> f64 {
    first.1.min(second.1) - first.0.max(second.0)
}

fn point_in_polygon(point: (f64, f64), points: &[(f64, f64)]) -> bool {
    let mut inside = false;
    let mut j = points.len().saturating_sub(1);
    for i in 0..points.len() {
        let pi = points[i];
        let pj = points[j];
        if ((pi.1 > point.1) != (pj.1 > point.1))
            && point.0 < (pj.0 - pi.0) * (point.1 - pi.1) / (pj.1 - pi.1) + pi.0
        {
            inside = !inside;
        }
        j = i;
    }
    inside
}

fn body_kind(body: &crate::engine::component::Component) -> BodyKind {
    match body.get_string("body_type", "dynamic").as_str() {
        "static" => BodyKind::Static,
        "kinematic" => BodyKind::Kinematic,
        _ => BodyKind::Dynamic,
    }
}

fn rigidbody_kind(entity: &GameObject) -> BodyKind {
    entity
        .get_component("Rigidbody2D")
        .map(body_kind)
        .unwrap_or(BodyKind::Static)
}

fn inverse_mass(entity: &GameObject, kind: BodyKind) -> f64 {
    if kind != BodyKind::Dynamic {
        return 0.0;
    }
    entity
        .get_component("Rigidbody2D")
        .map(|body| 1.0 / body.get_f64("mass", 1.0).max(0.0001))
        .unwrap_or(0.0)
}

fn collider_is_trigger(entity: &GameObject) -> bool {
    entity
        .get_component("Collider2D")
        .is_some_and(|collider| collider.get_bool("is_trigger", false))
}

fn collision_layer(entity: &GameObject) -> String {
    let Some(collider) = entity.get_component("Collider2D") else {
        return entity.layer.clone();
    };
    let layer = collider.get_string("collision_layer", &entity.layer);
    if layer == "Default" && entity.layer != "Default" {
        entity.layer.clone()
    } else {
        layer
    }
}

fn collider_mask_allows(entity: &GameObject, other_layer: &str) -> bool {
    let Some(collider) = entity.get_component("Collider2D") else {
        return true;
    };
    let mask = collider.get_string_list("collision_mask");
    mask.is_empty()
        || mask.iter().any(|layer| {
            layer == "*"
                || layer == other_layer
                || (layer == "Default" && other_layer == entity.layer.as_str())
        })
}

fn ordered_pair_names(first: &GameObject, second: &GameObject) -> (String, String) {
    if first.id <= second.id {
        (first.name.clone(), second.name.clone())
    } else {
        (second.name.clone(), first.name.clone())
    }
}

fn rotate_point(point: (f64, f64), radians: f64) -> (f64, f64) {
    let (sin, cos) = radians.sin_cos();
    (point.0 * cos - point.1 * sin, point.0 * sin + point.1 * cos)
}

fn normalize(vector: (f64, f64)) -> Option<(f64, f64)> {
    let len = length(vector);
    if len <= f64::EPSILON {
        None
    } else {
        Some((vector.0 / len, vector.1 / len))
    }
}

fn length(vector: (f64, f64)) -> f64 {
    (vector.0 * vector.0 + vector.1 * vector.1).sqrt()
}

fn dot(first: (f64, f64), second: (f64, f64)) -> f64 {
    first.0 * second.0 + first.1 * second.1
}

fn cross(first: (f64, f64), second: (f64, f64)) -> f64 {
    first.0 * second.1 - first.1 * second.0
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
