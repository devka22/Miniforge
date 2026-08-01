use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use crate::entities::game_object::GameObject;
use crate::systems::rapier_physics_bridge::{RapierPhysicsBridge, RapierSceneReport};
use crate::systems::spatial_index::{EntitySpatialIndex, entity_aabb};

type Vec2 = (f64, f64);
type RayShapeHit = (f64, Vec2, Vec2);
type PairMap = BTreeMap<(u64, u64), PairType>;
type ContactMap = BTreeMap<(u64, u64), Contact>;
type PairNameMap = BTreeMap<(u64, u64), (String, String)>;

#[derive(Debug, Clone)]
pub struct PhysicsSystem {
    pub gravity: (f64, f64),
    pub solver_iterations: usize,
    pub fixed_delta: f64,
    pub max_substeps: usize,
    pub continuous_collision: bool,
    pub sleeping_enabled: bool,
    pub active_pairs: BTreeMap<(u64, u64), PairType>,
    pub active_contacts: BTreeMap<(u64, u64), Contact>,
    pub layer_matrix: BTreeMap<(String, String), bool>,
    pub events: Vec<PhysicsEvent>,
    pub stats: BTreeMap<String, usize>,
    pub rapier_report: Option<RapierSceneReport>,
    active_pair_names: BTreeMap<(u64, u64), (String, String)>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PhysicsDebugSnapshot2D {
    pub bodies: Vec<PhysicsDebugBody2D>,
    pub contacts: Vec<PhysicsDebugContact2D>,
    pub joints: Vec<PhysicsDebugJoint2D>,
    pub force_fields: Vec<PhysicsDebugForceField2D>,
    pub stats: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PhysicsDebugBody2D {
    pub entity_id: u64,
    pub entity_name: String,
    pub body_type: String,
    pub position: [f64; 2],
    pub velocity: [f64; 2],
    pub sleeping: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PhysicsDebugContact2D {
    pub first_id: u64,
    pub second_id: u64,
    pub first_name: String,
    pub second_name: String,
    pub normal: [f64; 2],
    pub depth: f64,
    pub trigger: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PhysicsDebugJoint2D {
    pub entity_id: u64,
    pub target_id: u64,
    pub joint_type: String,
    pub anchor: [f64; 2],
    pub target_anchor: [f64; 2],
    pub broken: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PhysicsDebugForceField2D {
    pub entity_id: u64,
    pub field_type: String,
    pub position: [f64; 2],
    pub direction: [f64; 2],
    pub strength: f64,
    pub radius: f64,
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

struct CollisionPass {
    collider_count: usize,
    brute_force_pairs: usize,
    pairs: PairMap,
    contacts: ContactMap,
    names: PairNameMap,
    broadphase_candidates: usize,
}

#[derive(Debug, Clone)]
struct ForceField {
    entity_id: u64,
    field_type: String,
    position: Vec2,
    direction: Vec2,
    strength: f64,
    radius: f64,
    falloff: f64,
    layers: Vec<String>,
}

#[derive(Debug, Clone)]
struct JointLink {
    owner_index: usize,
    target_index: usize,
    owner_id: u64,
    target_id: u64,
    joint_type: String,
    anchor: Vec2,
    target_anchor: Vec2,
    rest_length: f64,
    min_distance: f64,
    max_distance: f64,
    stiffness: f64,
    damping: f64,
    break_force: f64,
}

#[derive(Debug, Clone, Copy)]
struct PhysicsMaterial {
    friction: f64,
    bounciness: f64,
    friction_combine: MaterialCombine,
    bounce_combine: MaterialCombine,
}

#[derive(Debug, Clone, Copy)]
enum MaterialCombine {
    Average,
    Minimum,
    Maximum,
    Multiply,
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

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PhysicsQueryFilter<'a> {
    pub include_triggers: bool,
    pub layers: Option<&'a [String]>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoxCastQuery<'a> {
    pub origin: (f64, f64),
    pub half_extents: (f64, f64),
    pub direction: (f64, f64),
    pub max_distance: f64,
    pub filter: PhysicsQueryFilter<'a>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CircleCastQuery<'a> {
    pub origin: (f64, f64),
    pub radius: f64,
    pub direction: (f64, f64),
    pub max_distance: f64,
    pub filter: PhysicsQueryFilter<'a>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShapeCastQuery<'a> {
    pub shape: ShapeCastKind,
    pub origin: (f64, f64),
    pub direction: (f64, f64),
    pub max_distance: f64,
    pub filter: PhysicsQueryFilter<'a>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BodyKind {
    Dynamic,
    Static,
    Kinematic,
}

impl BodyKind {
    fn name(self) -> &'static str {
        match self {
            Self::Dynamic => "dynamic",
            Self::Static => "static",
            Self::Kinematic => "kinematic",
        }
    }
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
            fixed_delta: 1.0 / 60.0,
            max_substeps: 4,
            continuous_collision: true,
            sleeping_enabled: true,
            active_pairs: BTreeMap::new(),
            active_contacts: BTreeMap::new(),
            layer_matrix: BTreeMap::new(),
            events: Vec::new(),
            stats: default_stats(),
            rapier_report: None,
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

        let fixed_delta = if self.fixed_delta.is_finite() {
            self.fixed_delta.clamp(1.0 / 480.0, 1.0 / 15.0)
        } else {
            1.0 / 60.0
        };
        let max_substeps = self.max_substeps.clamp(1, 16);
        let dt = dt.clamp(0.0, fixed_delta * max_substeps as f64);
        let body_requests_ccd = entities.iter().any(|entity| {
            entity
                .get_component("Rigidbody2D")
                .is_some_and(|body| body.get_bool("continuous_collision", false))
        });
        let substeps = if self.continuous_collision || body_requests_ccd {
            (dt / fixed_delta).ceil().max(1.0) as usize
        } else {
            1
        }
        .clamp(1, max_substeps);
        let step_dt = dt / substeps as f64;
        let mut final_pass = None;
        let mut peak_broadphase_candidates = 0;
        let mut active_joints = 0;
        let mut broken_joints = 0;
        for _ in 0..substeps {
            self.apply_force_fields(entities, step_dt);
            self.integrate_bodies(entities, step_dt);
            let joint_stats = self.solve_joints(entities, step_dt);
            active_joints = active_joints.max(joint_stats.0);
            broken_joints = broken_joints.max(joint_stats.1);
            let pass = self.solve_collision_pass(entities);
            peak_broadphase_candidates = peak_broadphase_candidates.max(pass.broadphase_candidates);
            final_pass = Some(pass);
        }
        self.update_sleeping(entities, dt);
        clear_accumulated_forces(entities);
        self.rapier_report = Some(RapierPhysicsBridge::inspect_scene(entities, self.gravity));

        let final_pass = final_pass.expect("at least one physics substep");

        self.dispatch_pair_events(&final_pass.pairs, &final_pass.contacts, &final_pass.names);
        let bodies = entities
            .iter()
            .filter(|entity| {
                entity.get_component("Rigidbody2D").is_some()
                    || entity.get_component("KinematicBody2D").is_some()
                    || entity.get_component("StaticBody2D").is_some()
            })
            .count();
        let sleeping_bodies = entities
            .iter()
            .filter(|entity| {
                entity
                    .get_component("Rigidbody2D")
                    .is_some_and(|body| body.get_bool("sleeping", false))
            })
            .count();
        let force_fields = entities
            .iter()
            .filter(|entity| {
                entity
                    .get_component("ForceField2D")
                    .is_some_and(|field| field.enabled && field.get_bool("enabled", true))
            })
            .count();
        let triggers = final_pass
            .pairs
            .values()
            .filter(|pair| **pair == PairType::Trigger)
            .count();
        let collisions = final_pass.pairs.len().saturating_sub(triggers);
        self.stats.extend(BTreeMap::from([
            ("bodies".to_string(), bodies),
            ("colliders".to_string(), final_pass.collider_count),
            ("pairs".to_string(), final_pass.pairs.len()),
            ("contacts".to_string(), final_pass.pairs.len()),
            (
                "broadphase_candidates".to_string(),
                peak_broadphase_candidates,
            ),
            (
                "broadphase_rejected".to_string(),
                final_pass
                    .brute_force_pairs
                    .saturating_sub(peak_broadphase_candidates),
            ),
            ("triggers".to_string(), triggers),
            ("collisions".to_string(), collisions),
            ("sleeping_bodies".to_string(), sleeping_bodies),
            ("joints".to_string(), active_joints),
            ("broken_joints".to_string(), broken_joints),
            ("force_fields".to_string(), force_fields),
            ("substeps".to_string(), substeps),
            (
                "rapier_ready_colliders".to_string(),
                self.rapier_report
                    .as_ref()
                    .map(|report| report.colliders)
                    .unwrap_or(0),
            ),
        ]));
    }

    pub fn apply_force(&self, entities: &mut [GameObject], entity_id: u64, force: Vec2) -> bool {
        let Some(body) = entities
            .iter_mut()
            .find(|entity| entity.id == entity_id)
            .and_then(|entity| entity.get_component_mut("Rigidbody2D"))
        else {
            return false;
        };
        if body_kind(body) != BodyKind::Dynamic {
            return false;
        }
        let force = finite_vec(force);
        body.set_f64(
            "_force_x",
            body.get_f64("_force_x", 0.0) + force.0.clamp(-1.0e9, 1.0e9),
        );
        body.set_f64(
            "_force_y",
            body.get_f64("_force_y", 0.0) + force.1.clamp(-1.0e9, 1.0e9),
        );
        wake_component(body);
        true
    }

    pub fn apply_impulse(
        &self,
        entities: &mut [GameObject],
        entity_id: u64,
        impulse: Vec2,
    ) -> bool {
        let Some(body) = entities
            .iter_mut()
            .find(|entity| entity.id == entity_id)
            .and_then(|entity| entity.get_component_mut("Rigidbody2D"))
        else {
            return false;
        };
        if body_kind(body) != BodyKind::Dynamic {
            return false;
        }
        let impulse = finite_vec(impulse);
        let inverse_mass = 1.0 / body.get_f64("mass", 1.0).max(0.0001);
        body.set_f64(
            "velocity_x",
            body.get_f64("velocity_x", 0.0) + impulse.0 * inverse_mass,
        );
        body.set_f64(
            "velocity_y",
            body.get_f64("velocity_y", 0.0) + impulse.1 * inverse_mass,
        );
        wake_component(body);
        true
    }

    pub fn apply_torque(&self, entities: &mut [GameObject], entity_id: u64, torque: f64) -> bool {
        let Some(body) = entities
            .iter_mut()
            .find(|entity| entity.id == entity_id)
            .and_then(|entity| entity.get_component_mut("Rigidbody2D"))
        else {
            return false;
        };
        if body_kind(body) != BodyKind::Dynamic {
            return false;
        }
        body.set_f64(
            "_torque",
            body.get_f64("_torque", 0.0) + finite(torque).clamp(-1.0e9, 1.0e9),
        );
        wake_component(body);
        true
    }

    pub fn wake_body(&self, entities: &mut [GameObject], entity_id: u64) -> bool {
        let Some(body) = entities
            .iter_mut()
            .find(|entity| entity.id == entity_id)
            .and_then(|entity| entity.get_component_mut("Rigidbody2D"))
        else {
            return false;
        };
        wake_component(body);
        true
    }

    pub fn debug_snapshot(
        &self,
        entities: &[GameObject],
        maximum_items: usize,
    ) -> PhysicsDebugSnapshot2D {
        let limit = maximum_items.clamp(1, 10_000);
        let bodies = entities
            .iter()
            .filter_map(|entity| {
                let component = motion_body_component(entity)?;
                Some(PhysicsDebugBody2D {
                    entity_id: entity.id,
                    entity_name: entity.name.clone(),
                    body_type: body_kind(component).name().to_string(),
                    position: [entity.x, entity.y],
                    velocity: [
                        component.get_f64("velocity_x", 0.0),
                        component.get_f64("velocity_y", 0.0),
                    ],
                    sleeping: component.get_bool("sleeping", false),
                })
            })
            .take(limit)
            .collect();
        let contacts = self
            .active_contacts
            .iter()
            .take(limit)
            .map(|(pair, contact)| {
                let names = self
                    .active_pair_names
                    .get(pair)
                    .cloned()
                    .unwrap_or_default();
                PhysicsDebugContact2D {
                    first_id: pair.0,
                    second_id: pair.1,
                    first_name: names.0,
                    second_name: names.1,
                    normal: [contact.normal.0, contact.normal.1],
                    depth: contact.depth,
                    trigger: self.active_pairs.get(pair) == Some(&PairType::Trigger),
                }
            })
            .collect();
        let joints = joint_links(entities)
            .into_iter()
            .take(limit)
            .map(|joint| PhysicsDebugJoint2D {
                entity_id: joint.owner_id,
                target_id: joint.target_id,
                joint_type: joint.joint_type,
                anchor: [joint.anchor.0, joint.anchor.1],
                target_anchor: [joint.target_anchor.0, joint.target_anchor.1],
                broken: false,
            })
            .collect();
        let force_fields = collect_force_fields(entities)
            .into_iter()
            .take(limit)
            .map(|field| PhysicsDebugForceField2D {
                entity_id: field.entity_id,
                field_type: field.field_type,
                position: [field.position.0, field.position.1],
                direction: [field.direction.0, field.direction.1],
                strength: field.strength,
                radius: field.radius,
            })
            .collect();
        PhysicsDebugSnapshot2D {
            bodies,
            contacts,
            joints,
            force_fields,
            stats: self.stats.clone(),
        }
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
        let dir = normalize(direction)?;
        let max_distance = max_distance.max(0.0);
        let mut nearest = None;
        for entity in entities {
            let Some(hit) =
                raycast_entity_hit(entity, origin, dir, max_distance, include_triggers, layers)
            else {
                continue;
            };
            if nearest
                .as_ref()
                .is_none_or(|current: &RaycastHit| hit.distance < current.distance)
            {
                nearest = Some(hit);
            }
        }
        nearest
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
            if let Some(hit) =
                raycast_entity_hit(entity, origin, dir, max_distance, include_triggers, layers)
            {
                hits.push(hit);
            }
        }
        hits.sort_by(|a, b| {
            a.distance
                .partial_cmp(&b.distance)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        hits
    }

    pub fn box_cast_filtered(
        &self,
        entities: &[GameObject],
        query: BoxCastQuery<'_>,
    ) -> Option<RaycastHit> {
        self.shape_cast_all_filtered(
            entities,
            ShapeCastQuery {
                shape: ShapeCastKind::Box {
                    half_extents: (query.half_extents.0.abs(), query.half_extents.1.abs()),
                },
                origin: query.origin,
                direction: query.direction,
                max_distance: query.max_distance,
                filter: query.filter,
            },
        )
        .into_iter()
        .next()
    }

    pub fn circle_cast_filtered(
        &self,
        entities: &[GameObject],
        query: CircleCastQuery<'_>,
    ) -> Option<RaycastHit> {
        self.shape_cast_all_filtered(
            entities,
            ShapeCastQuery {
                shape: ShapeCastKind::Circle {
                    radius: query.radius.abs(),
                },
                origin: query.origin,
                direction: query.direction,
                max_distance: query.max_distance,
                filter: query.filter,
            },
        )
        .into_iter()
        .next()
    }

    pub fn shape_cast_all_filtered(
        &self,
        entities: &[GameObject],
        query: ShapeCastQuery<'_>,
    ) -> Vec<RaycastHit> {
        let Some(dir) = normalize(query.direction) else {
            return Vec::new();
        };
        let max_distance = query.max_distance.max(0.0);
        let steps = ((max_distance / 0.25).ceil() as usize).clamp(1, 96);
        let mut hits = Vec::new();
        for entity in entities {
            if !entity.enabled || !entity.visible {
                continue;
            }
            let Some(collider) = physics_shape_component(entity) else {
                continue;
            };
            if !collider.enabled {
                continue;
            }
            let is_trigger = collider_is_trigger(entity);
            if is_trigger && !query.filter.include_triggers {
                continue;
            }
            let layer = collision_layer(entity);
            if query
                .filter
                .layers
                .is_some_and(|items| !items.iter().any(|item| item == &layer))
            {
                continue;
            }
            let Some(target_shape) = collider_shape(entity) else {
                continue;
            };
            for step in 0..=steps {
                let distance = max_distance * step as f64 / steps as f64;
                let center = (
                    query.origin.0 + dir.0 * distance,
                    query.origin.1 + dir.1 * distance,
                );
                let cast_shape = query.shape.shape_at(center);
                if let Some(contact) = shape_contact(&cast_shape, &target_shape) {
                    hits.push(RaycastHit {
                        entity_id: entity.id,
                        entity_name: entity.name.clone(),
                        point: center,
                        normal: contact.normal,
                        distance,
                        layer: layer.clone(),
                        is_trigger,
                    });
                    break;
                }
            }
        }
        hits.sort_by(|a, b| {
            a.distance
                .partial_cmp(&b.distance)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        hits
    }

    pub fn overlap_area_filtered(
        &self,
        entities: &[GameObject],
        center: (f64, f64),
        half_extents: (f64, f64),
        include_triggers: bool,
        layers: Option<&[String]>,
    ) -> Vec<RaycastHit> {
        let area = ShapeCastKind::Box {
            half_extents: (half_extents.0.abs(), half_extents.1.abs()),
        }
        .shape_at(center);
        let mut hits = Vec::new();
        for entity in entities {
            if !entity.enabled || !entity.visible {
                continue;
            }
            let Some(collider) = physics_shape_component(entity) else {
                continue;
            };
            if !collider.enabled {
                continue;
            }
            let is_trigger = collider_is_trigger(entity);
            if is_trigger && !include_triggers {
                continue;
            }
            let layer = collision_layer(entity);
            if layers.is_some_and(|items| !items.iter().any(|item| item == &layer)) {
                continue;
            }
            let Some(target_shape) = collider_shape(entity) else {
                continue;
            };
            if let Some(contact) = shape_contact(&area, &target_shape) {
                let target_center = target_shape.center();
                hits.push(RaycastHit {
                    entity_id: entity.id,
                    entity_name: entity.name.clone(),
                    point: target_center,
                    normal: contact.normal,
                    distance: length((target_center.0 - center.0, target_center.1 - center.1)),
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

    fn apply_force_fields(&self, entities: &mut [GameObject], dt: f64) {
        let fields = collect_force_fields(entities);
        if fields.is_empty() {
            return;
        }
        for entity in entities {
            let layer = collision_layer(entity);
            let position = (entity.x, entity.y);
            let Some(body) = entity.get_component_mut("Rigidbody2D") else {
                continue;
            };
            if !body.enabled || body_kind(body) != BodyKind::Dynamic {
                continue;
            }
            let mass = body.get_f64("mass", 1.0).max(0.0001);
            let mut accumulated = (0.0, 0.0);
            for field in &fields {
                if !field.layers.is_empty()
                    && !field
                        .layers
                        .iter()
                        .any(|item| item == "*" || item == &layer)
                {
                    continue;
                }
                let offset = (position.0 - field.position.0, position.1 - field.position.1);
                let distance = length(offset);
                if field.radius > 0.0 && distance > field.radius {
                    continue;
                }
                let attenuation = if field.radius <= f64::EPSILON {
                    1.0
                } else {
                    (1.0 - distance / field.radius)
                        .clamp(0.0, 1.0)
                        .powf(field.falloff)
                };
                let direction = match field.field_type.as_str() {
                    "radial" => normalize(offset).unwrap_or((0.0, 0.0)),
                    "vortex" => normalize((-offset.1, offset.0)).unwrap_or((0.0, 0.0)),
                    _ => field.direction,
                };
                accumulated.0 += direction.0 * field.strength * attenuation;
                accumulated.1 += direction.1 * field.strength * attenuation;
            }
            if length(accumulated) <= f64::EPSILON {
                continue;
            }
            body.set_f64(
                "velocity_x",
                body.get_f64("velocity_x", 0.0) + accumulated.0 / mass * dt,
            );
            body.set_f64(
                "velocity_y",
                body.get_f64("velocity_y", 0.0) + accumulated.1 / mass * dt,
            );
            wake_component(body);
        }
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
                force_x,
                force_y,
                torque,
                mass,
            ) = {
                let Some(body) = motion_body_component(entity) else {
                    continue;
                };
                if !body.enabled || (self.sleeping_enabled && body.get_bool("sleeping", false)) {
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
                    body.get_f64("_force_x", 0.0),
                    body.get_f64("_force_y", 0.0),
                    body.get_f64("_torque", 0.0),
                    body.get_f64("mass", 1.0).max(0.0001),
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
                vx += force_x / mass * dt;
                vy += force_y / mass * dt;
                angular_velocity += torque / mass * dt;
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

            if let Some(body) = motion_body_component_mut(entity) {
                body.set_f64("velocity_x", vx);
                body.set_f64("velocity_y", vy);
                body.set_f64("angular_velocity", angular_velocity);
            }
            entity.sync_to_components();
        }
    }

    fn solve_joints(&self, entities: &mut [GameObject], dt: f64) -> (usize, usize) {
        let links = joint_links(entities);
        let mut active = 0;
        let mut broken = 0;
        for link in links {
            let (first, second) = two_entities_mut(entities, link.owner_index, link.target_index);
            let first_anchor = link.anchor;
            let second_anchor = link.target_anchor;
            let delta = (
                first_anchor.0 - second_anchor.0,
                first_anchor.1 - second_anchor.1,
            );
            let distance = length(delta);
            let normal = normalize(delta).unwrap_or((1.0, 0.0));
            let target_distance = if link.joint_type == "hinge" || link.joint_type == "fixed" {
                link.rest_length.max(0.0)
            } else if link.max_distance > 0.0 && distance > link.max_distance {
                link.max_distance
            } else if link.min_distance > 0.0 && distance < link.min_distance {
                link.min_distance
            } else {
                link.rest_length.max(0.0)
            };
            let error = distance - target_distance;
            let first_kind = rigidbody_kind(first);
            let second_kind = rigidbody_kind(second);
            let first_inverse_mass = inverse_mass(first, first_kind);
            let second_inverse_mass = inverse_mass(second, second_kind);
            let inverse_mass_sum = first_inverse_mass + second_inverse_mass;
            if inverse_mass_sum <= f64::EPSILON {
                active += 1;
                continue;
            }

            let first_velocity = body_velocity(first);
            let second_velocity = body_velocity(second);
            let relative_speed = dot(
                (
                    first_velocity.0 - second_velocity.0,
                    first_velocity.1 - second_velocity.1,
                ),
                normal,
            );
            let estimated_force = (error.abs() * link.stiffness
                + relative_speed.abs() * link.damping)
                / dt.max(1.0 / 480.0);
            if link.break_force > 0.0 && estimated_force > link.break_force {
                if let Some(joint) = first.get_component_mut("Joint2D") {
                    joint.set("broken", serde_json::json!(true));
                    joint.set_f64("current_force", estimated_force);
                }
                broken += 1;
                continue;
            }

            let spring = link.joint_type == "spring";
            let positional_strength = if spring {
                (link.stiffness * dt * 4.0).clamp(0.0, 0.35)
            } else {
                link.stiffness.clamp(0.0, 1.0)
            };
            let correction = error * positional_strength;
            if first_kind == BodyKind::Dynamic {
                let share = first_inverse_mass / inverse_mass_sum;
                first.x -= normal.0 * correction * share;
                first.y -= normal.1 * correction * share;
            }
            if second_kind == BodyKind::Dynamic {
                let share = second_inverse_mass / inverse_mass_sum;
                second.x += normal.0 * correction * share;
                second.y += normal.1 * correction * share;
            }

            let impulse = if spring {
                -(error * link.stiffness + relative_speed * link.damping) * dt
            } else {
                -relative_speed * link.damping.clamp(0.0, 1.0)
            };
            apply_velocity_impulse(first, normal, impulse * first_inverse_mass);
            apply_velocity_impulse(second, normal, -impulse * second_inverse_mass);
            first.sync_to_components();
            second.sync_to_components();
            if let Some(joint) = first.get_component_mut("Joint2D") {
                joint.set_f64("current_force", estimated_force);
                joint.set_f64("current_distance", distance);
            }
            active += 1;
        }
        (active, broken)
    }

    fn update_sleeping(&self, entities: &mut [GameObject], dt: f64) {
        if !self.sleeping_enabled {
            return;
        }
        for entity in entities {
            let Some(body) = entity.get_component_mut("Rigidbody2D") else {
                continue;
            };
            if !body.enabled
                || body_kind(body) != BodyKind::Dynamic
                || !body.get_bool("allow_sleeping", true)
            {
                body.set_f64("_sleep_timer", 0.0);
                body.set("sleeping", serde_json::json!(false));
                continue;
            }
            let linear_threshold = body.get_f64("linear_sleep_threshold", 0.03).max(0.0);
            let angular_threshold = body.get_f64("angular_sleep_threshold", 0.03).max(0.0);
            let sleep_time = body.get_f64("sleep_time_threshold", 0.5).max(0.0);
            let velocity = (
                body.get_f64("velocity_x", 0.0),
                body.get_f64("velocity_y", 0.0),
            );
            let angular_velocity = body.get_f64("angular_velocity", 0.0).abs();
            let has_force = body.get_f64("_force_x", 0.0).abs() > f64::EPSILON
                || body.get_f64("_force_y", 0.0).abs() > f64::EPSILON
                || body.get_f64("_torque", 0.0).abs() > f64::EPSILON;
            if has_force
                || length(velocity) > linear_threshold
                || angular_velocity > angular_threshold
            {
                body.set_f64("_sleep_timer", 0.0);
                body.set("sleeping", serde_json::json!(false));
                continue;
            }
            let timer = body.get_f64("_sleep_timer", 0.0) + dt.max(0.0);
            body.set_f64("_sleep_timer", timer);
            if timer >= sleep_time {
                body.set_f64("velocity_x", 0.0);
                body.set_f64("velocity_y", 0.0);
                body.set_f64("angular_velocity", 0.0);
                body.set("sleeping", serde_json::json!(true));
            }
        }
    }

    fn solve_collision_pass(&self, entities: &mut [GameObject]) -> CollisionPass {
        let colliders = entities
            .iter()
            .enumerate()
            .filter_map(|(index, entity)| {
                if entity.enabled
                    && entity.visible
                    && physics_shape_component(entity).is_some_and(|collider| collider.enabled)
                {
                    Some(index)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        let brute_force_pairs = colliders
            .len()
            .saturating_mul(colliders.len().saturating_sub(1))
            / 2;
        let mut current_pairs = BTreeMap::new();
        let mut current_contacts = BTreeMap::new();
        let mut current_names = BTreeMap::new();
        let mut broadphase_candidates = 0;
        for _ in 0..self.solver_iterations.max(1) {
            // Rebuild after each solver pass because contact resolution can move bodies.
            // R-tree construction is O(n log n) and prevents the narrow phase from
            // degenerating into O(n²) for sparse scenes.
            let broadphase = EntitySpatialIndex::from_bounds(colliders.iter().map(|&index| {
                let (min, max) = entity_aabb(&entities[index]);
                (index, entities[index].id, min, max)
            }));
            let candidates = broadphase.overlapping_pairs();
            broadphase_candidates = broadphase_candidates.max(candidates.len());
            for (first_index, second_index) in candidates {
                let (first, second) = two_entities_mut(entities, first_index, second_index);
                if !self.layer_collision_enabled(first, second) {
                    continue;
                }
                if joint_collision_disabled(first, second) {
                    continue;
                }
                let Some(contact) = compute_contact(first, second) else {
                    continue;
                };
                if one_way_contact_ignored(first, second, contact) {
                    continue;
                }
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
        CollisionPass {
            collider_count: colliders.len(),
            brute_force_pairs,
            pairs: current_pairs,
            contacts: current_contacts,
            names: current_names,
            broadphase_candidates,
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

fn raycast_entity_hit(
    entity: &GameObject,
    origin: Vec2,
    direction: Vec2,
    max_distance: f64,
    include_triggers: bool,
    layers: Option<&[String]>,
) -> Option<RaycastHit> {
    if !entity.enabled || !entity.visible {
        return None;
    }
    let collider = physics_shape_component(entity)?;
    if !collider.enabled {
        return None;
    }
    let is_trigger = collider_is_trigger(entity);
    if is_trigger && !include_triggers {
        return None;
    }
    let (min, max) = entity_aabb(entity);
    if !ray_intersects_aabb(origin, direction, max_distance, min, max) {
        return None;
    }
    let filtered_layer = if let Some(items) = layers {
        let layer = collision_layer(entity);
        if !items.iter().any(|item| item == &layer) {
            return None;
        }
        Some(layer)
    } else {
        None
    };
    let shape = collider_shape(entity)?;
    let (distance, point, normal) = ray_shape_hit(origin, direction, max_distance, &shape)?;
    let layer = filtered_layer.unwrap_or_else(|| collision_layer(entity));
    Some(RaycastHit {
        entity_id: entity.id,
        entity_name: entity.name.clone(),
        point,
        normal,
        distance,
        layer,
        is_trigger,
    })
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ShapeCastKind {
    Box { half_extents: (f64, f64) },
    Circle { radius: f64 },
}

impl ShapeCastKind {
    fn shape_at(self, center: (f64, f64)) -> ColliderShape {
        match self {
            Self::Box { half_extents } => ColliderShape::Polygon {
                center,
                points: rect_points(half_extents.0 * 2.0, half_extents.1 * 2.0)
                    .into_iter()
                    .map(|point| (center.0 + point.0, center.1 + point.1))
                    .collect(),
            },
            Self::Circle { radius } => ColliderShape::Circle {
                center,
                radius: radius.max(0.001),
            },
        }
    }
}

fn default_stats() -> BTreeMap<String, usize> {
    BTreeMap::from([
        ("bodies".to_string(), 0),
        ("colliders".to_string(), 0),
        ("pairs".to_string(), 0),
        ("contacts".to_string(), 0),
        ("broadphase_candidates".to_string(), 0),
        ("broadphase_rejected".to_string(), 0),
        ("triggers".to_string(), 0),
        ("collisions".to_string(), 0),
        ("sleeping_bodies".to_string(), 0),
        ("joints".to_string(), 0),
        ("broken_joints".to_string(), 0),
        ("force_fields".to_string(), 0),
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

fn one_way_contact_ignored(first: &GameObject, second: &GameObject, contact: Contact) -> bool {
    if let Some(normal) = one_way_normal(second) {
        return dot(contact.normal, normal) < 0.35;
    }
    if let Some(normal) = one_way_normal(first) {
        return dot((-contact.normal.0, -contact.normal.1), normal) < 0.35;
    }
    false
}

fn one_way_normal(entity: &GameObject) -> Option<(f64, f64)> {
    if let Some(one_way) = entity
        .get_component("OneWayPlatform2D")
        .filter(|component| component.enabled && component.get_bool("enabled", true))
    {
        return normalize((
            one_way.get_f64("normal_x", 0.0),
            one_way.get_f64("normal_y", -1.0),
        ))
        .or(Some((0.0, -1.0)));
    }
    if let Some(static_body) = entity
        .get_component("StaticBody2D")
        .filter(|component| component.enabled && component.get_bool("one_way", false))
    {
        return normalize((
            static_body.get_f64("one_way_normal_x", 0.0),
            static_body.get_f64("one_way_normal_y", -1.0),
        ))
        .or(Some((0.0, -1.0)));
    }
    entity
        .get_component("Collider2D")
        .filter(|component| component.get_bool("one_way", false))
        .and_then(|collider| {
            normalize((
                collider.get_f64("one_way_normal_x", 0.0),
                collider.get_f64("one_way_normal_y", -1.0),
            ))
            .or(Some((0.0, -1.0)))
        })
}

fn collider_shape(entity: &GameObject) -> Option<ColliderShape> {
    let collider = physics_shape_component(entity)?;
    let center = (
        entity.x + collider.get_f64("offset_x", 0.0),
        entity.y + collider.get_f64("offset_y", 0.0),
    );
    let shape = collider.get_string("shape", "rect").to_lowercase();
    if shape == "circle" {
        // Circle colliders remain circular under a non-uniform entity transform.
        // Use the largest axis, matching the conservative broad-phase AABB.
        // This keeps both phases aligned; a future ellipse shape can model the
        // two axes independently.
        let scale = entity.scale_x.abs().max(entity.scale_y.abs());
        return Some(ColliderShape::Circle {
            center,
            radius: (collider.get_f64("radius", 0.5).max(0.001) * scale).max(0.001),
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
    }
    if second_kind == BodyKind::Dynamic {
        let share = second_inv_mass / total_inv_mass;
        second.x -= nx * depth * share;
        second.y -= ny * depth * share;
    }

    let first_velocity = body_velocity(first);
    let second_velocity = body_velocity(second);
    let relative_velocity = (
        first_velocity.0 - second_velocity.0,
        first_velocity.1 - second_velocity.1,
    );
    let normal_speed = dot(relative_velocity, contact.normal);
    if normal_speed < 0.0 {
        let first_material = physics_material(first);
        let second_material = physics_material(second);
        let bounciness = combine_material_value(
            first_material.bounciness,
            second_material.bounciness,
            strongest_combine(
                first_material.bounce_combine,
                second_material.bounce_combine,
            ),
        )
        .clamp(0.0, 1.0);
        let normal_impulse = -(1.0 + bounciness) * normal_speed / total_inv_mass;
        apply_velocity_impulse(first, contact.normal, normal_impulse * first_inv_mass);
        apply_velocity_impulse(second, contact.normal, -normal_impulse * second_inv_mass);

        let updated_relative = {
            let first_velocity = body_velocity(first);
            let second_velocity = body_velocity(second);
            (
                first_velocity.0 - second_velocity.0,
                first_velocity.1 - second_velocity.1,
            )
        };
        let tangent = (
            updated_relative.0 - dot(updated_relative, contact.normal) * nx,
            updated_relative.1 - dot(updated_relative, contact.normal) * ny,
        );
        if let Some(tangent) = normalize(tangent) {
            let tangent_speed = dot(updated_relative, tangent);
            let friction = combine_material_value(
                first_material.friction,
                second_material.friction,
                strongest_combine(
                    first_material.friction_combine,
                    second_material.friction_combine,
                ),
            )
            .clamp(0.0, 4.0);
            let tangent_impulse = (-tangent_speed / total_inv_mass)
                .clamp(-normal_impulse * friction, normal_impulse * friction);
            apply_velocity_impulse(first, tangent, tangent_impulse * first_inv_mass);
            apply_velocity_impulse(second, tangent, -tangent_impulse * second_inv_mass);
        }
    }
    first.sync_to_components();
    second.sync_to_components();
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

fn ray_intersects_aabb(
    origin: Vec2,
    direction: Vec2,
    max_distance: f64,
    min: [f64; 2],
    max: [f64; 2],
) -> bool {
    let mut t_min: f64 = 0.0;
    let mut t_max: f64 = max_distance.max(0.0);
    for axis in 0..2 {
        let origin_axis = if axis == 0 { origin.0 } else { origin.1 };
        let direction_axis = if axis == 0 { direction.0 } else { direction.1 };
        if direction_axis.abs() <= f64::EPSILON {
            if origin_axis < min[axis] || origin_axis > max[axis] {
                return false;
            }
            continue;
        }
        let inv_direction = 1.0 / direction_axis;
        let mut enter = (min[axis] - origin_axis) * inv_direction;
        let mut exit = (max[axis] - origin_axis) * inv_direction;
        if enter > exit {
            std::mem::swap(&mut enter, &mut exit);
        }
        t_min = t_min.max(enter);
        t_max = t_max.min(exit);
        if t_min > t_max {
            return false;
        }
    }
    true
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
    if body.component_type == "KinematicBody2D" {
        return BodyKind::Kinematic;
    }
    if body.component_type == "StaticBody2D" {
        return BodyKind::Static;
    }
    match body.get_string("body_type", "dynamic").as_str() {
        "static" => BodyKind::Static,
        "kinematic" => BodyKind::Kinematic,
        _ => BodyKind::Dynamic,
    }
}

fn rigidbody_kind(entity: &GameObject) -> BodyKind {
    motion_body_component(entity)
        .map(body_kind)
        .or_else(|| {
            entity
                .get_component("StaticBody2D")
                .filter(|body| body.enabled)
                .map(body_kind)
        })
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

fn motion_body_component(entity: &GameObject) -> Option<&crate::engine::component::Component> {
    entity
        .get_component("Rigidbody2D")
        .filter(|component| component.enabled)
        .or_else(|| {
            entity
                .get_component("KinematicBody2D")
                .filter(|component| component.enabled)
        })
}

fn motion_body_component_mut(
    entity: &mut GameObject,
) -> Option<&mut crate::engine::component::Component> {
    let index = entity.components.iter().position(|component| {
        component.enabled
            && (component.component_type == "Rigidbody2D"
                || component.component_type == "KinematicBody2D")
    })?;
    entity.components.get_mut(index)
}

fn body_velocity(entity: &GameObject) -> Vec2 {
    motion_body_component(entity)
        .map(|body| {
            (
                body.get_f64("velocity_x", 0.0),
                body.get_f64("velocity_y", 0.0),
            )
        })
        .unwrap_or((0.0, 0.0))
}

fn apply_velocity_impulse(entity: &mut GameObject, direction: Vec2, magnitude: f64) {
    if magnitude.abs() <= f64::EPSILON {
        return;
    }
    let Some(body) = entity.get_component_mut("Rigidbody2D") else {
        return;
    };
    if !body.enabled || body_kind(body) != BodyKind::Dynamic {
        return;
    }
    let mut vx = body.get_f64("velocity_x", 0.0) + direction.0 * magnitude;
    let mut vy = body.get_f64("velocity_y", 0.0) + direction.1 * magnitude;
    if length((vx, vy)) < 0.001 {
        vx = 0.0;
        vy = 0.0;
    }
    body.set_f64("velocity_x", finite(vx));
    body.set_f64("velocity_y", finite(vy));
}

fn wake_component(body: &mut crate::engine::component::Component) {
    body.set("sleeping", serde_json::json!(false));
    body.set_f64("_sleep_timer", 0.0);
}

fn clear_accumulated_forces(entities: &mut [GameObject]) {
    for entity in entities {
        let Some(body) = entity.get_component_mut("Rigidbody2D") else {
            continue;
        };
        body.set_f64("_force_x", 0.0);
        body.set_f64("_force_y", 0.0);
        body.set_f64("_torque", 0.0);
    }
}

fn collect_force_fields(entities: &[GameObject]) -> Vec<ForceField> {
    entities
        .iter()
        .filter_map(|entity| {
            let field = entity
                .get_component("ForceField2D")
                .filter(|field| field.enabled && field.get_bool("enabled", true))?;
            let direction = normalize((
                field.get_f64("direction_x", 1.0),
                field.get_f64("direction_y", 0.0),
            ))
            .unwrap_or((1.0, 0.0));
            Some(ForceField {
                entity_id: entity.id,
                field_type: match field.get_string("field_type", "directional").as_str() {
                    "radial" => "radial",
                    "vortex" => "vortex",
                    _ => "directional",
                }
                .to_string(),
                position: (entity.x, entity.y),
                direction,
                strength: finite(field.get_f64("strength", 10.0)).clamp(-1.0e6, 1.0e6),
                radius: finite(field.get_f64("radius", 8.0)).clamp(0.0, 1.0e6),
                falloff: finite(field.get_f64("falloff", 1.0)).clamp(0.0, 16.0),
                layers: field.get_string_list("layers"),
            })
        })
        .collect()
}

fn joint_links(entities: &[GameObject]) -> Vec<JointLink> {
    entities
        .iter()
        .enumerate()
        .filter_map(|(owner_index, owner)| {
            let joint = owner
                .get_component("Joint2D")
                .filter(|joint| joint.enabled && !joint.get_bool("broken", false))?;
            let target_id = joint.get("target_id").and_then(serde_json::Value::as_u64);
            let target_name = joint.get_string("target_name", "");
            let target_index = entities.iter().position(|candidate| {
                candidate.id != owner.id
                    && (target_id.is_some_and(|id| candidate.id == id)
                        || (!target_name.is_empty() && candidate.name == target_name))
            })?;
            let target = &entities[target_index];
            let joint_type = match joint.get_string("joint_type", "distance").as_str() {
                "spring" => "spring",
                "fixed" => "fixed",
                "hinge" => "hinge",
                _ => "distance",
            }
            .to_string();
            Some(JointLink {
                owner_index,
                target_index,
                owner_id: owner.id,
                target_id: target.id,
                joint_type,
                anchor: (
                    owner.x + joint.get_f64("anchor_x", 0.0),
                    owner.y + joint.get_f64("anchor_y", 0.0),
                ),
                target_anchor: (
                    target.x + joint.get_f64("target_anchor_x", 0.0),
                    target.y + joint.get_f64("target_anchor_y", 0.0),
                ),
                rest_length: finite(joint.get_f64("rest_length", 2.0)).max(0.0),
                min_distance: finite(joint.get_f64("min_distance", 0.0)).max(0.0),
                max_distance: finite(joint.get_f64("max_distance", 2.0)).max(0.0),
                stiffness: finite(joint.get_f64("stiffness", 0.85)).clamp(0.0, 1.0e5),
                damping: finite(joint.get_f64("damping", 0.18)).clamp(0.0, 1.0e5),
                break_force: finite(joint.get_f64("break_force", 0.0)).max(0.0),
            })
        })
        .collect()
}

fn joint_collision_disabled(first: &GameObject, second: &GameObject) -> bool {
    [first, second].into_iter().any(|owner| {
        owner
            .get_component("Joint2D")
            .filter(|joint| joint.enabled && !joint.get_bool("broken", false))
            .is_some_and(|joint| {
                if joint.get_bool("collide_connected", false) {
                    return false;
                }
                let other = if owner.id == first.id { second } else { first };
                joint
                    .get("target_id")
                    .and_then(serde_json::Value::as_u64)
                    .is_some_and(|target_id| target_id == other.id)
                    || {
                        let target_name = joint.get_string("target_name", "");
                        !target_name.is_empty() && target_name == other.name
                    }
            })
    })
}

fn physics_material(entity: &GameObject) -> PhysicsMaterial {
    let explicit_material = entity
        .get_component("PhysicsMaterial2D")
        .filter(|material| material.enabled);
    let collider_material = entity
        .get_component("Collider2D")
        .and_then(|collider| collider.get("material"))
        .and_then(serde_json::Value::as_object);
    let body_or_static = entity
        .get_component("Rigidbody2D")
        .or_else(|| entity.get_component("StaticBody2D"));
    let body_friction = body_or_static.map(|body| body.get_f64("friction", 0.25));
    let collider_friction = collider_material
        .and_then(|material| material.get("friction"))
        .and_then(serde_json::Value::as_f64);
    let body_bounce = body_or_static.map(|body| body.get_f64("bounciness", 0.0));
    let collider_bounce = collider_material
        .and_then(|material| material.get("bounciness"))
        .and_then(serde_json::Value::as_f64);
    let friction = explicit_material
        .map(|material| material.get_f64("friction", 0.25))
        .unwrap_or_else(|| choose_surface_value(body_friction, collider_friction, 0.25));
    let bounciness = explicit_material
        .map(|material| material.get_f64("bounciness", 0.0))
        .unwrap_or_else(|| choose_surface_value(body_bounce, collider_bounce, 0.0));
    let friction_combine = explicit_material
        .map(|material| material.get_string("friction_combine", "average"))
        .or_else(|| {
            collider_material
                .and_then(|material| material.get("friction_combine"))
                .and_then(serde_json::Value::as_str)
                .map(ToString::to_string)
        })
        .unwrap_or_else(|| "average".to_string());
    let bounce_combine = explicit_material
        .map(|material| material.get_string("bounce_combine", "maximum"))
        .or_else(|| {
            collider_material
                .and_then(|material| material.get("bounce_combine"))
                .and_then(serde_json::Value::as_str)
                .map(ToString::to_string)
        })
        .unwrap_or_else(|| "maximum".to_string());
    PhysicsMaterial {
        friction: finite(friction).clamp(0.0, 4.0),
        bounciness: finite(bounciness).clamp(0.0, 1.0),
        friction_combine: parse_material_combine(&friction_combine),
        bounce_combine: parse_material_combine(&bounce_combine),
    }
}

fn choose_surface_value(body: Option<f64>, collider: Option<f64>, default: f64) -> f64 {
    match (body, collider) {
        (Some(body), Some(collider)) => {
            let body_is_default = (body - default).abs() <= 1e-9;
            let collider_is_default = (collider - default).abs() <= 1e-9;
            match (body_is_default, collider_is_default) {
                (true, false) => collider,
                (false, true) => body,
                // Collider material is the more specific surface authoring
                // location when both sources intentionally override defaults.
                _ => collider,
            }
        }
        (Some(body), None) => body,
        (None, Some(collider)) => collider,
        (None, None) => default,
    }
}

fn parse_material_combine(value: &str) -> MaterialCombine {
    match value {
        "minimum" | "min" => MaterialCombine::Minimum,
        "maximum" | "max" => MaterialCombine::Maximum,
        "multiply" => MaterialCombine::Multiply,
        _ => MaterialCombine::Average,
    }
}

fn strongest_combine(first: MaterialCombine, second: MaterialCombine) -> MaterialCombine {
    use MaterialCombine::{Average, Maximum, Minimum, Multiply};
    match (first, second) {
        (Maximum, _) | (_, Maximum) => Maximum,
        (Multiply, _) | (_, Multiply) => Multiply,
        (Minimum, _) | (_, Minimum) => Minimum,
        _ => Average,
    }
}

fn combine_material_value(first: f64, second: f64, mode: MaterialCombine) -> f64 {
    match mode {
        MaterialCombine::Average => (first + second) * 0.5,
        MaterialCombine::Minimum => first.min(second),
        MaterialCombine::Maximum => first.max(second),
        MaterialCombine::Multiply => first * second,
    }
}

fn collider_is_trigger(entity: &GameObject) -> bool {
    entity
        .get_component("Area2D")
        .is_some_and(|area| area.enabled)
        || entity
            .get_component("Collider2D")
            .is_some_and(|collider| collider.get_bool("is_trigger", false))
}

fn collision_layer(entity: &GameObject) -> String {
    let Some(collider) = physics_shape_component(entity) else {
        return entity.layer.clone();
    };
    let layer = collider
        .get("collision_layer")
        .and_then(serde_json::Value::as_str)
        .or_else(|| collider.get("layer").and_then(serde_json::Value::as_str))
        .unwrap_or(&entity.layer)
        .to_string();
    if layer == "Default" && entity.layer != "Default" {
        entity.layer.clone()
    } else {
        layer
    }
}

fn collider_mask_allows(entity: &GameObject, other_layer: &str) -> bool {
    let Some(collider) = physics_shape_component(entity) else {
        return true;
    };
    let mut mask = collider.get_string_list("collision_mask");
    if mask.is_empty() {
        mask = collider.get_string_list("overlap_mask");
    }
    mask.is_empty()
        || mask.iter().any(|layer| {
            layer == "*"
                || layer == other_layer
                || (layer == "Default" && other_layer == entity.layer.as_str())
        })
}

fn physics_shape_component(entity: &GameObject) -> Option<&crate::engine::component::Component> {
    entity
        .get_component("Collider2D")
        .filter(|component| component.enabled)
        .or_else(|| {
            entity
                .get_component("Area2D")
                .filter(|component| component.enabled && component.get_bool("monitoring", true))
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

fn finite(value: f64) -> f64 {
    if value.is_finite() { value } else { 0.0 }
}

fn finite_vec(vector: Vec2) -> Vec2 {
    (finite(vector.0), finite(vector.1))
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

#[cfg(test)]
mod tests {
    use super::*;

    fn dynamic_body(name: &str, x: f64, y: f64) -> GameObject {
        let mut entity = GameObject::new(x, y, Some(name.to_string()));
        entity.add_component(
            crate::engine::component::default_component("Rigidbody2D")
                .expect("rigidbody component"),
        );
        let body = entity
            .get_component_mut("Rigidbody2D")
            .expect("rigidbody was added");
        body.set("use_gravity", serde_json::json!(false));
        body.set_f64("drag", 0.0);
        entity.sync_to_components();
        entity
    }

    #[test]
    fn raycast_uses_scaled_circle_radius_in_narrow_phase() {
        let mut circle = GameObject::new(3.0, 0.0, Some("ScaledCircle".to_string()));
        circle.scale_x = 4.0;
        circle.scale_y = 1.0;
        let collider = circle
            .get_component_mut("Collider2D")
            .expect("default game object has a collider");
        collider.set("shape", serde_json::json!("circle"));
        collider.set_f64("radius", 0.5);

        let hit = PhysicsSystem::new()
            .raycast(&[circle], (0.0, 0.0), (1.0, 0.0), 10.0)
            .expect("scaled circle should be hit");

        assert!((hit.distance - 1.0).abs() < 1e-9, "hit was {hit:?}");
        assert!((hit.point.0 - 1.0).abs() < 1e-9, "hit was {hit:?}");
    }

    #[test]
    fn adaptive_substeps_stop_fast_body_before_thin_wall() {
        let mut projectile = GameObject::new(0.0, 0.0, Some("Projectile".to_string()));
        projectile.add_component(
            crate::engine::component::default_component("Rigidbody2D")
                .expect("rigidbody component"),
        );
        let body = projectile
            .get_component_mut("Rigidbody2D")
            .expect("rigidbody was added");
        body.set("use_gravity", serde_json::json!(false));
        body.set("continuous_collision", serde_json::json!(true));
        body.set_f64("drag", 0.0);
        body.set_f64("velocity_x", 100.0);
        projectile.sync_to_components();

        let mut wall = GameObject::new(2.0, 0.0, Some("ThinWall".to_string()));
        let wall_collider = wall
            .get_component_mut("Collider2D")
            .expect("default collider");
        wall_collider.set("shape", serde_json::json!("rect"));
        wall_collider.set_f64("width", 0.2);
        wall_collider.set_f64("height", 10.0);
        wall.sync_to_components();

        let mut physics = PhysicsSystem::new();
        physics.fixed_delta = 0.005;
        physics.max_substeps = 16;
        physics.continuous_collision = true;
        let mut entities = vec![projectile, wall];
        physics.update_entities_mut(&mut entities, 0.05, "PLAY");

        assert!(
            entities[0].x < 1.5,
            "projectile tunneled to {}",
            entities[0].x
        );
        assert_eq!(
            entities[0]
                .get_component("Rigidbody2D")
                .expect("rigidbody")
                .get_f64("velocity_x", -1.0),
            0.0
        );
        assert_eq!(physics.stats.get("substeps"), Some(&10));
    }

    #[test]
    fn standalone_kinematic_body_moves_without_rigidbody() {
        let mut platform = GameObject::new(0.0, 0.0, Some("MovingPlatform".to_string()));
        platform.add_component(
            crate::engine::component::default_component("KinematicBody2D")
                .expect("kinematic component"),
        );
        platform
            .get_component_mut("KinematicBody2D")
            .expect("kinematic body")
            .set_f64("velocity_x", 3.0);
        let mut entities = vec![platform];

        PhysicsSystem::new().update_entities_mut(&mut entities, 1.0 / 60.0, "PLAY");

        assert!((entities[0].x - 0.05).abs() < 1e-9);
        assert_eq!(rigidbody_kind(&entities[0]), BodyKind::Kinematic);
    }

    #[test]
    fn collider_material_participates_in_restitution() {
        let mut ball = dynamic_body("Ball", 0.0, 0.0);
        ball.get_component_mut("Rigidbody2D")
            .expect("rigidbody")
            .set_f64("velocity_x", 10.0);
        let mut wall = GameObject::new(0.9, 0.0, Some("RubberWall".to_string()));
        wall.get_component_mut("Collider2D").expect("collider").set(
            "material",
            serde_json::json!({
                "friction": 0.0,
                "bounciness": 1.0,
                "bounce_combine": "maximum"
            }),
        );
        let mut entities = vec![ball, wall];

        PhysicsSystem::new().update_entities_mut(&mut entities, 1.0 / 120.0, "PLAY");

        assert!(
            entities[0]
                .get_component("Rigidbody2D")
                .expect("rigidbody")
                .get_f64("velocity_x", 0.0)
                < -9.0
        );
    }

    #[test]
    fn force_and_impulse_respect_body_mass_and_clear_accumulator() {
        let mut body = dynamic_body("Crate", 0.0, 0.0);
        body.get_component_mut("Rigidbody2D")
            .expect("rigidbody")
            .set_f64("mass", 2.0);
        let mut entities = vec![body];
        let mut physics = PhysicsSystem::new();
        let id = entities[0].id;
        assert!(physics.apply_force(&mut entities, id, (60.0, 0.0)));
        physics.update_entities_mut(&mut entities, 1.0 / 60.0, "PLAY");
        let velocity_after_force = entities[0]
            .get_component("Rigidbody2D")
            .expect("rigidbody")
            .get_f64("velocity_x", 0.0);
        assert!((velocity_after_force - 0.5).abs() < 1e-9);
        assert_eq!(
            entities[0]
                .get_component("Rigidbody2D")
                .expect("rigidbody")
                .get_f64("_force_x", -1.0),
            0.0
        );
        assert!(physics.apply_impulse(&mut entities, id, (2.0, 0.0)));
        assert!(
            (entities[0]
                .get_component("Rigidbody2D")
                .expect("rigidbody")
                .get_f64("velocity_x", 0.0)
                - 1.5)
                .abs()
                < 1e-9
        );
    }

    #[test]
    fn directional_force_field_accelerates_matching_dynamic_bodies() {
        let mut field = GameObject::new(0.0, 0.0, Some("Wind".to_string()));
        field.add_component(
            crate::engine::component::default_component("ForceField2D")
                .expect("force field component"),
        );
        let force = field
            .get_component_mut("ForceField2D")
            .expect("force field");
        force.set_f64("strength", 12.0);
        force.set_f64("radius", 10.0);
        force.set_f64("falloff", 0.0);
        let body = dynamic_body("Leaf", 2.0, 0.0);
        let mut entities = vec![field, body];

        PhysicsSystem::new().update_entities_mut(&mut entities, 1.0 / 60.0, "PLAY");

        assert!(
            (entities[1]
                .get_component("Rigidbody2D")
                .expect("rigidbody")
                .get_f64("velocity_x", 0.0)
                - 0.2)
                .abs()
                < 1e-9
        );
    }

    #[test]
    fn distance_joint_constrains_two_bodies_and_can_disable_collision() {
        let mut owner = dynamic_body("LinkedCrate", 0.0, 0.0);
        let target = GameObject::new(4.0, 0.0, Some("Anchor".to_string()));
        let target_id = target.id;
        owner.add_component(
            crate::engine::component::default_component("Joint2D").expect("joint component"),
        );
        let joint = owner.get_component_mut("Joint2D").expect("joint");
        joint.set("target_id", serde_json::json!(target_id));
        joint.set_f64("rest_length", 2.0);
        joint.set_f64("max_distance", 2.0);
        joint.set_f64("stiffness", 1.0);
        let mut entities = vec![owner, target];
        let mut physics = PhysicsSystem::new();

        physics.update_entities_mut(&mut entities, 1.0 / 60.0, "PLAY");

        assert!((entities[0].x - 2.0).abs() < 1e-9);
        assert_eq!(physics.stats.get("joints"), Some(&1));
        assert!(joint_collision_disabled(&entities[0], &entities[1]));
    }

    #[test]
    fn quiet_dynamic_body_automatically_sleeps_and_debug_snapshot_reports_it() {
        let body = dynamic_body("Sleeper", 0.0, 0.0);
        let mut entities = vec![body];
        let mut physics = PhysicsSystem::new();
        for _ in 0..31 {
            physics.update_entities_mut(&mut entities, 1.0 / 60.0, "PLAY");
        }

        assert!(
            entities[0]
                .get_component("Rigidbody2D")
                .expect("rigidbody")
                .get_bool("sleeping", false)
        );
        let snapshot = physics.debug_snapshot(&entities, 100);
        assert_eq!(snapshot.bodies.len(), 1);
        assert!(snapshot.bodies[0].sleeping);
        assert_eq!(snapshot.stats.get("sleeping_bodies"), Some(&1));
        let serialized = entities[0]
            .get_component("Rigidbody2D")
            .expect("rigidbody")
            .serialize();
        assert!(serialized.get("_sleep_timer").is_none());
    }
}
