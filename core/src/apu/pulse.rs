// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023 Robert Hrusecky <jadedpastabowl@gmail.com>

use std::num::Wrapping;

use bilge::prelude::*;

use super::{
    Channel, Envelope, LengthTimer, LengthTimerRegs, Nrx2, Nrx4, PeriodDivider, PeriodDividerRegs,
};

#[bitsize(8)]
#[derive(Default, FromBits, DebugBits, Clone, Copy)]
pub(super) struct Nr10 {
    sweep_slope: u3,
    decrease_sweep: bool,
    sweep_pace: u3,
    __: u1,
}

#[bitsize(2)]
#[derive(Default, FromBits, Debug, Clone, Copy)]
pub(super) enum WaveDuty {
    #[default]
    W12,
    W25,
    W50,
    W75,
}

#[bitsize(8)]
#[derive(Default, FromBits, DebugBits, Clone, Copy)]
pub(super) struct Nrx1 {
    pub(super) initial_length_timer: u6,
    wave_duty: WaveDuty,
}

pub(super) enum SweepAction {
    Nothing,
    Disable,
    SetPeriod(u16),
}

pub(super) trait Sweep {
    fn trigger(&mut self);
    fn clock(&mut self, regs: &impl PeriodDividerRegs) -> SweepAction;
}

#[derive(Default)]
pub(super) struct NoSweep;

impl Sweep for NoSweep {
    fn trigger(&mut self) {}

    fn clock(&mut self, _regs: &impl PeriodDividerRegs) -> SweepAction {
        unimplemented!()
    }
}

#[derive(Default)]
pub(super) struct Sweeper {
    pub(super) nr10: Nr10,
    count: u8,
}

impl Sweep for Sweeper {
    fn trigger(&mut self) {
        self.count = self.nr10.sweep_pace().value();
    }

    fn clock(&mut self, regs: &impl PeriodDividerRegs) -> SweepAction {
        let slope = self.nr10.sweep_slope().value();
        if self.count > 0 {
            self.count -= 1;
            SweepAction::Nothing
        } else if slope == 0 {
            SweepAction::Nothing
        } else {
            let period = regs.neg_period();
            let offset = period >> slope;
            if self.nr10.decrease_sweep() {
                SweepAction::SetPeriod(period - offset)
            } else if period as u32 + offset as u32 > 0x7ff {
                // overflow
                SweepAction::Disable
            } else {
                SweepAction::SetPeriod(period + offset)
            }
        }
    }
}

#[derive(Default)]
pub(super) struct PulseRegs {
    pub(super) nrx1: Nrx1,
    pub(super) nrx2: Nrx2,
    pub(super) nrx3: u8,
    pub(super) nrx4: Nrx4,
}

impl LengthTimerRegs for PulseRegs {
    type Timer = u8;
    const MAX: Self::Timer = 0x40;
    const INC: Self::Timer = 1;

    fn initial(&self) -> Self::Timer {
        self.nrx1.initial_length_timer().value()
    }

    fn enabled(&self) -> bool {
        self.nrx4.sound_length_enabled()
    }
}

impl PeriodDividerRegs for PulseRegs {
    fn neg_period(&self) -> u16 {
        ((self.nrx4.period_high().value() as u16) << 8) | self.nrx3 as u16
    }
}

#[derive(Default)]
pub(super) struct PulseChannel<S: Sweep> {
    pub(super) sweeper: S,
    pub(super) regs: PulseRegs,
    duty_step: Wrapping<u8>,
    period_div: PeriodDivider,
    length_timer: LengthTimer<PulseRegs>,
    envelope: Envelope,
    enabled: bool,
}

impl<S: Sweep> PulseChannel<S> {
    pub(super) fn dac_enabled(&self) -> bool {
        self.regs.nrx2.initial_volume().value() != 0 || self.regs.nrx2.increase_envelope()
    }

    pub(super) fn envelope_clock(&mut self) {
        self.envelope.clock();
    }

    pub(super) fn sweep_clock(&mut self) {
        match self.sweeper.clock(&self.regs) {
            SweepAction::Nothing => (),
            SweepAction::Disable => self.enabled = false,
            SweepAction::SetPeriod(period) => {
                self.regs.nrx3 = period as u8;
                self.regs.nrx4.set_period_high(u3::new((period >> 8) as u8));
            }
        }
    }
}

impl<S: Sweep> Channel for PulseChannel<S> {
    fn enabled(&self) -> bool {
        self.enabled
    }

    fn wave(&self) -> (u8, u8) {
        let index = self.duty_step.0 & 0x7;
        let on = match self.regs.nrx1.wave_duty() {
            WaveDuty::W12 => index != 7,
            WaveDuty::W25 => index != 0 && index != 7,
            WaveDuty::W50 => index > 0 && index <= 4,
            WaveDuty::W75 => index == 0 || index == 7,
        };

        let wave = if on { self.envelope.volume } else { 0 };
        (wave, self.envelope.volume)
    }

    fn clock(&mut self) {
        if self.regs.nrx4.trigger() {
            self.regs.nrx4.set_trigger(false);
            self.enabled = true;
            self.period_div.trigger(&self.regs);
            self.length_timer.trigger(&self.regs);
            self.envelope = self.regs.nrx2.into();
            self.sweeper.trigger();
        }

        self.period_div.clock(&self.regs, || self.duty_step += 1);
    }

    fn length_clock(&mut self) {
        self.length_timer.clock(&self.regs, &mut self.enabled);
    }
}
