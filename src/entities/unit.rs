use crate::entities::game_object::GameObject;

pub type Unit = GameObject;

pub fn new(x: f64, y: f64, name: Option<String>) -> Unit {
    GameObject::new_unit(x, y, name)
}
