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

#[derive(Debug, Clone, Copy)]
pub enum Interrupt {
    VBlank = 0,
    Stat,
    Timer,
    #[allow(unused)]
    Serial,
    Joypad,
}

pub struct InterruptState {
    pub enable: u8,
    pub flags: u8,
}

impl InterruptState {
    pub fn new() -> Self {
        Self { enable: 0, flags: 0 }
    }

    pub fn request(&mut self, interrupt: Interrupt) {
        self.flags |= 1 << interrupt as usize;
    }

    fn pending_bits(&self) -> u8 {
        self.enable & self.flags
    }

    pub fn pending(&self) -> bool {
        self.pending_bits() != 0
    }

    pub fn pop(&mut self) -> Option<u8> {
        let bit = self.pending_bits().trailing_zeros() as u8;
        if bit > 7 {
            // No interrupts are pending.
            return None;
        }
        // Toggle off the flag bit to mark the interrupt as handled.
        self.flags ^= 1 << bit;
        Some(bit)
    }
}
