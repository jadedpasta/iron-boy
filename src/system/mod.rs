// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023 Robert Hrusecky <jadedpastabowl@gmail.com>
mod cpu;
mod dma;
mod joypad;
mod ppu;
mod timer;

use partial_borrow::{prelude::*, SplitOff};

use crate::{
    cpu::{Cpu, CpuBus},
    dma::{Dma, DmaBus},
    interrupt::InterruptState,
    joypad::{Button, ButtonState, Joypad},
    memory::MemoryData,
    ppu::{Ppu, PpuBus},
    timer::{Timer, TimerBus},
};

const BOOT_ROM: &'static [u8] = include_bytes!("../../sameboy_boot.bin");

#[derive(PartialBorrow)]
pub struct CgbSystem {
    cpu: Cpu,
    timer: Timer,
    ppu: Ppu,
    dma: Dma,
    mem: MemoryData,
    joypad: Joypad,
    interrupt: InterruptState,
    boot_rom_mapped: bool,
    cgb_mode: bool,
    key0: u8, // TODO: This can probably be combined with cgb_mode
}

impl CgbSystem {
    pub fn new(rom: impl Into<Vec<u8>>) -> Box<Self> {
        Box::new(CgbSystem {
            cpu: Cpu::default(),
            timer: Timer::new(),
            dma: Dma::new(),
            ppu: Ppu::new(),
            mem: MemoryData::new(rom),
            joypad: Joypad::new(),
            interrupt: InterruptState::new(),
            boot_rom_mapped: true,
            cgb_mode: true,
            key0: 0,
        })
    }

    pub fn split_cpu(&mut self) -> (&mut Cpu, &mut impl CpuBus) {
        let (bus, system) = SplitOff::split_off_mut(self);
        return (&mut system.cpu, bus);
    }

    pub fn split_ppu(&mut self) -> (&mut Ppu, &mut impl PpuBus) {
        let (bus, system) = SplitOff::split_off_mut(self);
        return (&mut system.ppu, bus);
    }

    pub fn split_dma(&mut self) -> (&mut Dma, &mut impl DmaBus) {
        let (bus, system) = SplitOff::split_off_mut(self);
        return (&mut system.dma, bus);
    }

    pub fn split_timer(&mut self) -> (&mut Timer, &mut impl TimerBus) {
        let (bus, system) = SplitOff::split_off_mut(self);
        return (&mut system.timer, bus);
    }

    pub fn lcd_on(&self) -> bool {
        self.ppu.lcd_enabled()
    }

    pub fn handle_joypad(&mut self, button: Button, state: ButtonState) {
        let (bus, system) = SplitOff::split_off_mut(self);
        system.joypad.handle(button, state, bus);
    }
}
