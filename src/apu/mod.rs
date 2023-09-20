// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023 Robert Hrusecky <jadedpastabowl@gmail.com>

use std::{f32, num::Wrapping, ops::AddAssign};

use bilge::prelude::*;

use self::{
    noise::NoiseChannel,
    pulse::{NoSweep, PulseChannel, Sweeper},
    wave::WaveChannel,
};

mod noise;
mod pulse;
mod wave;

pub trait ApuBus {
    fn div(&self) -> u8;
}

trait Channel {
    fn enabled(&self) -> bool;
    fn wave(&self) -> (u8, u8);
    fn clock(&mut self);
    fn length_clock(&mut self);

    fn sample(&self) -> (u8, u8) {
        if !self.enabled() {
            return (0, 0);
        }

        self.wave()
    }
}

trait PeriodDividerRegs {
    fn neg_period(&self) -> u16;
    fn period(&self) -> u16 {
        (!self.neg_period() + 1) & 0x7ff
    }
}

#[derive(Default)]
struct PeriodDivider {
    div: Wrapping<u16>,
}

impl PeriodDivider {
    fn trigger(&mut self, regs: &impl PeriodDividerRegs) {
        self.div.0 = regs.period();
    }

    fn clock(&mut self, regs: &impl PeriodDividerRegs, wave: impl FnOnce()) {
        self.div -= 1;
        if self.div.0 == 0 {
            self.div.0 = regs.period();
            wave();
        }
    }
}

trait LengthTimerRegs {
    type Timer: Ord + AddAssign;
    const MAX: Self::Timer;
    const INC: Self::Timer;

    fn initial(&self) -> Self::Timer;
    fn enabled(&self) -> bool;
}

#[derive(Default)]
struct LengthTimer<R: LengthTimerRegs> {
    timer: R::Timer,
}

impl<R: LengthTimerRegs> LengthTimer<R> {
    fn trigger(&mut self, regs: &R) {
        self.timer = regs.initial();
    }

    fn clock(&mut self, regs: &R, enabled: &mut bool) {
        if !regs.enabled() {
            return;
        }
        if self.timer >= R::MAX {
            *enabled = false;
        } else {
            self.timer += R::INC;
        }
    }
}

#[derive(Default)]
struct Envelope {
    volume: u8,
    increase: bool,
    sweep_pace: u8,
    countdown: u8,
}

impl Envelope {
    fn clock(&mut self) {
        if self.sweep_pace == 0 {
            return;
        }

        if self.countdown > 1 {
            self.countdown -= 1;
        } else {
            if self.increase {
                if self.volume < 0xf {
                    self.volume += 1;
                }
            } else if self.volume > 0 {
                self.volume -= 1;
            }
            self.countdown = self.sweep_pace;
        }
    }
}

#[bitsize(8)]
#[derive(Default, FromBits, DebugBits, Clone, Copy)]
struct Nrx2 {
    sweep_pace: u3,
    increase_envelope: bool,
    initial_volume: u4,
}

impl From<Nrx2> for Envelope {
    fn from(nr12: Nrx2) -> Self {
        let sweep_pace = nr12.sweep_pace().value();
        Self {
            volume: nr12.initial_volume().value(),
            increase: nr12.increase_envelope(),
            sweep_pace,
            countdown: sweep_pace,
        }
    }
}

#[bitsize(8)]
#[derive(Default, FromBits, DebugBits, Clone, Copy)]
struct Nrx4 {
    period_high: u3,
    __: u3,
    sound_length_enabled: bool,
    trigger: bool,
}

#[bitsize(8)]
#[derive(Default, FromBits, DebugBits, Clone, Copy)]
struct Nr50 {
    vol_right: u3,
    vin_right: bool,
    vol_left: u3,
    vin_left: bool,
}

#[bitsize(4)]
#[derive(Default, FromBits, DebugBits, Clone, Copy)]
struct MixerBits {
    channel_1: bool,
    channel_2: bool,
    channel_3: bool,
    channel_4: bool,
}

#[bitsize(8)]
#[derive(Default, FromBits, DebugBits, Clone, Copy)]
struct Nr51 {
    right: MixerBits,
    left: MixerBits,
}

#[bitsize(8)]
#[derive(Default, FromBits, DebugBits, Clone, Copy)]
struct Nr52 {
    channel_1_enabled: bool,
    channel_2_enabled: bool,
    channel_3_enabled: bool,
    channel_4_enabled: bool,
    __: u3,
    sound_enabled: bool,
}

#[derive(Default)]
struct RisingEdgeDetector {
    edge_seen: bool,
}

impl RisingEdgeDetector {
    fn at_edge(&mut self, signal: bool) -> bool {
        let edge = !self.edge_seen && signal;
        self.edge_seen = signal;
        edge
    }
}

