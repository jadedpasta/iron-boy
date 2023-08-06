use partial_borrow::prelude::*;

use crate::{interrupt::Interrupt, joypad::JoypadBus};

use super::CgbSystem;

impl JoypadBus for partial!(CgbSystem ! joypad, mut interrupt) {
    fn request_joypad_interrupt(&mut self) {
        self.interrupt.request(Interrupt::Joypad);
    }
}
