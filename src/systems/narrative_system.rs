use serde_json::{Value, json};

use crate::engine::game_api::GameAPI;
use crate::entities::game_object::GameObject;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct NarrativeEvent {
    pub kind: String,
    pub target_id: Option<u64>,
    pub speaker: String,
    pub text: String,
    pub choice_count: usize,
    pub quest_updated: bool,
    pub score_changes: Value,
}

#[derive(Debug, Clone, Default)]
pub struct NarrativeSystem {
    pub active_dialogue: Option<u64>,
    pub last_event: Option<NarrativeEvent>,
    pub interactions: usize,
    pub choices_made: usize,
}

impl NarrativeSystem {
    pub fn interact(&mut self, entities: &mut [GameObject]) -> Option<NarrativeEvent> {
        let player = entities
            .iter()
            .find(|entity| entity.enabled && entity.tag == "Player")?;
        let player_position = (player.x, player.y);
        let target_index = entities
            .iter()
            .enumerate()
            .filter(|(_, entity)| entity.enabled && entity.get_component("Interaction").is_some())
            .filter_map(|(index, entity)| {
                let interaction = entity.get_component("Interaction")?;
                let radius = interaction.get_f64("radius", 1.5).max(0.0);
                let dx = entity.x - player_position.0;
                let dy = entity.y - player_position.1;
                let distance = (dx * dx + dy * dy).sqrt();
                (distance <= radius).then_some((index, distance))
            })
            .min_by(|left, right| left.1.total_cmp(&right.1))
            .map(|(index, _)| index)?;

        let (target_id, target_name, event, quest) = {
            let target = &mut entities[target_index];
            let target_id = target.id;
            let target_name = target.name.clone();
            let interaction = target.get_component("Interaction")?.clone();
            let quest = interaction
                .get("quest_id")
                .and_then(Value::as_str)
                .zip(interaction.get("objective_id").and_then(Value::as_str))
                .map(|(quest, objective)| (quest.to_string(), objective.to_string()));
            let event = if let Some(dialogue) = target.get_component_mut("Dialogue") {
                let active = dialogue.get_bool("is_active", false);
                if !active {
                    dialogue.dialogue_reset();
                } else {
                    let _ = dialogue.dialogue_advance();
                }
                let speaker = dialogue.get_string("speaker", &target_name);
                let text = dialogue.dialogue_current_line();
                let choices = dialogue
                    .get("choices")
                    .and_then(Value::as_array)
                    .map(Vec::len)
                    .unwrap_or(0);
                self.active_dialogue = Some(target_id);
                NarrativeEvent {
                    kind: "dialogue".to_string(),
                    target_id: Some(target_id),
                    speaker,
                    text,
                    choice_count: choices,
                    quest_updated: false,
                    score_changes: json!({}),
                }
            } else {
                let text = interaction.get_string("prompt", &target_name);
                NarrativeEvent {
                    kind: "interaction".to_string(),
                    target_id: Some(target_id),
                    speaker: target_name.clone(),
                    text,
                    choice_count: 0,
                    quest_updated: false,
                    score_changes: json!({}),
                }
            };
            (target_id, target_name, event, quest)
        };

        let mut event = event;
        event.quest_updated = quest.as_ref().is_some_and(|(quest_id, objective_id)| {
            update_player_quest(entities, quest_id, objective_id)
        });
        let display = if event.speaker.is_empty() {
            event.text.clone()
        } else {
            format!("{}: {}", event.speaker, event.text)
        };
        GameAPI::set_ui_text_by_name(entities, "HUD_Dialogue", &display);
        GameAPI::set_ui_text_by_name(
            entities,
            "HUD_Status",
            &format!("Interacción con {target_name} · E para continuar"),
        );
        self.interactions += 1;
        self.last_event = Some(event.clone());
        let _ = target_id;
        Some(event)
    }

