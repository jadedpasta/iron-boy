// This file is part of Iron Boy, a CGB emulator.
// Copyright (C) 2023 Robert Hrusecky <jadedpastabowl@gmail.com>
//
// This program is free software: you can redistribute it and/or modify it under the terms of the
// GNU General Public License as published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See
// the GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License along with this program. If
// not, see <https://www.gnu.org/licenses/>.

use partial_borrow::prelude::*;

use crate::{interrupt::Interrupt, joypad::JoypadBus};

use super::CgbSystem;

impl JoypadBus for partial!(CgbSystem ! joypad, mut interrupt) {
    fn request_joypad_interrupt(&mut self) {
        self.interrupt.request(Interrupt::Joypad);
    }
}
