// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023 Robert Hrusecky <jadedpastabowl@gmail.com>

use ambassador::{delegatable_trait, Delegate};
use thiserror::Error;

use self::{
    mbc1::Mbc1,
    mbc2::Mbc2,
    mbc3::Mbc3,
    mem::{Mem, OptionalSegment, Segment},
    save::{CartSave, MbcSave},
    simple::Simple,
};

mod mbc1;
mod mbc2;
mod mbc3;
mod mem;
mod rtc;
pub mod save;
mod simple;

#[delegatable_trait]
pub trait Mbc {
    fn read_low(&self, addr: u16, mem: &Mem) -> u8;
    fn write_low(&mut self, addr: u16, val: u8, mem: &mut Mem);
    fn read_high(&self, addr: u16, mem: &Mem) -> u8;
    fn write_high(&mut self, addr: u16, val: u8, mem: &mut Mem);
    fn save(&self) -> MbcSave;
}

#[derive(Delegate)]
#[delegate(Mbc)]
pub enum AnyMbc {
    Simple(Simple),
    Mbc1(Mbc1),
    Mbc2(Mbc2),
    Mbc3(Mbc3),
}

pub struct Cart<M = AnyMbc> {
    mem: Mem,
    mbc: M,
    battery_backed: bool,
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

#[derive(Error, Debug)]
pub enum RomParseError {
    #[error("Unknown cartrige type: {0:#x}")]
    UnknownCartType(u8),
    #[error("Unknown ROM size ID: {0:#x}")]
    UnknownRomSize(u8),
    #[error("Unknown RAM size ID: {0:#x}")]
    UnknownRamSize(u8),
    #[error("Provided ROM is too large")]
    LargeRom,
}

impl Cart {
    pub fn from_rom(mut rom: Box<[u8]>) -> Result<Self, RomParseError> {
        let cart_type = rom[0x147];
        let rom_size = match rom[0x148] {
            id @ 0x0..=0x8 => 1 << (id + 15),
            id => return Err(RomParseError::UnknownRomSize(id)),
        };
        let mut ram_size = match rom[0x149] {
            0x00 => 0,
            0x02 => 0x2000,
            0x03 => 0x8000,
            0x04 => 0x20000,
            0x05 => 0x10000,
            id => return Err(RomParseError::UnknownRamSize(id)),
        };

        let mbc = match cart_type {
            0x00 | 0x08 | 0x09 => AnyMbc::Simple(Default::default()),
            0x01..=0x03 => AnyMbc::Mbc1(Default::default()),
            0x05 | 0x06 => {
                ram_size = 512;
                AnyMbc::Mbc2(Default::default())
            }
            0x0f | 0x10 => AnyMbc::Mbc3(Mbc3::new_with_rtc()),
            0x11..=0x13 => AnyMbc::Mbc3(Default::default()),
            _ => return Err(RomParseError::UnknownCartType(cart_type)),
        };

        let battery_backed = matches!(
            cart_type,
            0x03 | 0x06 | 0x09 | 0x0d | 0x0f | 0x10 | 0x13 | 0x1b | 0x1e | 0x22 | 0xff
        );

        if rom_size < rom.len() {
            return Err(RomParseError::LargeRom);
        }
        if rom_size > rom.len() {
            let mut vec = Vec::from(rom);
            vec.resize(rom_size, 0);
            rom = vec.into_boxed_slice();
        }
        let rom = Segment::try_from(rom).unwrap();

        let ram = OptionalSegment::new(ram_size);

        Ok(Self {
            mem: Mem { rom, ram },
            mbc,
            battery_backed,
        })
    }

    pub fn load_from_save(&mut self, save: CartSave) {
        if let MbcSave::Rtc(rtc) = save.mbc {
            if let AnyMbc::Mbc3(mbc3) = &mut self.mbc {
                if mbc3.has_rtc() {
                    mbc3.set_rtc(rtc.into())
                }
            }
        }

        self.mem.ram = save.ram.try_into().unwrap();
    }

    pub fn battery_backed(&self) -> bool {
        self.battery_backed
    }

    pub fn save(&self) -> Option<CartSave> {
        if self.battery_backed {
            Some(CartSave {
                mbc: self.mbc.save(),
                ram: self.mem.ram.raw(),
            })
        } else {
            None
        }
    }
}
