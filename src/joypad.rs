// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023 Robert Hrusecky <jadedpastabowl@gmail.com>
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

pub trait JoypadBus {
    fn request_joypad_interrupt(&mut self);
}

pub struct Joypad {
    state: u8,
    p1: u8,
}

impl Joypad {
    pub fn new() -> Self {
        Self { state: 0, p1: 0 }
    }

    pub fn handle(&mut self, button: Button, state: ButtonState, bus: &mut impl JoypadBus) {
        let button = 1 << button as u8;
        match state {
            ButtonState::Pressed => {
                self.state |= button;
                bus.request_joypad_interrupt();
            }
            ButtonState::Released => self.state &= !button,
        }
    }

    fn direction_bits(&self) -> u8 {
        self.state & 0x0f
    }

    fn action_bits(&self) -> u8 {
        (self.state >> 4) & 0x0f
    }

    pub fn p1(&self) -> u8 {
        let mut bits = 0;
        if (self.p1 >> 4) & 0x1 == 0 {
            bits |= self.direction_bits();
        }
        if (self.p1 >> 5) & 0x1 == 0 {
            bits |= self.action_bits();
        }

        self.p1 & 0xf0 | !bits & 0x0f
    }

    pub fn set_p1(&mut self, p1: u8) {
        self.p1 &= 0x0f;
        self.p1 |= p1 & 0xf0;
    }
}
