// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023 Robert Hrusecky <jadedpastabowl@gmail.com>

use super::{Mbc, mem::Mem};

#[derive(Default)]
pub struct Simple;

impl Mbc for Simple {
    fn read_low(&self, addr: u16, mem: &Mem) -> u8 {
        mem.rom.read(addr as usize)
    }

    fn write_low(&mut self, _addr: u16, _val: u8, _mem: &mut Mem) {}

    fn read_high(&self, addr: u16, mem: &Mem) -> u8 {
        mem.ram.read(addr as usize)
    }

    fn write_high(&mut self, addr: u16, val: u8, mem: &mut Mem) {
        mem.ram.write(addr as usize, val)
    }
}
