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

use std::mem::{self, MaybeUninit};

pub struct WorkRam {
    low: [u8; 0x1000],
    high: [[u8; 0x1000]; 7],
    pub svbk: u8,
}

impl WorkRam {
    fn bank(&self, cgb_mode: bool) -> usize {
        if !cgb_mode || self.svbk == 0 {
            0
        } else {
            (self.svbk - 1) as usize & 0x3
        }
    }

    pub fn read_low(&self, addr: u16) -> u8 {
        self.low[addr as usize & 0xfff]
    }

    pub fn read_high(&self, addr: u16, cgb_mode: bool) -> u8 {
        self.high[self.bank(cgb_mode)][addr as usize & 0xfff]
    }

    pub fn write_low(&mut self, addr: u16, val: u8) {
        self.low[addr as usize & 0xfff] = val;
    }

    pub fn write_high(&mut self, addr: u16, val: u8, cgb_mode: bool) {
        self.high[self.bank(cgb_mode)][addr as usize & 0xfff] = val;
    }
}

pub type VRamBytes = [[u8; 0x2000]; 2];

pub struct VideoRam {
    vram: VRamBytes,
    pub vbk: u8,
}

impl VideoRam {
    pub fn bank(&self, cgb_mode: bool) -> usize {
        if cgb_mode {
            self.vbk as usize & 0x1
        } else {
            0
        }
    }

    pub fn read(&self, addr: u16, cgb_mode: bool) -> u8 {
        self.vram[self.bank(cgb_mode)][addr as usize & 0x1fff]
    }

    pub fn write(&mut self, addr: u16, val: u8, cgb_mode: bool) {
        self.vram[self.bank(cgb_mode)][addr as usize & 0x1fff] = val;
    }

    pub fn bytes(&self) -> &VRamBytes {
        &self.vram
    }
}

pub type Color = [u8; 2];
pub type Palette = [Color; 4];
pub type Palettes = [Palette; 8];

pub struct PaletteRam {
    ram: [u8; 64],
    pub select: u8,
}

impl PaletteRam {
    fn index(&self) -> usize {
        (self.select & 0x3f) as usize
    }

    pub fn read_data(&self) -> u8 {
        self.ram[self.index()]
    }

    pub fn write_data(&mut self, val: u8) {
        self.ram[self.index()] = val;
        self.select = (self.select & 0xc0) | self.select.wrapping_add(self.select >> 7) & 0x3f;
    }

    pub fn palettes(&self) -> &Palettes {
        unsafe { mem::transmute(&self.ram) }
    }
}

pub type OamBytes = [u8; 0xa0];

pub struct MemoryData {
    pub cartrige_rom: [u8; 0x8000], // TODO: MBCs
    pub vram: VideoRam,
    pub cartrige_ram: [u8; 0x2000], // TODO: MBCs
    pub wram: WorkRam,
    // echo_ram: mirror of 0xc000~0xddff
    pub oam: OamBytes,
    // prohibited_area: 0xfea0~0xfeff
    pub hram: [u8; 0x7f],
    pub bg_palette: PaletteRam,
    pub obj_palette: PaletteRam,
}

impl MemoryData {
    pub fn new(rom: impl Into<Vec<u8>>) -> Self {
        // SAFTEY: All zeros is valid for MemoryData, which is just a bunch of nested arrays of u8
        let mut mem = unsafe { MaybeUninit::<MemoryData>::zeroed().assume_init() };

        let mut rom = rom.into();
        rom.resize(mem::size_of_val(&mem.cartrige_rom), 0);
        mem.cartrige_rom.copy_from_slice(&rom[..]);

        mem
    }
}
