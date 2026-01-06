use std::collections::{HashMap, HashSet};

use winit::keyboard::KeyCode;

#[derive(Default)]
pub struct InputState {
    pub pressed: HashSet<KeyCode>,
}
