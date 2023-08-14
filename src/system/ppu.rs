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

use crate::{
    interrupt::Interrupt,
    memory::{OamBytes, Palettes, VRamBytes},
    ppu::PpuBus,
};

use super::CgbSystem;

impl PpuBus for partial!(CgbSystem ! ppu, mut mem interrupt) {
    fn request_vblank_interrupt(&mut self) {
        self.interrupt.request(Interrupt::VBlank);
    }

    fn request_stat_interrupt(&mut self) {
        self.interrupt.request(Interrupt::Stat);
    }

    fn vram(&self) -> &VRamBytes {
        &self.mem.vram.bytes()
    }

    fn bg_palette_ram(&self) -> &Palettes {
        &self.mem.bg_palette.palettes()
    }

    fn obj_palette_ram(&self) -> &Palettes {
        &self.mem.obj_palette.palettes()
    }

    fn oam(&self) -> &OamBytes {
        &self.mem.oam
    }

    fn cgb_mode(&self) -> bool {
        *self.cgb_mode
    }
}
