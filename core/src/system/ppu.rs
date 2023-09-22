// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023 Robert Hrusecky <jadedpastabowl@gmail.com>
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
        self.mem.vram.bytes()
    }

    fn bg_palette_ram(&self) -> &Palettes {
        self.mem.bg_palette.palettes()
    }

    fn obj_palette_ram(&self) -> &Palettes {
        self.mem.obj_palette.palettes()
    }

    fn oam(&self) -> &OamBytes {
        &self.mem.oam
    }

    fn cgb_mode(&self) -> bool {
        *self.cgb_mode
    }
}
