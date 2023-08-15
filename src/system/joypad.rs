// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023 Robert Hrusecky <jadedpastabowl@gmail.com>
use partial_borrow::prelude::*;

use crate::{interrupt::Interrupt, joypad::JoypadBus};

use super::CgbSystem;

impl JoypadBus for partial!(CgbSystem ! joypad, mut interrupt) {
    fn request_joypad_interrupt(&mut self) {
        self.interrupt.request(Interrupt::Joypad);
    }
}
