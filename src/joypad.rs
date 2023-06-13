use winit::event::{ElementState, VirtualKeyCode};

pub enum Button {
    Right = 0,
    Left,
    Up,
    Down,
    A,
    B,
    Select,
    Start,
}

impl Button {
    pub fn from_keycode(key: VirtualKeyCode) -> Option<Self> {
        use VirtualKeyCode::*;
        match key {
            W => Some(Self::Up),
            A => Some(Self::Left),
            S => Some(Self::Down),
            D => Some(Self::Right),
            LBracket => Some(Self::Start),
            RBracket => Some(Self::Select),
            Comma => Some(Self::A),
            Period => Some(Self::B),
            _ => None,
        }
    }
}

pub enum ButtonState {
    Pressed,
    Released,
}

impl ButtonState {
    pub fn from_state(state: ElementState) -> Self {
        match state {
            ElementState::Pressed => Self::Pressed,
            ElementState::Released => Self::Released,
        }
    }
}

pub struct JoypadState(u8);

impl JoypadState {
    pub fn new() -> Self {
        Self(0)
    }

    pub fn handle(&mut self, button: Button, state: ButtonState) {
        let button = 1 << button as u8;
        match state {
            ButtonState::Pressed => self.0 |= button,
            ButtonState::Released => self.0 &= !button,
        }
    }

    pub fn direction_bits(&self) -> u8 {
        self.0 & 0x0f
    }

    pub fn action_bits(&self) -> u8 {
        (self.0 >> 4) & 0x0f
    }
}
