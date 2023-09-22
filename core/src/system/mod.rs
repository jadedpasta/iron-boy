// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023 Robert Hrusecky <jadedpastabowl@gmail.com>
mod apu;
mod cpu;
mod dma;
mod joypad;
mod ppu;
mod timer;

use std::time::Duration;

use partial_borrow::{prelude::*, SplitOff};

use crate::{
    apu::{Apu, ApuBus},
    cart::Cart,
    cpu::{Cpu, CpuBus},
    dma::{Dma, DmaBus},
    interrupt::InterruptState,
    joypad::{Button, ButtonState, Joypad},
    memory::MemoryData,
    ppu::{Ppu, PpuBus},
    timer::{Timer, TimerBus},
};

const BOOT_ROM: &[u8] = include_bytes!("../../sameboy_boot.bin");

pub const SCREEN_WIDTH: usize = 160;
pub const SCREEN_HEIGHT: usize = 144;
pub const VBLANK_LINES: usize = 10;
pub const FRAME_LINES: usize = SCREEN_HEIGHT + VBLANK_LINES;
pub type FrameBuffer = [[[u8; 4]; SCREEN_WIDTH]; SCREEN_HEIGHT];

#[derive(Debug, Clone, Copy)]
pub struct MachineCycle(pub usize);

impl MachineCycle {
    pub const FREQ: usize = 1 << 20;
    pub const PER_LINE: usize = 114;
    pub const PER_FRAME: usize = FRAME_LINES * Self::PER_LINE;
}

impl From<MachineCycle> for Duration {
    fn from(cycles: MachineCycle) -> Duration {
        Self::from_secs_f64(cycles.0 as f64 / MachineCycle::FREQ as f64)
    }
}

#[derive(PartialBorrow)]
pub struct CgbSystem {
    cpu: Cpu,
    timer: Timer,
    ppu: Ppu,
    dma: Dma,
    apu: Apu,
    mem: MemoryData,
    joypad: Joypad,
    interrupt: InterruptState,
    boot_rom_mapped: bool,
    cgb_mode: bool,
    key0: u8, // TODO: This can probably be combined with cgb_mode
    cart: Cart,
}

impl CgbSystem {
    pub fn new(cart: Cart) -> Self {
        CgbSystem {
            cpu: Cpu::default(),
            timer: Timer::new(),
            dma: Dma::new(),
            ppu: Ppu::new(),
            apu: Apu::default(),
            mem: MemoryData::new(),
            joypad: Joypad::new(),
            interrupt: InterruptState::new(),
            boot_rom_mapped: true,
            cgb_mode: true,
            key0: 0,
            cart,
        }
    }

    pub fn cart(&self) -> &Cart {
        &self.cart
    }

    fn split_cpu(&mut self) -> (&mut Cpu, &mut impl CpuBus) {
        let (bus, system) = SplitOff::split_off_mut(self);
        (&mut system.cpu, bus)
    }

    fn split_ppu(&mut self) -> (&mut Ppu, &mut impl PpuBus) {
        let (bus, system) = SplitOff::split_off_mut(self);
        (&mut system.ppu, bus)
    }

    fn split_dma(&mut self) -> (&mut Dma, &mut impl DmaBus) {
        let (bus, system) = SplitOff::split_off_mut(self);
        (&mut system.dma, bus)
    }

    fn split_apu(&mut self) -> (&mut Apu, &mut impl ApuBus) {
        let (bus, system) = SplitOff::split_off_mut(self);
        (&mut system.apu, bus)
    }

    fn split_timer(&mut self) -> (&mut Timer, &mut impl TimerBus) {
        let (bus, system) = SplitOff::split_off_mut(self);
        (&mut system.timer, bus)
    }

    pub fn handle_joypad(&mut self, button: Button, state: ButtonState) {
        let (bus, system) = SplitOff::split_off_mut(self);
        system.joypad.handle(button, state, bus);
    }

    fn execute_machine_cycle(
        &mut self,
        frame_buff: &mut FrameBuffer,
        audio_callback: &mut impl FnMut([f32; 2]),
    ) {
        let (ppu, bus) = self.split_ppu();
        ppu.execute(frame_buff, bus);
        let (dma, bus) = self.split_dma();
        dma.execute(bus);
        let (apu, bus) = self.split_apu();
        apu.execute(bus).into_iter().for_each(audio_callback);
        let (cpu, bus) = self.split_cpu();
        cpu.execute(bus);
        let (timer, bus) = self.split_timer();
        timer.execute(bus);
    }

    pub fn execute(
        &mut self,
        frame_buff: &mut FrameBuffer,
        mut audio_callback: impl FnMut([f32; 2]),
    ) -> MachineCycle {
        let lcd_on = self.ppu.lcd_enabled();
        let mut cycles = MachineCycle::PER_FRAME;
        for c in 1..=cycles {
            self.execute_machine_cycle(frame_buff, &mut audio_callback);
            if !lcd_on && self.ppu.lcd_enabled() {
                cycles = c;
                break;
            }
        }

        if !lcd_on {
            // If the LCD is off, make sure we are showing a white screen
            *frame_buff = [[[0xff; 4]; SCREEN_WIDTH]; SCREEN_HEIGHT];
        }

        MachineCycle(cycles)
    }
}
