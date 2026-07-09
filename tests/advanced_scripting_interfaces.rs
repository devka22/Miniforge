use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use miniforge::engine::game_api::GameAPI;
use miniforge::engine::luau_scripting::LuauScriptRuntime;
use miniforge::engine::miniforge_2d::blueprint::minimal_blueprint_graph;
use miniforge::engine::miniforge_2d::ui_designer::UiDesigner2D;
use miniforge::engine::miniforge_2d::ui_framework::{
    UiBinding2D, UiCanvas2D, UiWidget2D, main_menu_canvas,
};
use miniforge::engine::ui_advanced::{UiAdvancedInterface2D, UiBindingContext2D};
use miniforge::entities::game_object::GameObject;
use serde_json::json;

fn temp_dir(label: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("miniforge-advanced-{label}-{stamp}"));
    fs::create_dir_all(path.join("scripts")).unwrap();
    path
}

fn find_widget_mut<'a>(widgets: &'a mut [UiWidget2D], id: &str) -> Option<&'a mut UiWidget2D> {
    for widget in widgets {
        if widget.id == id {
            return Some(widget);
        }
        if let Some(found) = find_widget_mut(&mut widget.children, id) {
            return Some(found);
        }
    }
    None
}

#[test]
fn luau_context_persists_and_custom_events_keep_structured_payloads() {
    let root = temp_dir("luau-state");
    fs::write(
        root.join("scripts/Stateful.luau"),
        r#"
local ticks = 0

on_update = function(_dt)
    ticks += 1
    set_blackboard("ticks", ticks)
end

on_event = function(name, payload)
    set_blackboard("last_event", {
        name = name,
        payload = payload,
        tick_snapshot = ticks,
    })
end
"#,
    )
    .unwrap();

    let mut entity = GameObject::new(0.0, 0.0, Some("StatefulActor".to_string()));
    entity.script = Some("Stateful.luau".to_string());
    let mut entities = vec![entity];
    let mut runtime = LuauScriptRuntime::new(&root);

    runtime.update_entities(&mut entities, 1.0 / 60.0, "PLAY");
    runtime.update_entities(&mut entities, 1.0 / 60.0, "PLAY");
    assert_eq!(
        GameAPI::get_blackboard(&entities[0], "ticks", json!(0)),
        json!(2)
    );

    let entity_id = entities[0].id;
    let report = runtime.run_custom_event_for_entity(
        &mut entities,
        entity_id,
        "quest.updated",
        json!({"quest": "forge", "steps": [1, 2, 3]}),
    );
    assert_eq!(report.scripts_run, 1);
    assert_eq!(
        GameAPI::get_blackboard(&entities[0], "last_event", serde_json::Value::Null),
        json!({
            "name": "quest.updated",
            "payload": {"quest": "forge", "steps": [1, 2, 3]},
            "tick_snapshot": 2,
        })
    );
    let snapshot = runtime.debug_snapshot(&root, &entities);
    assert_eq!(snapshot.persistent_contexts, 1);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn blueprint_palette_is_context_sensitive_and_links_are_rewireable() {
    let mut graph = minimal_blueprint_graph();
    let suggestions = graph
        .connection_suggestions("begin_play", "then", "branch", 8)
        .unwrap();
    let branch = suggestions
        .iter()
        .find(|suggestion| suggestion.node_kind == "Branch")
        .expect("Branch should accept an exec output");
    assert_eq!(branch.compatible_pin, "exec");
    assert_eq!(branch.direction, "in");

    let branch_id = graph
        .add_node("Branch", "Branch", 600.0, 0.0, json!({}))
        .unwrap();
    assert!(
        graph
            .connect_nodes_checked("print_ready", "then", &branch_id, "exec")
            .unwrap()
    );
    assert_eq!(graph.break_pin_links(&branch_id, "exec"), 1);
    assert!(
        !graph
            .edges
            .iter()
            .any(|edge| edge.to == branch_id && edge.to_pin == "exec")
    );
}

#[test]
fn advanced_ui_binds_localizes_resolves_and_audits_the_interface() {
    let mut canvas: UiCanvas2D = main_menu_canvas("MiniForge");
    let title = find_widget_mut(&mut canvas.widgets, "TitleText").unwrap();
    title.bindings.push(UiBinding2D {
        property: "text".to_string(),
        source_path: "player.title|uppercase".to_string(),
        fallback: json!("MINIFORGE"),
    });
    let continue_button = find_widget_mut(&mut canvas.widgets, "ContinueButton").unwrap();
    continue_button.bindings.push(UiBinding2D {
        property: "text".to_string(),
        source_path: "@i18n:menu.continue".to_string(),
        fallback: json!("Continue"),
    });

    let advanced = UiAdvancedInterface2D {
        bindings: UiBindingContext2D {
            state: json!({"player": {"title": "iron skies"}}),
            locale: "es".to_string(),
            translations: [(
                "es".to_string(),
                [("menu.continue".to_string(), "Continuar".to_string())]
                    .into_iter()
                    .collect(),
            )]
            .into_iter()
            .collect(),
        },
        ..Default::default()
    };

    let prepared = advanced.prepare(&canvas, (1280.0, 720.0));
    assert_eq!(prepared.layout.breakpoint, "desktop");
    assert_eq!(prepared.bindings.applied, 2);
    assert_eq!(
        prepared.canvas.find_widget("TitleText").unwrap().properties["text"],
        json!("IRON SKIES")
    );
    assert_eq!(
        prepared
            .canvas
            .find_widget("ContinueButton")
            .unwrap()
            .properties["text"],
        json!("Continuar")
    );
    assert_eq!(prepared.accessibility.errors(), 0);
    assert!(
        prepared
            .accessibility
            .focus_order
            .contains(&"ContinueButton".to_string())
    );

    let designer = UiDesigner2D::main_menu("MiniForge");
    assert_eq!(designer.accessibility_report().errors(), 0);
}
