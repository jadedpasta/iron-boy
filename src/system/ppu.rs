use partial_borrow::prelude::*;

use crate::{
    interrupt::Interrupt,
    memory::{OamBytes, Palettes, VRamBytes},
    ppu::PpuBus,
};

use super::CgbSystem;

impl PpuBus for partial!(CgbSystem ! ppu, mut mem interrupt) {
    fn trigger_vblank_interrupt(&mut self) {
        self.interrupt.request(Interrupt::VBlank);
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
