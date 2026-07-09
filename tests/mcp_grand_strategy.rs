use miniforge::engine::component::default_component;
use miniforge::engine::scene_manager::SceneManager;

#[test]
fn scene_manager_uses_configurable_start_scene() {
    let manager = SceneManager::new_with_start_scene("projects/Test", "campaign_1836");
    assert_eq!(manager.current_scene, "campaign_1836.scene");
    assert_eq!(manager.loaded_scenes, vec!["campaign_1836.scene"]);
}

#[test]
fn grand_strategy_components_are_registered() {
    for component_type in [
        "Province2D",
        "Nation2D",
        "PopulationPops2D",
        "Market2D",
        "Factory2D",
        "Diplomacy2D",
        "ResearchTree2D",
        "ArmyStack2D",
        "WarGoal2D",
        "TradeRoute2D",
    ] {
        assert!(
            default_component(component_type).is_some(),
            "{component_type} should be available to scenes and prefabs"
        );
    }
}
