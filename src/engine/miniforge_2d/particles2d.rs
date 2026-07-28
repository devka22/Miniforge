use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::engine::component::{Component, default_component};
use crate::systems::particle_system::ParticleEmitterConfig;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ParticleSpace2D {
    World,
    Local,
    Screen,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ParticleShape2D {
    Point,
    Circle { radius: f32 },
    Rect { width: f32, height: f32 },
    Cone { angle_degrees: f32, radius: f32 },
    Line { width: f32 },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ParticleBlendMode2D {
    Alpha,
    Additive,
    Multiply,
    Premultiplied,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ParticleSimulationTarget2D {
    CpuStable,
    GpuPreferred,
    /// Production compute simulation on WGPU with a generated CPU fallback.
    GpuCompute,
    /// Legacy serialized name kept for backwards compatibility.
    GpuRequiredFuture,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct RangeF32 {
    pub min: f32,
    pub max: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ParticleBurst2D {
    pub time: f32,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ParticleSpawn2D {
    pub looped: bool,
    pub rate_per_second: f32,
    pub max_particles: usize,
    pub shape: ParticleShape2D,
    #[serde(default)]
    pub bursts: Vec<ParticleBurst2D>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct ParticleVelocity2D {
    pub x: f32,
    pub y: f32,
    pub spread: f32,
    pub radial: f32,
    pub tangent: f32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct GradientStop2D {
    pub t: f32,
    pub color: [u8; 4],
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Gradient2D {
    #[serde(default)]
    pub stops: Vec<GradientStop2D>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct CurvePoint2D {
    pub t: f32,
    pub value: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Curve2D {
    #[serde(default)]
    pub points: Vec<CurvePoint2D>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct ParticleRotation2D {
    pub start_degrees: f32,
    pub velocity_degrees_per_second: f32,
    pub random_degrees: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TextureSheetAnimation2D {
    pub texture: String,
    pub columns: u32,
    pub rows: u32,
    pub fps: u32,
    pub looped: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ParticleCollision2D {
    pub enabled: bool,
    pub bounce: f32,
    pub friction: f32,
    #[serde(default)]
    pub layers: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct ParticleAttractor2D {
    pub x: f32,
    pub y: f32,
    pub strength: f32,
    pub radius: f32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct ParticleNoise2D {
    pub strength: f32,
    pub frequency: f32,
    pub seed: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ParticleTrail2D {
    pub enabled: bool,
    pub lifetime: f32,
    pub width: Curve2D,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ParticleRendererSettings2D {
    pub material: String,
    pub texture: Option<String>,
    pub blend_mode: ParticleBlendMode2D,
    pub sort_by_depth: bool,
    pub soft_particles: bool,
    pub gpu_instancing: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ParticleModule2D {
    pub name: String,
    pub kind: String,
    pub enabled: bool,
    #[serde(default)]
    pub params: BTreeMap<String, f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ParticleEmitter2D {
    pub name: String,
    pub enabled: bool,
    pub spawn: ParticleSpawn2D,
    pub lifetime: RangeF32,
    pub velocity: ParticleVelocity2D,
    pub acceleration: [f32; 2],
    pub color_over_life: Gradient2D,
    pub size_over_life: Curve2D,
    pub rotation: ParticleRotation2D,
    pub renderer: ParticleRendererSettings2D,
    pub texture_sheet: Option<TextureSheetAnimation2D>,
    pub collision: Option<ParticleCollision2D>,
    #[serde(default)]
    pub attractors: Vec<ParticleAttractor2D>,
    pub noise: Option<ParticleNoise2D>,
    pub trails: Option<ParticleTrail2D>,
    #[serde(default)]
    pub modules: Vec<ParticleModule2D>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ParticleLod2D {
    pub max_camera_distance: f32,
    pub rate_multiplier: f32,
    pub max_particles_multiplier: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ParticleEvent2D {
    pub name: String,
    pub trigger: String,
    pub payload: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ParticleSystem2D {
    pub name: String,
    pub space: ParticleSpace2D,
    pub simulation_target: ParticleSimulationTarget2D,
    pub simulation_rate: f32,
    pub warmup_seconds: f32,
    #[serde(default)]
    pub emitters: Vec<ParticleEmitter2D>,
    #[serde(default)]
    pub lods: Vec<ParticleLod2D>,
    #[serde(default)]
    pub events: Vec<ParticleEvent2D>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ParticleTemplate2D {
    pub name: String,
    pub category: String,
    pub description: String,
    pub system: ParticleSystem2D,
}

impl RangeF32 {
    pub fn new(min: f32, max: f32) -> Self {
        Self { min, max }
    }

    pub fn average(self) -> f32 {
        (self.min + self.max) * 0.5
    }
}

impl Gradient2D {
    pub fn from_colors(start: [u8; 4], end: [u8; 4]) -> Self {
        Self {
            stops: vec![
                GradientStop2D {
                    t: 0.0,
                    color: start,
                },
                GradientStop2D { t: 1.0, color: end },
            ],
        }
    }

    pub fn first_color(&self) -> [u8; 4] {
        self.stops
            .first()
            .map(|stop| stop.color)
            .unwrap_or([255, 255, 255, 255])
    }
}

impl Curve2D {
    pub fn constant(value: f32) -> Self {
        Self {
            points: vec![
                CurvePoint2D { t: 0.0, value },
                CurvePoint2D { t: 1.0, value },
            ],
        }
    }

    pub fn from_values(start: f32, end: f32) -> Self {
        Self {
            points: vec![
                CurvePoint2D {
                    t: 0.0,
                    value: start,
                },
                CurvePoint2D { t: 1.0, value: end },
            ],
        }
    }

    pub fn first_value(&self) -> f32 {
        self.points.first().map(|point| point.value).unwrap_or(1.0)
    }

    pub fn last_value(&self) -> f32 {
        self.points.last().map(|point| point.value).unwrap_or(1.0)
    }
}

impl ParticleSystem2D {
    pub fn estimate_max_particles(&self) -> usize {
        self.emitters
            .iter()
            .filter(|emitter| emitter.enabled)
            .map(ParticleEmitter2D::estimate_max_particles)
            .sum()
    }

    pub fn gpu_recommended(&self) -> bool {
        self.simulation_target != ParticleSimulationTarget2D::CpuStable
            || self.estimate_max_particles() > 2_000
            || self.emitters.iter().any(|emitter| {
                emitter.renderer.gpu_instancing
                    || emitter.noise.is_some()
                    || emitter.attractors.len() > 2
            })
    }

    pub fn validate(&self) -> Vec<String> {
        let mut issues = Vec::new();
        if self.name.trim().is_empty() {
            issues.push("ParticleSystem2D necesita nombre".to_string());
        }
        if self.emitters.is_empty() {
            issues.push(format!("{} no tiene emitters", self.name));
        }
        if self.simulation_rate <= 0.0 {
            issues.push(format!("{} tiene simulation_rate invalido", self.name));
        }
        for emitter in &self.emitters {
            issues.extend(emitter.validate(&self.name));
        }
        issues
    }

    pub fn to_runtime_emitter_config(&self) -> Option<ParticleEmitterConfig> {
        self.emitters
            .iter()
            .find(|emitter| emitter.enabled)
            .map(ParticleEmitter2D::to_runtime_emitter_config)
    }

    /// Builds the components consumed directly by the current runtime.
    ///
    /// GPU-ready systems receive both components so projects can switch
    /// renderer backends without rewriting gameplay or particle assets.
    pub fn to_runtime_components(&self) -> Vec<Component> {
        let Some(emitter) = self.emitters.iter().find(|emitter| emitter.enabled) else {
            return Vec::new();
        };
        let config = emitter.to_runtime_emitter_config();
        let mut cpu = default_component("ParticleEmitter")
            .expect("ParticleEmitter is a built-in runtime component");
        cpu.set("looped", json!(config.looped));
        cpu.set_f64("rate", config.rate);
        cpu.set("burst_count", json!(config.burst_count));
        cpu.set_f64("lifetime", config.lifetime);
        cpu.set_f64("velocity_x", config.velocity_x);
        cpu.set_f64("velocity_y", config.velocity_y);
        cpu.set_f64("spread", config.spread);
        cpu.set_f64("start_size", config.start_size);
        cpu.set_f64("end_size", config.end_size);
        cpu.set("color", json!(config.color));
        cpu.set("max_particles", json!(config.max_particles));
        cpu.set(
            "blend_mode",
            json!(particle_blend_mode_name(emitter.renderer.blend_mode)),
        );

        if !self.gpu_recommended() {
            return vec![cpu];
        }
        let mut gpu = default_component("GpuParticles2D")
            .expect("GpuParticles2D is a built-in runtime component");
        gpu.set("template", json!(self.name));
        gpu.set("max_particles", json!(config.max_particles));
        gpu.set("emission_rate", json!(config.rate));
        gpu.set("burst_count", json!(config.burst_count));
        gpu.set("lifetime", json!(config.lifetime));
        gpu.set("velocity_x", json!(config.velocity_x));
        gpu.set("velocity_y", json!(config.velocity_y));
        gpu.set("spread", json!(config.spread));
        gpu.set("gravity_x", json!(emitter.acceleration[0]));
        gpu.set("gravity_y", json!(emitter.acceleration[1]));
        gpu.set("start_size", json!(config.start_size));
        gpu.set("end_size", json!(config.end_size.max(0.25)));
        gpu.set("color", json!(config.color));
        gpu.set(
            "blend_mode",
            json!(particle_blend_mode_name(emitter.renderer.blend_mode)),
        );
        gpu.set("simulation", json!("compute"));
        gpu.set("fallback", json!("cpu_emitter"));
        gpu.set("local_space", json!(self.space == ParticleSpace2D::Local));
        vec![gpu, cpu]
    }
}

fn particle_blend_mode_name(mode: ParticleBlendMode2D) -> &'static str {
    match mode {
        ParticleBlendMode2D::Alpha => "alpha",
        ParticleBlendMode2D::Additive => "additive",
        ParticleBlendMode2D::Multiply => "multiply",
        ParticleBlendMode2D::Premultiplied => "premultiplied_alpha",
    }
}

impl ParticleEmitter2D {
    pub fn estimate_max_particles(&self) -> usize {
        let lifetime = self.lifetime.average().max(0.01);
        let streaming = (self.spawn.rate_per_second.max(0.0) * lifetime).ceil() as usize;
        let bursts = self
            .spawn
            .bursts
            .iter()
            .map(|burst| burst.count)
            .sum::<usize>();
        self.spawn.max_particles.max(streaming + bursts)
    }

    pub fn validate(&self, system_name: &str) -> Vec<String> {
        let mut issues = Vec::new();
        let path = format!("{system_name}.{}", self.name);
        if self.name.trim().is_empty() {
            issues.push(format!("{system_name} tiene un emitter sin nombre"));
        }
        if self.spawn.max_particles == 0 {
            issues.push(format!("{path} tiene max_particles=0"));
        }
        if self.spawn.rate_per_second < 0.0 {
            issues.push(format!("{path} tiene rate_per_second negativo"));
        }
        if self.lifetime.min <= 0.0 || self.lifetime.max < self.lifetime.min {
            issues.push(format!("{path} tiene lifetime invalido"));
        }
        if self.color_over_life.stops.is_empty() {
            issues.push(format!("{path} no tiene color_over_life"));
        }
        if self.size_over_life.points.is_empty() {
            issues.push(format!("{path} no tiene size_over_life"));
        }
        issues
    }

    pub fn to_runtime_emitter_config(&self) -> ParticleEmitterConfig {
        ParticleEmitterConfig {
            looped: self.spawn.looped,
            rate: self.spawn.rate_per_second as f64,
            burst_count: self.spawn.bursts.iter().map(|burst| burst.count).sum(),
            lifetime: self.lifetime.average() as f64,
            velocity_x: self.velocity.x as f64,
            velocity_y: self.velocity.y as f64,
            spread: self.velocity.spread as f64,
            start_size: self.size_over_life.first_value() as f64,
            end_size: self.size_over_life.last_value() as f64,
            color: self.color_over_life.first_color(),
            max_particles: self.spawn.max_particles,
        }
    }
}

pub fn particle_templates() -> Vec<ParticleTemplate2D> {
    vec![
        template(
            "Explosion2D",
            "combat",
            "Burst additive explosion with sparks and smoke.",
            ParticleSystem2D {
                name: "FX_Explosion2D".to_string(),
                simulation_target: ParticleSimulationTarget2D::GpuCompute,
                emitters: vec![
                    emitter(EmitterSpec2D {
                        name: "CoreFlash",
                        rate: 0.0,
                        bursts: vec![ParticleBurst2D {
                            time: 0.0,
                            count: 48,
                        }],
                        lifetime: RangeF32::new(0.18, 0.35),
                        velocity: ParticleVelocity2D {
                            x: 0.0,
                            y: -20.0,
                            spread: 180.0,
                            radial: 220.0,
                            tangent: 0.0,
                        },
                        color: Gradient2D::from_colors([255, 230, 120, 255], [255, 40, 0, 0]),
                        size: Curve2D::from_values(18.0, 2.0),
                        blend: ParticleBlendMode2D::Additive,
                    }),
                    emitter(EmitterSpec2D {
                        name: "SmokeRing",
                        rate: 12.0,
                        bursts: Vec::new(),
                        lifetime: RangeF32::new(1.1, 1.8),
                        velocity: ParticleVelocity2D {
                            x: 0.0,
                            y: -12.0,
                            spread: 70.0,
                            radial: 80.0,
                            tangent: 20.0,
                        },
                        color: Gradient2D::from_colors([90, 90, 90, 160], [30, 30, 30, 0]),
                        size: Curve2D::from_values(8.0, 36.0),
                        blend: ParticleBlendMode2D::Alpha,
                    }),
                ],
                ..system_defaults("FX_Explosion2D")
            },
        ),
        template(
            "Fire2D",
            "environment",
            "Looping flame for torches, hazards and ambience.",
            ParticleSystem2D {
                name: "FX_Fire2D".to_string(),
                emitters: vec![emitter(EmitterSpec2D {
                    name: "Flame",
                    rate: 64.0,
                    bursts: Vec::new(),
                    lifetime: RangeF32::new(0.55, 0.95),
                    velocity: ParticleVelocity2D {
                        x: 0.0,
                        y: -70.0,
                        spread: 28.0,
                        radial: 12.0,
                        tangent: 6.0,
                    },
                    color: Gradient2D::from_colors([255, 180, 30, 230], [150, 20, 5, 0]),
                    size: Curve2D::from_values(6.0, 18.0),
                    blend: ParticleBlendMode2D::Additive,
                })],
                ..system_defaults("FX_Fire2D")
            },
        ),
        template(
            "MagicAura2D",
            "ability",
            "Orbiting aura with attractor-friendly particles.",
            ParticleSystem2D {
                name: "FX_MagicAura2D".to_string(),
                simulation_target: ParticleSimulationTarget2D::GpuCompute,
                emitters: vec![ParticleEmitter2D {
                    attractors: vec![ParticleAttractor2D {
                        x: 0.0,
                        y: 0.0,
                        strength: 22.0,
                        radius: 96.0,
                    }],
                    noise: Some(ParticleNoise2D {
                        strength: 14.0,
                        frequency: 0.8,
                        seed: 1337,
                    }),
                    ..emitter(EmitterSpec2D {
                        name: "Aura",
                        rate: 96.0,
                        bursts: Vec::new(),
                        lifetime: RangeF32::new(1.0, 1.6),
                        velocity: ParticleVelocity2D {
                            x: 0.0,
                            y: -18.0,
                            spread: 24.0,
                            radial: 40.0,
                            tangent: 80.0,
                        },
                        color: Gradient2D::from_colors([120, 90, 255, 210], [40, 200, 255, 0]),
                        size: Curve2D::from_values(4.0, 1.0),
                        blend: ParticleBlendMode2D::Additive,
                    })
                }],
                ..system_defaults("FX_MagicAura2D")
            },
        ),
        template(
            "Rain2D",
            "weather",
            "Wide screen-space rain field.",
            ParticleSystem2D {
                name: "FX_Rain2D".to_string(),
                space: ParticleSpace2D::Screen,
                simulation_target: ParticleSimulationTarget2D::GpuCompute,
                emitters: vec![ParticleEmitter2D {
                    spawn: ParticleSpawn2D {
                        shape: ParticleShape2D::Rect {
                            width: 1600.0,
                            height: 64.0,
                        },
                        max_particles: 4_096,
                        rate_per_second: 1200.0,
                        looped: true,
                        bursts: Vec::new(),
                    },
                    renderer: ParticleRendererSettings2D {
                        material: "M_RainStreak".to_string(),
                        texture: Some("assets/particles/rain.png".to_string()),
                        blend_mode: ParticleBlendMode2D::Alpha,
                        sort_by_depth: false,
                        soft_particles: false,
                        gpu_instancing: true,
                    },
                    ..emitter(EmitterSpec2D {
                        name: "RainLines",
                        rate: 1200.0,
                        bursts: Vec::new(),
                        lifetime: RangeF32::new(0.7, 1.0),
                        velocity: ParticleVelocity2D {
                            x: -48.0,
                            y: 680.0,
                            spread: 12.0,
                            radial: 0.0,
                            tangent: 0.0,
                        },
                        color: Gradient2D::from_colors([150, 190, 255, 150], [150, 190, 255, 0]),
                        size: Curve2D::constant(2.0),
                        blend: ParticleBlendMode2D::Alpha,
                    })
                }],
                ..system_defaults("FX_Rain2D")
            },
        ),
        simple_template("HitSpark2D", "combat", 24, [255, 250, 190, 255]),
        simple_template("Dust2D", "movement", 18, [170, 140, 95, 170]),
        simple_template("MuzzleFlash2D", "combat", 12, [255, 240, 180, 255]),
        simple_template("PickupBurst2D", "ui_gameplay", 32, [80, 255, 150, 230]),
        simple_template("LevelUp2D", "ui_gameplay", 96, [255, 220, 80, 255]),
        simple_template("Snow2D", "weather", 256, [230, 245, 255, 180]),
    ]
}

pub fn minimal_particle_system() -> Value {
    json!(
        particle_templates()
            .into_iter()
            .find(|template| template.name == "Explosion2D")
            .map(|template| template.system)
            .unwrap_or_else(|| system_defaults("FX_Minimal"))
    )
}

fn system_defaults(name: &str) -> ParticleSystem2D {
    ParticleSystem2D {
        name: name.to_string(),
        space: ParticleSpace2D::World,
        simulation_target: ParticleSimulationTarget2D::CpuStable,
        simulation_rate: 60.0,
        warmup_seconds: 0.0,
        emitters: Vec::new(),
        lods: vec![
            ParticleLod2D {
                max_camera_distance: 512.0,
                rate_multiplier: 1.0,
                max_particles_multiplier: 1.0,
            },
            ParticleLod2D {
                max_camera_distance: 1024.0,
                rate_multiplier: 0.5,
                max_particles_multiplier: 0.5,
            },
        ],
        events: Vec::new(),
    }
}

fn template(
    name: &str,
    category: &str,
    description: &str,
    system: ParticleSystem2D,
) -> ParticleTemplate2D {
    ParticleTemplate2D {
        name: name.to_string(),
        category: category.to_string(),
        description: description.to_string(),
        system,
    }
}

fn simple_template(
    name: &str,
    category: &str,
    burst_count: usize,
    color: [u8; 4],
) -> ParticleTemplate2D {
    template(
        name,
        category,
        "Reusable burst preset for gameplay feedback.",
        ParticleSystem2D {
            name: format!("FX_{name}"),
            emitters: vec![emitter(EmitterSpec2D {
                name: "Burst",
                rate: 0.0,
                bursts: vec![ParticleBurst2D {
                    time: 0.0,
                    count: burst_count,
                }],
                lifetime: RangeF32::new(0.35, 0.8),
                velocity: ParticleVelocity2D {
                    x: 0.0,
                    y: -32.0,
                    spread: 90.0,
                    radial: 96.0,
                    tangent: 12.0,
                },
                color: Gradient2D::from_colors(color, [color[0], color[1], color[2], 0]),
                size: Curve2D::from_values(5.0, 0.0),
                blend: ParticleBlendMode2D::Additive,
            })],
            ..system_defaults(&format!("FX_{name}"))
        },
    )
}

struct EmitterSpec2D {
    name: &'static str,
    rate: f32,
    bursts: Vec<ParticleBurst2D>,
    lifetime: RangeF32,
    velocity: ParticleVelocity2D,
    color: Gradient2D,
    size: Curve2D,
    blend: ParticleBlendMode2D,
}

fn emitter(spec: EmitterSpec2D) -> ParticleEmitter2D {
    let burst_particles = spec.bursts.iter().map(|burst| burst.count).sum::<usize>();
    ParticleEmitter2D {
        name: spec.name.to_string(),
        enabled: true,
        spawn: ParticleSpawn2D {
            looped: spec.rate > 0.0,
            rate_per_second: spec.rate,
            max_particles: burst_particles
                .max((spec.rate.max(1.0) * spec.lifetime.max).ceil() as usize),
            shape: ParticleShape2D::Circle { radius: 6.0 },
            bursts: spec.bursts,
        },
        lifetime: spec.lifetime,
        velocity: spec.velocity,
        acceleration: [0.0, 0.0],
        color_over_life: spec.color,
        size_over_life: spec.size,
        rotation: ParticleRotation2D {
            start_degrees: 0.0,
            velocity_degrees_per_second: 90.0,
            random_degrees: 180.0,
        },
        renderer: ParticleRendererSettings2D {
            material: "M_Particle2D".to_string(),
            texture: Some("assets/particles/default.png".to_string()),
            blend_mode: spec.blend,
            sort_by_depth: false,
            soft_particles: false,
            gpu_instancing: false,
        },
        texture_sheet: None,
        collision: None,
        attractors: Vec::new(),
        noise: None,
        trails: None,
        modules: Vec::new(),
    }
}