#[derive(Default)]
struct DivCounter {
    last: u8,
    counter: Wrapping<u8>,
    length: RisingEdgeDetector,
    envelope: RisingEdgeDetector,
    sweep: RisingEdgeDetector,
}

impl DivCounter {
    const MASK: u8 = 0x10;

    fn clock(&mut self, bus: &mut impl ApuBus) {
        let div = bus.div();
        if !div & self.last & Self::MASK != 0 {
            self.counter += 1;
        }
        self.last = div;
    }

    fn length_clock(&mut self) -> bool {
        self.length.at_edge(self.counter.0 & 0x01 == 0)
    }

    fn envelope_clock(&mut self) -> bool {
        self.envelope.at_edge(self.counter.0 & 0x7 == 0x7)
    }

    fn sweep_clock(&mut self) -> bool {
        self.sweep.at_edge(self.counter.0 & 0x3 == 0x2)
    }
}

fn dac(enabled: bool, (input, volume): (u8, u8)) -> f32 {
    if enabled {
        (volume as i8 - input as i8 * 2) as f32 / 15.0
    } else {
        0.0
    }
}

fn mixer(bits: MixerBits, ch1: f32, ch2: f32, ch3: f32, ch4: f32) -> f32 {
    let mut out = 0.0;

    if bits.channel_1() {
        out += ch1;
    }
    if bits.channel_2() {
        out += ch2;
    }
    if bits.channel_3() {
        out += ch3;
    }
    if bits.channel_4() {
        out += ch4;
    }

    out / 4.0
}

#[derive(Default)]
pub struct Apu {
    nr50: Nr50,
    nr51: Nr51,
    div_counter: DivCounter,
    ch1: PulseChannel<Sweeper>,
    ch2: PulseChannel<NoSweep>,
    ch3: WaveChannel,
    ch4: NoiseChannel,
    enabled: bool,
}

impl Apu {
    pub fn nr10(&self) -> u8 {
        self.ch1.sweeper.nr10.into()
    }

    pub fn set_nr10(&mut self, nr10: u8) {
        self.ch1.sweeper.nr10 = nr10.into();
    }

    pub fn nr11(&self) -> u8 {
        let mut nrx1 = self.ch1.regs.nrx1;
        nrx1.set_initial_length_timer(Default::default());
        nrx1.into()
    }

    pub fn set_nr11(&mut self, nr11: u8) {
        self.ch1.regs.nrx1 = nr11.into();
    }

    pub fn nr12(&self) -> u8 {
        self.ch1.regs.nrx2.into()
    }

    pub fn set_nr12(&mut self, nr12: u8) {
        self.ch1.regs.nrx2 = nr12.into();
    }

    pub fn nr13(&self) -> u8 {
        self.ch1.regs.nrx3
    }

    pub fn set_nr13(&mut self, nr13: u8) {
        self.ch1.regs.nrx3 = nr13;
    }

    pub fn nr14(&self) -> u8 {
        let mut nrx4 = Nrx4::default();
        nrx4.set_sound_length_enabled(self.ch1.regs.nrx4.sound_length_enabled());
        nrx4.into()
    }

    pub fn set_nr14(&mut self, nr14: u8) {
        self.ch1.regs.nrx4 = nr14.into();
    }

    pub fn nr21(&self) -> u8 {
        let mut nrx1 = self.ch1.regs.nrx1;
        nrx1.set_initial_length_timer(Default::default());
        nrx1.into()
    }

    pub fn set_nr21(&mut self, nr21: u8) {
        self.ch2.regs.nrx1 = nr21.into();
    }

    pub fn nr22(&self) -> u8 {
        self.ch2.regs.nrx2.into()
    }

    pub fn set_nr22(&mut self, nr22: u8) {
        self.ch2.regs.nrx2 = nr22.into();
    }

    pub fn nr23(&self) -> u8 {
        self.ch2.regs.nrx3
    }

    pub fn set_nr23(&mut self, nr23: u8) {
        self.ch2.regs.nrx3 = nr23;
    }

    pub fn nr24(&self) -> u8 {
        let mut nrx4 = Nrx4::default();
        nrx4.set_sound_length_enabled(self.ch2.regs.nrx4.sound_length_enabled());
        nrx4.into()
    }

    pub fn set_nr24(&mut self, nr24: u8) {
        self.ch2.regs.nrx4 = nr24.into();
    }

    pub fn nr30(&self) -> u8 {
        self.ch3.regs.nr30.into()
    }

    pub fn set_nr30(&mut self, nr30: u8) {
        self.ch3.regs.nr30 = nr30.into();
        self.ch3.enabled &= self.ch3.dac_enabled();
    }

    pub fn nr31(&self) -> u8 {
        self.ch3.regs.nr31
    }

    pub fn set_nr31(&mut self, nr31: u8) {
        self.ch3.regs.nr31 = nr31;
    }

