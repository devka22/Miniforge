use crate::engine::component::Component;

pub struct ComponentValidation;

impl ComponentValidation {
    pub fn repair_component(component: &mut Component) -> bool {
        let mut changed = false;
        match component.component_type.as_str() {
            "AudioSource" => {
                changed |= clamp(component, "volume", 0.0, 1.0);
                changed |= clamp(component, "spatial_blend", 0.0, 1.0);
                changed |= clamp(component, "pitch", 0.1, 4.0);
            }
            "Rigidbody2D" => {
                changed |= min_value(component, "mass", 0.0001);
                changed |= min_value(component, "drag", 0.0);
                changed |= min_value(component, "angular_drag", 0.0);
                changed |= clamp(component, "friction", 0.0, 1.0);
                changed |= clamp(component, "bounciness", 0.0, 1.0);
                let body_type = component.get_string("body_type", "dynamic");
                if !matches!(body_type.as_str(), "dynamic" | "static" | "kinematic") {
                    component.set("body_type", serde_json::json!("dynamic"));
                    changed = true;
                }
            }
            "Collider2D" => {
                changed |= min_value(component, "width", 0.001);
                changed |= min_value(component, "height", 0.001);
                changed |= min_value(component, "radius", 0.001);
            }
            "RTSMovement" => {
                changed |= min_value(component, "speed", 0.0);
                changed |= min_value(component, "acceleration", 0.0);
                changed |= min_value(component, "turn_speed", 0.0);
            }
            "Vision" => {
                changed |= min_value(component, "radius", 0.0);
            }
            "ProductionQueue" => {
                changed |= min_value(component, "max_queue", 0.0);
                changed |= min_value(component, "production_speed", 0.0);
            }
            "Buildable" => {
                changed |= min_value(component, "footprint_w", 1.0);
                changed |= min_value(component, "footprint_h", 1.0);
                changed |= min_value(component, "build_time", 0.01);
            }
            "ConstructionSite" => {
                changed |= min_value(component, "build_time", 0.01);
                changed |= min_value(component, "build_rate", 0.0);
                changed |= clamp(
                    component,
                    "progress",
                    0.0,
                    component.get_f64("build_time", 8.0).max(0.01),
                );
            }
            "UIElement" => {
                changed |= min_value(component, "width", 1.0);
                changed |= min_value(component, "height", 1.0);
                changed |= clamp(component, "opacity", 0.0, 1.0);
                changed |= clamp(
                    component,
                    "progress",
                    0.0,
                    component.get_f64("max_progress", 1.0),
                );
            }
            _ => {}
        }
        changed
    }
}

fn clamp(component: &mut Component, key: &str, min: f64, max: f64) -> bool {
    let current = component.get_f64(key, min);
    let next = current.clamp(min, max);
    if (current - next).abs() > f64::EPSILON {
        component.set_f64(key, next);
        true
    } else {
        false
    }
}

fn min_value(component: &mut Component, key: &str, min: f64) -> bool {
    let current = component.get_f64(key, min);
    if current < min {
        component.set_f64(key, min);
        true
    } else {
        false
    }
}