    pub fn choose(
        &mut self,
        entities: &mut [GameObject],
        choice_index: usize,
    ) -> Option<NarrativeEvent> {
        let target_id = self.active_dialogue?;
        let target_index = entities.iter().position(|entity| entity.id == target_id)?;
        let (speaker, choice) = {
            let dialogue = entities[target_index].get_component("Dialogue")?;
            let choice = dialogue
                .get("choices")
                .and_then(Value::as_array)?
                .get(choice_index)?
                .clone();
            (dialogue.get_string("speaker", ""), choice)
        };
        let label = choice
            .get("text")
            .or_else(|| choice.get("label"))
            .and_then(Value::as_str)
            .unwrap_or("Choice")
            .to_string();
        let mut changes = serde_json::Map::new();
        for (field, key) in [
            ("affection_delta", "affection"),
            ("honesty_delta", "honesty"),
            ("courage_delta", "courage"),
        ] {
            let delta = choice.get(field).and_then(Value::as_f64).unwrap_or(0.0);
            if delta != 0.0 {
                let next = add_story_score(entities, key, delta);
                changes.insert(key.to_string(), json!({"delta": delta, "value": next}));
            }
        }
        let affection = story_score(entities, "affection", 35.0);
        if let Some(hud) = entities
            .iter_mut()
            .find(|entity| entity.name == "HUD_Affection")
        {
            GameAPI::set_ui_progress(hud, affection, 100.0);
            if let Some(ui) = hud.get_component_mut("UIElement") {
                ui.set("text", json!(format!("Affection {affection:.0}")));
            }
        }
        let quest_updated = update_player_quest(entities, "letters_under_rain", "choice");
        let response = format!("{speaker}: {label}");
        GameAPI::set_ui_text_by_name(entities, "HUD_Dialogue", &response);
        GameAPI::set_ui_text_by_name(
            entities,
            "HUD_Status",
            &format!("Decisión registrada · affection {affection:.0}/100"),
        );
        if let Some(dialogue) = entities[target_index].get_component_mut("Dialogue") {
            dialogue.set("is_active", json!(false));
            dialogue.set("selected_choice", json!(choice_index));
        }
        self.active_dialogue = None;
        self.choices_made += 1;
        let event = NarrativeEvent {
            kind: "choice".to_string(),
            target_id: Some(target_id),
            speaker,
            text: label,
            choice_count: 0,
            quest_updated,
            score_changes: Value::Object(changes),
        };
        self.last_event = Some(event.clone());
        Some(event)
    }
}

fn update_player_quest(entities: &mut [GameObject], quest: &str, objective: &str) -> bool {
    entities
        .iter_mut()
        .find(|entity| entity.tag == "Player")
        .is_some_and(|player| {
            GameAPI::set_quest_objective_progress(player, quest, objective, json!(1))
        })
}

fn story_score(entities: &[GameObject], key: &str, default: f64) -> f64 {
    entities
        .iter()
        .find(|entity| entity.name == "StoryDirector")
        .map(|director| GameAPI::get_blackboard(director, key, json!(default)))
        .and_then(|value| value.as_f64())
        .unwrap_or(default)
}

fn add_story_score(entities: &mut [GameObject], key: &str, delta: f64) -> f64 {
    let current = story_score(entities, key, 0.0);
    let next = (current + delta).clamp(0.0, 100.0);
    if let Some(director) = entities
        .iter_mut()
        .find(|entity| entity.name == "StoryDirector")
    {
        GameAPI::set_blackboard(director, key, json!(next));
    }
    next
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::component::default_component;

    #[test]
    fn interaction_advances_dialogue_and_choice_updates_story_score() {
        let mut director = GameObject::new(0.0, 0.0, Some("StoryDirector".to_string()));
        director.add_component(default_component("Blackboard").expect("blackboard"));
        GameAPI::set_blackboard(&mut director, "affection", json!(35.0));
        let mut player = GameObject::new(0.0, 0.0, Some("Sol".to_string()));
        player.tag = "Player".to_string();
        player.add_component(default_component("QuestLog").expect("quest log"));
        GameAPI::add_quest(
            &mut player,
            "letters_under_rain",
            "Letters Under Rain",
            json!([{"id":"meet","progress":0,"target":1},{"id":"choice","progress":0,"target":1}]),
        );
        let mut mara = GameObject::new(1.0, 0.0, Some("Mara".to_string()));
        let mut interaction = default_component("Interaction").expect("interaction");
        interaction.set_f64("radius", 2.0);
        interaction.set("quest_id", json!("letters_under_rain"));
        interaction.set("objective_id", json!("meet"));
        mara.add_component(interaction);
        let mut dialogue = default_component("Dialogue").expect("dialogue");
        dialogue.set("speaker", json!("Mara"));
        dialogue.set("lines", json!(["You came back."]));
        dialogue.set(
            "choices",
            json!([{"text":"Tell the truth","affection_delta":15}]),
        );
        mara.add_component(dialogue);
        let mut hud = GameObject::new(0.0, 0.0, Some("HUD_Dialogue".to_string()));
        hud.add_component(default_component("UIElement").expect("ui"));
        let mut affection_hud = GameObject::new(0.0, 0.0, Some("HUD_Affection".to_string()));
        affection_hud.add_component(default_component("UIElement").expect("ui"));
        let mut entities = vec![director, player, mara, hud, affection_hud];
        let mut system = NarrativeSystem::default();

        let interaction = system.interact(&mut entities).expect("interaction");
        assert_eq!(interaction.speaker, "Mara");
        let choice = system.choose(&mut entities, 0).expect("choice");
        assert_eq!(choice.text, "Tell the truth");
        assert_eq!(story_score(&entities, "affection", 0.0), 50.0);
    }
}
