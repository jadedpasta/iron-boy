// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023 Robert Hrusecky <jadedpastabowl@gmail.com>

use super::{mem::Mem, rtc::Rtc, Mbc};

#[derive(Default)]
pub struct Mbc3 {
    rom_bank: u8,
    ram_bank: u8,
    ram_enabled: bool,
    rtc: Option<Rtc>,
}

impl Mbc3 {
    pub fn new_with_rtc() -> Self {
        Self { rtc: Some(Default::default()), ..Default::default() }
    }
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
        let mut offset = (addr & 0x1fff) as usize;
        offset |= ((self.ram_bank & 0x3) as usize) << 13;
        offset
    }
}

impl Mbc for Mbc3 {
    fn read_low(&self, addr: u16, mem: &Mem) -> u8 {
        mem.rom.read(self.rom_offset(addr))
    }

    fn write_low(&mut self, addr: u16, val: u8, _mem: &mut Mem) {
        let reg_num = (addr >> 13) & 0x3;
        match reg_num {
            0 => self.ram_enabled = val & 0xf == 0xa,
            1 => self.rom_bank = val & 0x7f,
            2 => self.ram_bank = val,
            3 => {
                if let Some(rtc) = &mut self.rtc {
                    rtc.latch(val & 0x1 != 0);
                }
            }
            _ => unreachable!(),
        }
    }

    fn read_high(&self, addr: u16, mem: &Mem) -> u8 {
        match self {
            Self { ram_enabled: false, .. } => 0xff,
            Self { rtc: Some(rtc), ram_bank: rtc_reg @ 0x08..=0x0c, .. } => match rtc_reg {
                0x08 => rtc.seconds() as u8,
                0x09 => rtc.minutes() as u8,
                0x0a => rtc.hours() as u8,
                0x0b => rtc.days() as u8,
                0x0c => rtc.flags().into(),
                _ => unreachable!(),
            },
            _ => mem.ram.read(self.ram_offset(addr)),
        }
    }

    fn write_high(&mut self, addr: u16, val: u8, mem: &mut Mem) {
        match self {
            Self { ram_enabled: false, .. } => (),
            Self { rtc: Some(rtc), ram_bank: rtc_reg @ 0x08..=0x0c, .. } => match rtc_reg {
                0x08 => rtc.set_seconds(val),
                0x09 => rtc.set_minutes(val),
                0x0a => rtc.set_hours(val),
                0x0b => rtc.set_days(val),
                0x0c => rtc.set_flags(val.into()),
                _ => unreachable!(),
            },
            _ => mem.ram.write(self.ram_offset(addr), val),
        }
    }
}