    pub fn nr32(&self) -> u8 {
        self.ch3.regs.nr32.into()
    }

    pub fn set_nr32(&mut self, nr32: u8) {
        self.ch3.regs.nr32 = nr32.into();
    }

    pub fn nr33(&self) -> u8 {
        self.ch3.regs.nr33
    }

    pub fn set_nr33(&mut self, nr33: u8) {
        self.ch3.regs.nr33 = nr33;
    }

    pub fn nr34(&self) -> u8 {
        let mut nr34 = Nrx4::default();
        nr34.set_sound_length_enabled(self.ch3.regs.nr34.sound_length_enabled());
        nr34.into()
    }

    pub fn set_nr34(&mut self, nr34: u8) {
        self.ch3.regs.nr34 = nr34.into();
    }

    pub fn set_nr41(&mut self, nr41: u8) {
        self.ch4.regs.nr41 = nr41.into();
    }

    pub fn nr42(&self) -> u8 {
        self.ch4.regs.nr42.into()
    }

    pub fn set_nr42(&mut self, nr42: u8) {
        self.ch4.regs.nr42 = nr42.into();
    }

    pub fn nr43(&self) -> u8 {
        self.ch4.regs.nr43.into()
    }

    pub fn set_nr43(&mut self, nr43: u8) {
        self.ch4.regs.nr43 = nr43.into();
    }

    pub fn nr44(&self) -> u8 {
        let mut nrx4 = Nrx4::default();
        nrx4.set_sound_length_enabled(self.ch4.regs.nr44.sound_length_enabled());
        nrx4.into()
    }

    pub fn set_nr44(&mut self, nr44: u8) {
        self.ch4.regs.nr44 = nr44.into();
    }

    pub fn set_nr50(&mut self, nr50: u8) {
        self.nr50 = nr50.into();
    }

    pub fn nr50(&self) -> u8 {
        self.nr50.into()
    }

    pub fn set_nr51(&mut self, nr51: u8) {
        self.nr51 = nr51.into();
    }

    pub fn nr51(&self) -> u8 {
        self.nr51.into()
    }

    pub fn set_nr52(&mut self, nr52: u8) {
        let nr52 = Nr52::from(nr52);
        self.enabled = nr52.sound_enabled();
    }

    pub fn nr52(&self) -> u8 {
        let mut nr52 = Nr52::default();
        nr52.set_sound_enabled(self.enabled);
        nr52.set_channel_1_enabled(self.ch1.enabled());
        nr52.set_channel_2_enabled(self.ch2.enabled());
        nr52.set_channel_3_enabled(self.ch3.enabled());
        nr52.set_channel_4_enabled(self.ch4.enabled());
        nr52.into()
    }

    pub fn read_wave_ram(&self, addr: u16) -> u8 {
        self.ch3.wave_ram[self.ch3.wave_ram_access_offset(addr)]
    }

    pub fn write_wave_ram(&mut self, addr: u16, val: u8) {
        self.ch3.wave_ram[self.ch3.wave_ram_access_offset(addr)] = val;
    }

    fn frame(&self) -> [f32; 2] {
        let ch1 = dac(self.ch1.dac_enabled(), self.ch1.sample());
        let ch2 = dac(self.ch2.dac_enabled(), self.ch2.sample());
        let ch3 = dac(self.ch3.dac_enabled(), self.ch3.sample());
        let ch4 = dac(self.ch4.dac_enabled(), self.ch4.sample());

        let mut left = mixer(self.nr51.left(), ch1, ch2, ch3, ch4);
        let mut right = mixer(self.nr51.right(), ch1, ch2, ch3, ch4);

        left *= ((self.nr50.vol_left().value() + 1) as f32) / 8.0 / 4.0;
        right *= ((self.nr50.vol_right().value() + 1) as f32) / 8.0 / 4.0;

        [left, right]
    }

    pub fn execute(&mut self, bus: &mut impl ApuBus) -> [[f32; 2]; 2] {
        if !self.enabled {
            *self = Default::default();
            return [[0.0, 0.0], [0.0, 0.0]];
        }

        self.div_counter.clock(bus);

        self.ch1.clock();
        self.ch2.clock();
        self.ch3.clock();
        self.ch4.clock();

        if self.div_counter.length_clock() {
            self.ch1.length_clock();
            self.ch2.length_clock();
            self.ch3.length_clock();
            self.ch4.length_clock();
        }

        if self.div_counter.envelope_clock() {
            self.ch1.envelope_clock();
            self.ch2.envelope_clock();
            self.ch4.envelope_clock();
        }

        if self.div_counter.sweep_clock() {
            self.ch1.sweep_clock();
        }

        let frame1 = self.frame();

        self.ch3.clock();
        let frame2 = self.frame();

        [frame1, frame2]
    }
}
