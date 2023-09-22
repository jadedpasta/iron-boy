// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023 Robert Hrusecky <jadedpastabowl@gmail.com>
use crate::memory::OamBytes;

pub enum DmaType {
    Oam,
    General,
}

struct DmaState {
    pub ty: DmaType,
    pub len: u16,
    pub count: u16,
    pub oam_src: u16,
}

pub trait DmaBus {
    fn write_vram(&mut self, addr: u16, val: u8);
    fn oam_mut(&mut self) -> &mut OamBytes;
    fn read_8(&self, addr: u16) -> u8;
}

pub struct Dma {
    state: Option<DmaState>,
    cpu_paused: bool,
    dma: u8,
    pub hdma1: u8,
    pub hdma2: u8,
    pub hdma3: u8,
    pub hdma4: u8,
}

impl Dma {
    pub fn new() -> Self {
        Self {
            state: None,
            cpu_paused: false,
            dma: 0,
            hdma1: 0,
            hdma2: 0,
            hdma3: 0,
            hdma4: 0,
        }
    }

    pub fn cpu_paused(&self) -> bool {
        self.cpu_paused
    }

    fn start_general(&mut self, len: u16) {
        // TODO: Do some kind of cancel of an ongoing OAM DMA for simplicity
        self.state = Some(DmaState {
            ty: DmaType::General,
            len,
            count: 0,
            oam_src: 0,
        });
    }

    pub fn hdma5(&self) -> u8 {
        todo!("HDMA5 reads (see pandocs)")
    }

    pub fn set_hdma5(&mut self, hdma5: u8) {
        let len = ((hdma5 & 0x7f) as u16).wrapping_add(1) * 16;
        if hdma5 >> 7 != 0 {
            todo!("HBlank DMA");
        } else {
            self.start_general(len);
        }
    }

    fn start_oam(&mut self, oam_src: u16) {
        // TODO: Do some kind of cancel of an ongoing HDMA for simplicity
        self.state = Some(DmaState {
            ty: DmaType::Oam,
            len: 0xa0,
            count: 0,
            oam_src,
        });
    }

    pub fn dma(&self) -> u8 {
        self.dma
    }

    pub fn set_dma(&mut self, dma: u8) {
        self.dma = dma;
        self.start_oam((dma as u16) << 8);
    }

    fn general_src_addr(&self) -> u16 {
        u16::from_be_bytes([self.hdma1, self.hdma2]) & 0xfff0
    }

    fn general_dst_addr(&self) -> u16 {
        u16::from_be_bytes([self.hdma3, self.hdma4]) & 0x1ff0
    }

    pub fn execute(&mut self, bus: &mut impl DmaBus) {
        let Some(state) = &self.state else {
            return;
        };

        match state.ty {
            DmaType::General => {
                // Ensure the CPU is stalled during the transfer
                self.cpu_paused = true;
                // Copy 2 bytes per M-cycle
                let src_addr = self.general_src_addr().wrapping_add(state.count);
                let dst_addr = self.general_dst_addr().wrapping_add(state.count);
                bus.write_vram(dst_addr, bus.read_8(src_addr));
                let src_addr = src_addr.wrapping_add(1);
                let dst_addr = dst_addr.wrapping_add(1);
                bus.write_vram(dst_addr, bus.read_8(src_addr));
            }
            DmaType::Oam => {
                let src_addr = state.oam_src.wrapping_add(state.count);
                let dst_addr = state.count;
                bus.oam_mut()[dst_addr as usize] = bus.read_8(src_addr);
            }
        }

        // Gotta reborrow to keep the borrow checker happy. Hopefully this can be optimized out?
        let state = self.state.as_mut().unwrap();

        state.count += match state.ty {
            DmaType::General => 2,
            DmaType::Oam => 1,
        };

        if state.count == state.len {
            // Transfer is complete
            self.state = None;
            self.cpu_paused = false;
        }
    }
}
