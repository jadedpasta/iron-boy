// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023 Robert Hrusecky <jadedpastabowl@gmail.com>
use partial_borrow::prelude::*;

use crate::{dma::DmaBus, memory::OamBytes};

use super::{CgbSystem, BOOT_ROM};

impl DmaBus for partial!(CgbSystem ! dma, mut mem) {
    fn write_vram(&mut self, addr: u16, val: u8) {
        self.mem.vram.write(addr, val, *self.cgb_mode);
    }

    fn oam_mut(&mut self) -> &mut OamBytes {
        &mut self.mem.oam
    }

    fn read_8(&self, addr: u16) -> u8 {
        match (addr >> 8) as u8 {
            0x00..=0x00 | 0x02..=0x08 if *self.boot_rom_mapped => BOOT_ROM[addr as usize],
            0x00..=0x7f => self.cart.read_low(addr),
            0x80..=0x9f => self.mem.vram.read(addr, *self.cgb_mode),
            0xa0..=0xbf => self.cart.read_high(addr),
            0xc0..=0xcf | 0xe0..=0xef => self.mem.wram.read_low(addr),
            0xd0..=0xdf | 0xf0..=0xff => self.mem.wram.read_high(addr, *self.cgb_mode),
        }
    }
}
