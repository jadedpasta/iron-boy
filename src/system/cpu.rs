// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023 Robert Hrusecky <jadedpastabowl@gmail.com>
use partial_borrow::prelude::*;

use crate::{cpu::CpuBus, reg};

use super::{CgbSystem, BOOT_ROM};

const NON_CGB_KEY0_VAL: u8 = 0x04;

impl CpuBus for partial!(CgbSystem ! cpu, mut *) {
    fn read_8(&self, addr: u16) -> u8 {
        match (addr >> 8) as u8 {
            0x00..=0x00 | 0x02..=0x08 if *self.boot_rom_mapped => BOOT_ROM[addr as usize],
            0x00..=0x7f => self.cart.read_low(addr),
            0x80..=0x9f => self.mem.vram.read(addr, *self.cgb_mode),
            0xa0..=0xbf => self.cart.read_high(addr),
            0xc0..=0xcf | 0xe0..=0xef => self.mem.wram.read_low(addr),
            0xd0..=0xdf | 0xf0..=0xfd => self.mem.wram.read_high(addr, *self.cgb_mode),
            0xfe => match addr as u8 {
                low @ 0x00..=0x9f => self.mem.oam[low as usize],
                low @ 0xa0..=0xff => {
                    // CGB-E prohibited area reads, according to pandocs
                    let low = low & 0x0f;
                    low << 4 | low
                }
            },
            0xff => match addr as u8 {
                low @ 0x80..=0xfe => self.mem.hram[low as usize - 0x80],
                reg::BCPD if *self.cgb_mode => self.mem.bg_palette.read_data(),
                reg::OCPD if *self.cgb_mode => self.mem.obj_palette.read_data(),
                reg::BCPS if *self.cgb_mode => self.mem.bg_palette.select,
                reg::OCPS if *self.cgb_mode => self.mem.obj_palette.select,
                reg::HDMA5 if *self.cgb_mode => self.dma.hdma5(),
                reg::HDMA1 => self.dma.hdma1,
                reg::HDMA2 => self.dma.hdma2,
                reg::HDMA3 => self.dma.hdma3,
                reg::HDMA4 => self.dma.hdma4,
                reg::P1 => self.joypad.p1(),
                reg::DIV => self.timer.div(),
                reg::TIMA => self.timer.tima(),
                reg::TMA => self.timer.tma(),
                reg::TAC => self.timer.tac(),
                reg::SVBK => self.mem.wram.svbk,
                reg::VBK => self.mem.vram.vbk,
                reg::IF => self.interrupt.flags,
                reg::IE => self.interrupt.enable,
                reg::DMA => self.dma.dma(),
                reg::BGP => self.ppu.bgp,
                reg::LCDC => self.ppu.lcdc(),
                reg::LY => self.ppu.ly(),
                reg::LYC => self.ppu.lyc,
                reg::OBP0 => self.ppu.obp0,
                reg::OBP1 => self.ppu.obp1,
                reg::SCX => self.ppu.scx,
                reg::SCY => self.ppu.scy,
                reg::WX => self.ppu.wx,
                reg::WY => self.ppu.wy,
                reg::STAT => self.ppu.stat(),
                _ => 0, // unimplemented
            },
        }
    }

    fn write_8(&mut self, addr: u16, val: u8) {
        match (addr >> 8) as u8 {
            0x00..=0x7f => self.cart.write_low(addr, val),
            0x80..=0x9f => self.mem.vram.write(addr, val, *self.cgb_mode),
            0xa0..=0xbf => self.cart.write_high(addr, val),
            0xc0..=0xcf | 0xe0..=0xef => self.mem.wram.write_low(addr, val),
            0xd0..=0xdf | 0xf0..=0xfd => self.mem.wram.write_high(addr, val, *self.cgb_mode),
            0xfe => match addr as u8 {
                low @ 0x00..=0x9f => self.mem.oam[low as usize] = val,
                0xa0..=0xff => (),
            },
            0xff => match addr as u8 {
                low @ 0x80..=0xfe => self.mem.hram[low as usize - 0x80] = val,
                reg::BCPD if *self.cgb_mode => self.mem.bg_palette.write_data(val),
                reg::OCPD if *self.cgb_mode => self.mem.obj_palette.write_data(val),
                reg::BCPS if *self.cgb_mode => self.mem.bg_palette.select = val,
                reg::OCPS if *self.cgb_mode => self.mem.obj_palette.select = val,
                reg::HDMA5 if *self.cgb_mode => self.dma.set_hdma5(val),
                reg::DMA => self.dma.set_dma(val),
                reg::BANK if *self.boot_rom_mapped => {
                    *self.boot_rom_mapped = false;
                    *self.cgb_mode = *self.key0 != NON_CGB_KEY0_VAL;
                }
                reg::KEY0 => *self.key0 = val,
                reg::HDMA1 => self.dma.hdma1 = val,
                reg::HDMA2 => self.dma.hdma2 = val,
                reg::HDMA3 => self.dma.hdma3 = val,
                reg::HDMA4 => self.dma.hdma4 = val,
                reg::DIV => self.timer.reset_div(),
                reg::TIMA => self.timer.set_tima(val),
                reg::TMA => self.timer.set_tma(val),
                reg::TAC => self.timer.set_tac(val),
                reg::SVBK => self.mem.wram.svbk = val,
                reg::VBK => self.mem.vram.vbk = val,
                reg::P1 => self.joypad.set_p1(val),
                reg::IF => self.interrupt.flags = val,
                reg::IE => self.interrupt.enable = val,
                reg::BGP => self.ppu.bgp = val,
                reg::LCDC => self.ppu.set_lcdc(val),
                reg::LYC => self.ppu.lyc = val,
                reg::OBP0 => self.ppu.obp0 = val,
                reg::OBP1 => self.ppu.obp1 = val,
                reg::SCX => self.ppu.scx = val,
                reg::SCY => self.ppu.scy = val,
                reg::WX => self.ppu.wx = val,
                reg::WY => self.ppu.wy = val,
                reg::STAT => self.ppu.set_stat(val),
                _ => (), // unimplemented
            },
        }
    }

    fn cpu_dma_paused(&self) -> bool {
        self.dma.cpu_paused()
    }

    fn pop_interrupt(&mut self) -> Option<u8> {
        self.interrupt.pop()
    }

    fn interrupt_pending(&mut self) -> bool {
        self.interrupt.pending()
    }
}
