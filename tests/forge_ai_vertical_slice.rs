use std::collections::{BTreeMap, BTreeSet};

use miniforge::engine::forge_ai::{
    AiEditorHost, AiExecutionOptions, AiExecutor, AiFileChange, AiHostValidation, AiProjectContext,
    AiProvider, AiTestReport, AiTestStatus, AiTestSuite, AiValidator, ForgeAiAgent, ForgeAiRequest,
    LocalRuleProvider, ProjectDoctor,
};
use serde_json::Value;

#[derive(Debug, Default)]
struct TestHost {
    next_id: u64,
    entities: BTreeMap<u64, TestEntity>,
    files: BTreeMap<String, String>,
    prefabs: Vec<String>,
}

#[derive(Debug, Clone, Default)]
struct TestEntity {
    id: u64,
    name: String,
    components: BTreeMap<String, BTreeMap<String, Value>>,
    tag: String,
}

impl TestHost {
    fn entity(&self, name: &str) -> &TestEntity {
        self.entities
            .values()
            .find(|entity| entity.name == name)
            .expect("entity exists")
    }
}

impl AiEditorHost for TestHost {
    fn find_entity_id(&self, name: &str) -> miniforge::engine::forge_ai::AiResult<Option<u64>> {
        Ok(self
            .entities
            .values()
            .find(|entity| entity.name == name)
            .map(|entity| entity.id))
    }

    fn create_entity(
        &mut self,
        name: &str,
        _x: f64,
        _y: f64,
    ) -> miniforge::engine::forge_ai::AiResult<u64> {
        self.next_id += 1;
        let id = self.next_id;
        self.entities.insert(
            id,
            TestEntity {
                id,
                name: name.to_string(),
                tag: "Untagged".to_string(),
                ..TestEntity::default()
            },
        );
        Ok(id)
    }

    fn add_component(
        &mut self,
        entity_id: u64,
        component_type: &str,
    ) -> miniforge::engine::forge_ai::AiResult<()> {
        self.entities
            .get_mut(&entity_id)
            .expect("entity")
            .components
            .entry(component_type.to_string())
            .or_default();
        Ok(())
    }

    fn set_component_property(
        &mut self,
        entity_id: u64,
        component_type: &str,
        key: &str,
        value: Value,
    ) -> miniforge::engine::forge_ai::AiResult<Value> {
        let entity = self.entities.get_mut(&entity_id).expect("entity");
        if component_type == "Identity" && key == "tag" {
            let previous = Value::String(entity.tag.clone());
            entity.tag = value.as_str().unwrap_or("Untagged").to_string();
            return Ok(previous);
        }
        let previous = entity
            .components
            .entry(component_type.to_string())
            .or_default()
            .insert(key.to_string(), value)
            .unwrap_or(Value::Null);
        Ok(previous)
    }

    fn write_project_file(
        &mut self,
        relative_path: &str,
        contents: &str,
    ) -> miniforge::engine::forge_ai::AiResult<AiFileChange> {
        let created = !self.files.contains_key(relative_path);
        self.files
            .insert(relative_path.to_string(), contents.to_string());
        Ok(AiFileChange {
            relative_path: relative_path.to_string(),
            created,
            bytes_written: contents.len(),
        })
    }

    fn create_prefab(
        &mut self,
        _entity_id: u64,
        prefab_name: &str,
    ) -> miniforge::engine::forge_ai::AiResult<String> {
        let path = format!("assets/prefabs/{prefab_name}");
        self.prefabs.push(path.clone());
        Ok(path)
    }

    fn validate_project(&mut self) -> miniforge::engine::forge_ai::AiResult<AiHostValidation> {
        Ok(AiHostValidation::default())
    }

    fn run_ai_test(
        &mut self,
        suite_id: &str,
    ) -> miniforge::engine::forge_ai::AiResult<AiTestReport> {
        Ok(AiTestReport {
            suite_id: suite_id.to_string(),
            status: AiTestStatus::Passed,
            cases_run: AiTestSuite::enemy_smoke().cases.len(),
            failures: Vec::new(),
            replay_path: None,
        })
    }

    fn analyze_performance(&mut self) -> miniforge::engine::forge_ai::AiResult<Vec<String>> {
        Ok(vec!["test host performance snapshot".to_string()])
    }
}

#[test]
fn forge_ai_enemy_vertical_slice_generates_and_executes_typed_actions() {
    let provider = LocalRuleProvider::default();
    let context = AiProjectContext::default();
    let plan = provider
        .generate_plan(
            "Crea un enemigo 2D con vida, patrulla, persecucion y ataque al jugador.",
            &context,
        )
        .unwrap();
    let validation = AiValidator::validate_plan(&plan);
    assert!(validation.valid, "{:?}", validation.errors);
    assert!(
        plan.actions
            .iter()
            .any(|action| action.action_type() == "CreateLuauScript")
    );

    let executor = AiExecutor::default();
    let mut host = TestHost::default();
    let report = executor.execute_plan(
        &mut host,
        &plan,
        &AiExecutionOptions {
            approved: true,
            dry_run: false,
            continue_on_error: false,
        },
    );

    assert!(report.success, "{:?}", report.errors);
    assert_eq!(host.files.len(), 1);
    assert!(host.files["scripts/enemy_controller.luau"].contains("---@export"));
    assert_eq!(host.prefabs, vec!["assets/prefabs/Enemy2D.prefab"]);
    let enemy = host.entity("Enemy2D");
    let component_names = enemy.components.keys().cloned().collect::<BTreeSet<_>>();
    for required in [
        "Health",
        "AIController",
        "NavAgent",
        "Rigidbody2D",
        "ScriptComponent",
        "Light2D",
    ] {
        assert!(component_names.contains(required), "missing {required}");
    }
    assert_eq!(enemy.tag, "Enemy");
}

#[test]
fn forge_ai_agent_dry_run_returns_visible_plan_without_mutating_host() {
    let mut host = TestHost::default();
    let agent = ForgeAiAgent::default();
    let response = agent
        .run(
            &mut host,
            ForgeAiRequest {
                instruction: "Crea un enemigo 2D con vida, patrulla y ataque.".to_string(),
                approved: false,
                dry_run: true,
            },
            AiProjectContext::default(),
        )
        .unwrap();

    assert!(response.execution.success);
    assert!(response.execution.dry_run);
    assert!(response.execution.previews.len() >= 5);
    assert!(host.entities.is_empty());
}

#[test]
fn project_doctor_and_luau_docs_cover_recommendation_surface() {
    let diagnostics = ProjectDoctor::analyze(&AiProjectContext::default());
    assert!(diagnostics.iter().any(|item| item.code == "empty_scene"));

    let docs = AiValidator::api_doc();
    assert!(
        docs.classes
            .iter()
            .any(|class| class.name == "Navigation2D")
    );
    assert!(
        docs.globals
            .iter()
            .any(|symbol| symbol.signature.contains("set_component_number"))
    );
}
