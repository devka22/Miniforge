use crate::input::input_handler::InputHandler;

#[derive(Debug, Clone, Default)]
pub struct InputSystem {
    pub handler: InputHandler,
}

impl InputSystem {
    pub fn update(&mut self) {
        self.handler.update();
    }
}
