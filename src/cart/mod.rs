// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023 Robert Hrusecky <jadedpastabowl@gmail.com>

use ambassador::{delegatable_trait, Delegate};

use self::{
    mbc1::Mbc1,
    mbc2::Mbc2,
    mbc3::Mbc3,
    mem::{Mem, OptionalSegment, Segment},
    simple::Simple,
};

mod mbc1;
mod mbc2;
mod mbc3;
mod mem;
mod rtc;
mod simple;

#[delegatable_trait]
pub trait Mbc {
    fn read_low(&self, addr: u16, mem: &Mem) -> u8;
    fn write_low(&mut self, addr: u16, val: u8, mem: &mut Mem);
    fn read_high(&self, addr: u16, mem: &Mem) -> u8;
    fn write_high(&mut self, addr: u16, val: u8, mem: &mut Mem);
}

#[derive(Delegate)]
#[delegate(Mbc)]
pub enum AnyMbc {
    Simple(Simple),
    Mbc1(Mbc1),
    Mbc2(Mbc2),
    Mbc3(Mbc3),
}

fn header(rom: &[u8]) -> (u8, usize, usize) {
    let cart_type = rom[0x147];
    let rom_size = match rom[0x148] {
        id @ 0x0..=0x8 => 1 << (id + 15),
        id => panic!("Unknown ROM size ID: {id:x}"),
    };
    let ram_size = match rom[0x149] {
        0x00 => 0,
        0x02 => 0x2000,
        0x03 => 0x8000,
        0x04 => 0x20000,
        0x05 => 0x10000,
        id => panic!("Unknown RAM size ID: {id:x}"),
    };
    (cart_type, rom_size, ram_size)
}

pub struct Cart<M = AnyMbc> {
    mem: Mem,
    mbc: M,
}

impl<M: Mbc> Cart<M> {
    pub fn read_low(&self, addr: u16) -> u8 {
        self.mbc.read_low(addr, &self.mem)
    }

    pub fn write_low(&mut self, addr: u16, val: u8) {
        self.mbc.write_low(addr, val, &mut self.mem);
    }

    pub fn read_high(&self, addr: u16) -> u8 {
        self.mbc.read_high(addr, &self.mem)
    }

    pub fn write_high(&mut self, addr: u16, val: u8) {
        self.mbc.write_high(addr, val, &mut self.mem);
    }
}

impl Cart {
    pub fn from_rom(mut rom: Box<[u8]>) -> Self {
        let (cart_type, rom_size, mut ram_size) = header(&rom[..]);

        let mbc = match cart_type {
            0x00 | 0x08 | 0x09 => AnyMbc::Simple(Default::default()),
            0x01 | 0x02 | 0x03 => AnyMbc::Mbc1(Default::default()),
            0x05 | 0x06 => {
                ram_size = 512;
                AnyMbc::Mbc2(Default::default())
            }
            0x0f | 0x10 => AnyMbc::Mbc3(Mbc3::new_with_rtc()),
            0x11 | 0x12 | 0x13 => AnyMbc::Mbc3(Default::default()),
            _ => panic!("Unknown cartrige type: {cart_type:x}"),
        };

        assert!(rom_size >= rom.len(), "ROM is too big");
        if rom_size > rom.len() {
            let mut vec = Vec::from(rom);
            vec.resize(rom_size, 0);
            rom = vec.into_boxed_slice();
        }
        let rom = Segment::try_from(rom).unwrap();

        let ram = OptionalSegment::new(ram_size);

        Self {
            mem: Mem { rom, ram },
            mbc,
        }
    }
}
