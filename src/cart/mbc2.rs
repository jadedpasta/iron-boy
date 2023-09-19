// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023 Robert Hrusecky <jadedpastabowl@gmail.com>

use super::{mem::Mem, save::MbcSave, Mbc};

#[derive(Default)]
pub struct Mbc2 {
    rom_bank: u8,
    ram_enabled: bool,
}

impl Mbc2 {
    fn rom_bank_offset(&self) -> usize {
        let bank_num = if self.rom_bank == 0 { 1 } else { self.rom_bank };
        (bank_num as usize) << 14
    }

    fn rom_offset(&self, addr: u16) -> usize {
        let mut offset = (addr & 0x3fff) as usize;

        let upper_area = (addr & 0x4000) != 0;
        if upper_area {
            offset |= self.rom_bank_offset();
        }

        offset
    }

    fn ram_offset(&self, addr: u16) -> usize {
        (addr & 0x1fff) as usize
    }
}

impl Mbc for Mbc2 {
    fn read_low(&self, addr: u16, mem: &Mem) -> u8 {
        mem.rom.read(self.rom_offset(addr))
    }

    fn write_low(&mut self, addr: u16, val: u8, _mem: &mut Mem) {
        let reg_num = (addr >> 8) & 0x1;
        match reg_num {
            0 => self.ram_enabled = val & 0xf == 0xa,
            1 => self.rom_bank = val & 0x0f,
            _ => unreachable!(),
        }
    }

    fn read_high(&self, addr: u16, mem: &Mem) -> u8 {
        if self.ram_enabled {
            mem.ram.read(self.ram_offset(addr)) & 0x0f
        } else {
            0xff
        }
    }

    fn write_high(&mut self, addr: u16, val: u8, mem: &mut Mem) {
        if self.ram_enabled {
            mem.ram.write(self.ram_offset(addr), val & 0x0f);
        }
    }

    fn save(&self) -> MbcSave {
        MbcSave::None
    }
}
