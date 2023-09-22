// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023 Robert Hrusecky <jadedpastabowl@gmail.com>

use bilge::prelude::*;

use super::{
    Channel, Envelope, LengthTimer, LengthTimerRegs, Nrx2, PeriodDivider, PeriodDividerRegs,
};

#[bitsize(8)]
#[derive(Default, FromBits, DebugBits, Clone, Copy)]
pub(super) struct Nr41 {
    initial_length_timer: u6,
    __: u2,
}

#[bitsize(8)]
#[derive(Default, FromBits, DebugBits, Clone, Copy)]
pub(super) struct Nr43 {
    clock_divider: u3,
    short_mode: bool,
    clock_shift: u4,
}

#[bitsize(8)]
#[derive(Default, FromBits, DebugBits, Clone, Copy)]
pub(super) struct Nr44 {
    __: u6,
    pub(super) sound_length_enabled: bool,
    trigger: bool,
}

#[derive(Default)]
pub(super) struct NoiseRegs {
    pub(super) nr41: Nr41,
    pub(super) nr42: Nrx2,
    pub(super) nr43: Nr43,
    pub(super) nr44: Nr44,
}

impl LengthTimerRegs for NoiseRegs {
    type Timer = u8;
    const MAX: Self::Timer = 0x40;
    const INC: Self::Timer = 1;

    fn initial(&self) -> Self::Timer {
        self.nr41.initial_length_timer().value()
    }

    fn enabled(&self) -> bool {
        self.nr44.sound_length_enabled()
    }
}

impl PeriodDividerRegs for NoiseRegs {
    fn neg_period(&self) -> u16 {
        unimplemented!()
    }

    fn period(&self) -> u16 {
        let r = self.nr43.clock_divider().value() as u16;
        let base = if r == 0 { 2 } else { 4 * r };
        base << self.nr43.clock_shift().value()
    }
}

#[derive(Default)]
struct Lfsr {
    lfsr: u16,
}

impl Lfsr {
    fn wave(&self) -> bool {
        self.lfsr & 0x1 != 0
    }

    fn clock(&mut self, regs: &NoiseRegs) {
        let feedback = !(self.lfsr ^ (self.lfsr >> 1)) & 0x1;

        let mask = if regs.nr43.short_mode() {
            0x8080
        } else {
            0x8000
        };

        self.lfsr &= !mask;
        self.lfsr |= feedback * mask;
        self.lfsr >>= 1;
    }
}

#[derive(Default)]
pub(super) struct NoiseChannel {
    pub(super) regs: NoiseRegs,
    length_timer: LengthTimer<NoiseRegs>,
    period_div: PeriodDivider,
    envelope: Envelope,
    lfsr: Lfsr,
    enabled: bool,
}

impl NoiseChannel {
    pub(super) fn dac_enabled(&self) -> bool {
        self.regs.nr42.initial_volume().value() != 0 || self.regs.nr42.increase_envelope()
    }

    pub(super) fn envelope_clock(&mut self) {
        self.envelope.clock();
    }
}

impl Channel for NoiseChannel {
    fn enabled(&self) -> bool {
        self.enabled
    }

    fn wave(&self) -> (u8, u8) {
        let sample = if self.lfsr.wave() {
            self.envelope.volume
        } else {
            0
        };
        (sample, self.envelope.volume)
    }

    fn clock(&mut self) {
        if self.regs.nr44.trigger() {
            self.regs.nr44.set_trigger(false);
            self.enabled = true;
            self.period_div.trigger(&self.regs);
            self.length_timer.trigger(&self.regs);
            self.envelope = self.regs.nr42.into();
            self.lfsr = Default::default();
        }

        self.period_div
            .clock(&self.regs, || self.lfsr.clock(&self.regs));
    }

    fn length_clock(&mut self) {
        self.length_timer.clock(&self.regs, &mut self.enabled);
    }
}
