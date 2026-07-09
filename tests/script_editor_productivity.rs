use std::path::PathBuf;

use miniforge::engine::script_editor::{ScriptDocument, ScriptEditor};

#[test]
fn script_editor_builds_outline_and_stats_for_luau() {
    let mut editor = ScriptEditor {
        document: ScriptDocument {
            path: Some(PathBuf::from("Player.luau")),
            ..Default::default()
        },
        ..Default::default()
    };
    editor.set_text(
        "function on_start() end\nfunction on_update(dt: number)\n    move(1.0, 0.0)\nend",
    );

    assert!(editor.validate());
    assert_eq!(editor.outline.len(), 2);
    assert_eq!(editor.outline[0].name, "on_start");
    assert_eq!(editor.stats().functions, 2);
}

#[test]
fn script_editor_line_actions_keep_buffers_editable() {
    let mut editor = ScriptEditor::default();
    editor.set_text("function on_start()\nend");

    let (line, _) = editor.duplicate_line(0);
    assert_eq!(line, 1);
    assert!(editor.text().lines().count() >= 3);

    editor.toggle_line_comment(0);
    assert!(editor.lines[0].trim_start().starts_with("--"));

    editor.indent_line(0);
    assert!(editor.lines[0].starts_with("    "));

    editor.delete_line(0);
    assert!(!editor.lines.is_empty());
}

#[test]
fn script_editor_formats_json_and_reports_errors() {
    let mut editor = ScriptEditor {
        document: ScriptDocument {
            path: Some(PathBuf::from("Graph.mfgraph")),
            ..Default::default()
        },
        ..Default::default()
    };
    editor.set_text("{\"nodes\":[{\"id\":\"start\",\"type\":\"EventStart\"}]}");

    assert!(editor.format_json_pretty().unwrap());
    assert!(editor.text().contains('\n'));
    assert!(editor.validate());

    editor.set_text("{ broken");
    assert!(!editor.validate());
    assert!(!editor.diagnostics.is_empty());
}

#[test]
fn script_editor_offers_search_completions_minimap_and_refactors() {
    let mut editor = ScriptEditor {
        document: ScriptDocument {
            path: Some(PathBuf::from("Player.luau")),
            ..Default::default()
        },
        ..Default::default()
    };
    editor.set_text("function on_start()\n    local speed = 4\n    log(speed)\nend\n");
    assert!(editor.validate());

    let matches = editor.search_text("speed");
    assert_eq!(matches.len(), 2);
    assert_eq!(matches[0].line, 2);

    let completions = editor.completions_at(1, 7);
    assert!(
        completions
            .iter()
            .any(|completion| completion.label == "local")
    );
    let all_completions = editor.completions_at(0, 0);
    assert!(
        all_completions
            .iter()
            .any(|completion| completion.label == "set_ui_text")
    );

    assert_eq!(editor.rename_symbol("speed", "move_speed"), 2);
    assert!(editor.text().contains("move_speed"));

    let minimap = editor.minimap();
    assert!(minimap.iter().any(|line| line.kind == "function"));
    assert_eq!(editor.diagnostic_summary().errors, 0);
}

#[test]
fn script_editor_applies_code_actions_and_snippets() {
    let mut editor = ScriptEditor {
        document: ScriptDocument {
            path: Some(PathBuf::from("Graph.mfgraph")),
            ..Default::default()
        },
        ..Default::default()
    };
    editor.set_text("{\"nodes\":[{\"id\":\"start\",\"type\":\"EventStart\"},{\"id\":\"log\",\"type\":\"Log\"}]}");
    let action = editor
        .code_actions()
        .into_iter()
        .find(|action| action.kind == "blueprint.auto_layout")
        .unwrap();
    assert!(editor.apply_code_action(&action).unwrap());
    assert!(editor.text().contains("\"x\""));

    editor.document.path = Some(PathBuf::from("Player.luau"));
    editor.set_text("");
    assert!(editor.insert_snippet("log", 0, 0).is_some());
    assert!(editor.text().contains("log("));
    let end = editor.lines[0].len();
    assert!(editor.insert_snippet("quest", 0, end).is_some());
    assert!(editor.text().contains("add_quest("));
}

#[test]
fn script_editor_warns_about_broken_blueprint_links_and_luau_callbacks() {
    let mut graph = ScriptEditor {
        document: ScriptDocument {
            path: Some(PathBuf::from("Broken.mfgraph")),
            ..Default::default()
        },
        ..Default::default()
    };
    graph.set_text(
        r#"{"nodes":[{"id":"start","type":"EventStart","next":"missing"},{"id":"log","type":"Log"}]}"#,
    );
    assert!(graph.validate());
    assert_eq!(graph.diagnostic_summary().warnings, 1);
    assert!(
        graph
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("missing node"))
    );

    let mut luau = ScriptEditor {
        document: ScriptDocument {
            path: Some(PathBuf::from("Player.luau")),
            ..Default::default()
        },
        ..Default::default()
    };
    luau.set_text("function on_update()\n    spawn(\"Thing\", 1.0, 2.0)\nend");
    assert!(luau.validate());
    assert_eq!(luau.diagnostic_summary().warnings, 1);
    assert!(luau.diagnostic_summary().hints >= 1);
}
