// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023 Robert Hrusecky <jadedpastabowl@gmail.com>

use std::num::Wrapping;

use bilge::prelude::*;

use super::{Channel, LengthTimer, LengthTimerRegs, Nrx4, PeriodDivider, PeriodDividerRegs};

#[bitsize(8)]
#[derive(Default, FromBits, DebugBits, Clone, Copy)]
pub(super) struct Nr30 {
    __: u7,
    dac_enabled: bool,
}

#[bitsize(8)]
#[derive(Default, FromBits, DebugBits, Clone, Copy)]
pub(super) struct Nr32 {
    _unused1: u5,
    output_level: u2,
    _unused2: u1,
}

#[derive(Default)]
pub(super) struct WaveRegs {
    pub(super) nr30: Nr30,
    pub(super) nr31: u8,
    pub(super) nr32: Nr32,
    pub(super) nr33: u8,
    pub(super) nr34: Nrx4,
}

impl LengthTimerRegs for WaveRegs {
    type Timer = u16;
    const MAX: Self::Timer = 0xff;
    const INC: Self::Timer = 1;

    fn initial(&self) -> Self::Timer {
        self.nr31 as u16
    }

    fn enabled(&self) -> bool {
        self.nr34.sound_length_enabled()
    }
}

impl PeriodDividerRegs for WaveRegs {
    fn neg_period(&self) -> u16 {
        ((self.nr34.period_high().value() as u16) << 8) | self.nr33 as u16
    }
}

#[derive(Default)]
pub(super) struct WaveChannel {
    pub(super) wave_ram: [u8; 16],
    pub(super) regs: WaveRegs,
    index: Wrapping<u8>,
    length_timer: LengthTimer<WaveRegs>,
    period_div: PeriodDivider,
    pub(super) enabled: bool,
}

impl WaveChannel {
    pub(super) fn dac_enabled(&self) -> bool {
        self.regs.nr30.dac_enabled()
    }

    pub(super) fn wave_ram_access_offset(&self, addr: u16) -> usize {
        (if self.enabled {
            (self.index.0 >> 1) as usize
        } else {
            addr as usize
        }) & 0xf
    }
}

impl Channel for WaveChannel {
    fn enabled(&self) -> bool {
        self.enabled
    }

    fn wave(&self) -> (u8, u8) {
        let output_level = self.regs.nr32.output_level().value();
        if output_level == 0 {
            return (0, 0);
        }

        let index = self.index.0 & 0x1f;
        let val = self.wave_ram[index as usize >> 1];
        let val = if index & 0x1 == 0 {
            val >> 4
        } else {
            val & 0xf
        };
        (val >> (output_level - 1), 0xf)
    }

    fn clock(&mut self) {
        if self.regs.nr34.trigger() {
            self.enabled |= self.regs.nr30.dac_enabled();
            self.regs.nr34.set_trigger(false);
            self.period_div.trigger(&self.regs);
            self.length_timer.trigger(&self.regs);
            self.index.0 = 0;
        }

        self.period_div.clock(&self.regs, || self.index += 1);
    }

    fn length_clock(&mut self) {
        self.length_timer.clock(&self.regs, &mut self.enabled);
    }
}
