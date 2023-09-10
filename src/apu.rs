// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023 Robert Hrusecky <jadedpastabowl@gmail.com>

use std::{f32, num::Wrapping};

use bilge::prelude::*;

pub trait ApuBus {
    fn div(&self) -> u8;
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
            } else {
                if self.volume > 0 {
                    self.volume -= 1;
                }
            }
            self.countdown = self.sweep_pace;
        }
    }
}

#[bitsize(8)]
#[derive(Default, FromBits, DebugBits, Clone, Copy)]
struct Nr10 {
    sweep_slope: u3,
    increase_sweep: bool,
    sweep_pace: u3,
    __: u1,
}

#[bitsize(2)]
#[derive(Default, FromBits, Debug, Clone, Copy)]
enum WaveDuty {
    #[default]
    W12,
    W25,
    W50,
    W75,
}

#[bitsize(8)]
#[derive(Default, FromBits, DebugBits, Clone, Copy)]
struct Nrx1 {
    initial_length_timer: u6,
    wave_duty: WaveDuty,
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

const PERIOD_DIV_MAX: u16 = 0x800;

enum SweepAction {
    Nothing,
    Disable,
    SetPeriod(u16),
}

trait Sweep {
    fn trigger(&mut self);
    fn clock(&mut self, period: u16) -> SweepAction;
}

#[derive(Default)]
struct NoSweep;

impl Sweep for NoSweep {
    fn trigger(&mut self) {}

    fn clock(&mut self, _period: u16) -> SweepAction {
        unimplemented!()
    }
}

#[derive(Default)]
struct Sweeper {
    nr10: Nr10,
    count: u8,
}

impl Sweep for Sweeper {
    fn trigger(&mut self) {
        self.count = self.nr10.sweep_pace().value();
    }

    fn clock(&mut self, period: u16) -> SweepAction {
        let slope = self.nr10.sweep_slope().value();
        if self.count > 0 {
            self.count -= 1;
            SweepAction::Nothing
        } else if slope == 0 {
            SweepAction::Nothing
        } else {
            let offset = period >> slope;
            if self.nr10.increase_sweep() {
                if period as u32 + offset as u32 > 0x1ff {
                    // overflow
                    SweepAction::Disable
                } else {
                    SweepAction::SetPeriod(period + offset)
                }
            } else {
                SweepAction::SetPeriod(period - offset)
            }
        }
    }
}

#[derive(Default)]
struct PulseChannel<S: Sweep> {
    sweeper: S,
    duty_step: Wrapping<u8>,
    period_div: u16,
    length_timer: u8,
    envelope: Envelope,
    nrx1: Nrx1,
    nrx2: Nrx2,
    nrx3: u8,
    nrx4: Nrx4,
    enabled: bool,
}

impl<S: Sweep> PulseChannel<S> {
    const LENGTH_TIMER_MAX: u8 = 0x40;

    fn dac_enabled(&self) -> bool {
        self.nrx2.initial_volume().value() != 0 || self.nrx2.increase_envelope()
    }

    fn period(&self) -> u16 {
        ((self.nrx4.period_high().value() as u16) << 8) | self.nrx3 as u16
    }

    fn sample(&self) -> (u8, u8) {
        if !self.enabled {
            return (0, 0);
        }

        let index = self.duty_step.0 & 0x7;
        let wave = match self.nrx1.wave_duty() {
            WaveDuty::W12 => index != 7,
            WaveDuty::W25 => index != 0 && index != 7,
            WaveDuty::W50 => index > 0 && index <= 4,
            WaveDuty::W75 => index == 0 || index == 7,
        };

        let sample = if wave { self.envelope.volume } else { 0 };
        (sample, self.envelope.volume)
    }

    fn length_clock(&mut self) {
        if !self.nrx4.sound_length_enabled() {
            return;
        }
        if self.length_timer >= Self::LENGTH_TIMER_MAX {
            self.enabled = false;
        } else {
            self.length_timer += 1;
        }
    }

    fn envelope_clock(&mut self) {
        self.envelope.clock();
    }

    fn sweep_clock(&mut self) {
        match self.sweeper.clock(self.period()) {
            SweepAction::Nothing => (),
            SweepAction::Disable => self.enabled = false,
            SweepAction::SetPeriod(period) => {
                self.nrx3 = period as u8;
                self.nrx4.set_period_high(u3::new((period >> 8) as u8));
            }
        }
    }

    fn clock(&mut self) {
        if self.nrx4.trigger() {
            self.nrx4.set_trigger(false);
            self.enabled = true;
            self.period_div = self.period();
            self.length_timer = self.nrx1.initial_length_timer().value();
            self.envelope = self.nrx2.into();
            self.sweeper.trigger();
        }

        if self.period_div >= PERIOD_DIV_MAX {
            self.period_div = self.period();
            self.duty_step += 1;
        } else {
            self.period_div += 1;
        }
    }
}

#[bitsize(8)]
#[derive(Default, FromBits, DebugBits, Clone, Copy)]
struct Nr30 {
    __: u7,
    dac_enabled: bool,
}

#[bitsize(8)]
#[derive(Default, FromBits, DebugBits, Clone, Copy)]
struct Nr32 {
    _unused1: u5,
    output_level: u2,
    _unused2: u1,
}

#[derive(Default)]
struct WaveChannel {
    wave_ram: [u8; 16],
    index: Wrapping<u8>,
    period_div: u16,
    length_timer: u16,
    nr30: Nr30,
    nr31: u8,
    nr32: Nr32,
    nr33: u8,
    nr34: Nrx4,
    enabled: bool,
}

impl WaveChannel {
    const LENGTH_TIMER_MAX: u16 = 0xff;

    fn period(&self) -> u16 {
        ((self.nr34.period_high().value() as u16) << 8) | self.nr33 as u16
    }

    fn wave(&self) -> u8 {
        let index = self.index.0 & 0x1f;
        let val = self.wave_ram[index as usize >> 1];
        if index & 0x1 == 0 {
            val >> 4
        } else {
            val & 0xf
        }
    }

    fn dac_enabled(&self) -> bool {
        self.nr30.dac_enabled()
    }

    fn sample(&self) -> (u8, u8) {
        if !self.enabled {
            return (0, 0);
        }

        let output_level = self.nr32.output_level().value();
        if output_level > 0 {
            (self.wave() >> (output_level - 1), 0xf)
        } else {
            (0, 0)
        }
    }

    fn length_clock(&mut self) {
        if !self.nr34.sound_length_enabled() {
            return;
        }
        if self.length_timer >= Self::LENGTH_TIMER_MAX {
            self.enabled = false;
        } else {
            self.length_timer += 1;
        }
    }

    fn clock(&mut self) {
        if self.nr34.trigger() {
            self.enabled |= self.nr30.dac_enabled();
            self.nr34.set_trigger(false);
            self.period_div = self.period();
            self.length_timer = self.nr31 as u16;
            self.index.0 = 0;
        }

        if self.period_div >= PERIOD_DIV_MAX {
            self.period_div = self.period();
            self.index += 1;
        } else {
            self.period_div += 1;
        }
    }
}

#[bitsize(8)]
#[derive(Default, FromBits, DebugBits, Clone, Copy)]
struct Nr41 {
    initial_length_timer: u6,
    __: u2,
}

#[bitsize(8)]
#[derive(Default, FromBits, DebugBits, Clone, Copy)]
struct Nr43 {
    clock_divider: u3,
    short_mode: bool,
    clock_shift: u4,
}

#[bitsize(8)]
#[derive(Default, FromBits, DebugBits, Clone, Copy)]
struct Nr44 {
    __: u6,
    sound_length_enabled: bool,
    trigger: bool,
}

#[derive(Default)]
struct NoiseChannel {
    period_div: u16,
    length_timer: u8,
    envelope: Envelope,
    lfsr: u16,
    nr41: Nr41,
    nr42: Nrx2,
    nr43: Nr43,
    nr44: Nr44,
    enabled: bool,
}

impl NoiseChannel {
    const LENGTH_TIMER_MAX: u8 = 0x40;

    fn dac_enabled(&self) -> bool {
        self.nr42.initial_volume().value() != 0 || self.nr42.increase_envelope()
    }

    fn period(&self) -> u16 {
        let r = self.nr43.clock_divider().value() as u16;
        let base = if r == 0 { 2 } else { 4 * r };
        base << self.nr43.clock_shift().value()
    }

    fn sample(&self) -> (u8, u8) {
        if !self.enabled {
            return (0, 0);
        }

        let wave = self.lfsr & 0x1 != 0;

        let sample = if wave { self.envelope.volume } else { 0 };
        (sample, self.envelope.volume)
    }

    fn length_clock(&mut self) {
        if !self.nr44.sound_length_enabled() {
            return;
        }
        if self.length_timer >= Self::LENGTH_TIMER_MAX {
            self.enabled = false;
        } else {
            self.length_timer += 1;
        }
    }

    fn envelope_clock(&mut self) {
        self.envelope.clock();
    }

    fn lfsr_clock(&mut self) {
        let feedback = !(self.lfsr ^ (self.lfsr >> 1)) & 0x1;

        let mask = if self.nr43.short_mode() {
            0x8080
        } else {
            0x8000
        };

        self.lfsr &= !mask;
        self.lfsr |= feedback * mask;
        self.lfsr >>= 1;
    }

    fn clock(&mut self) {
        if self.nr44.trigger() {
            self.nr44.set_trigger(false);
            self.enabled = true;
            self.period_div = self.period();
            self.length_timer = self.nr41.initial_length_timer().value();
            self.envelope = self.nr42.into();
            self.lfsr = 0;
        }

        if self.period_div == 0 {
            self.period_div = self.period();
            self.lfsr_clock();
        } else {
            self.period_div -= 1;
        }
    }
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
        self.ch1.sweeper.nr10 = Nr10::from(nr10)
    }

    pub fn nr11(&self) -> u8 {
        let mut nrx1 = self.ch1.nrx1;
        nrx1.set_initial_length_timer(Default::default());
        nrx1.into()
    }

    pub fn set_nr11(&mut self, nr11: u8) {
        self.ch1.nrx1 = Nrx1::from(nr11);
    }

    pub fn nr12(&self) -> u8 {
        self.ch1.nrx2.into()
    }

    pub fn set_nr12(&mut self, nr12: u8) {
        self.ch1.nrx2 = Nrx2::from(nr12);
    }

    pub fn nr13(&self) -> u8 {
        self.ch1.nrx3.into()
    }

    pub fn set_nr13(&mut self, nr13: u8) {
        self.ch1.nrx3 = nr13;
    }

    pub fn nr14(&self) -> u8 {
        let mut nrx4 = Nrx4::default();
        nrx4.set_sound_length_enabled(self.ch1.nrx4.sound_length_enabled());
        nrx4.into()
    }

    pub fn set_nr14(&mut self, nr14: u8) {
        self.ch1.nrx4 = Nrx4::from(nr14);
    }

    pub fn nr21(&self) -> u8 {
        let mut nrx1 = self.ch1.nrx1;
        nrx1.set_initial_length_timer(Default::default());
        nrx1.into()
    }

    pub fn set_nr21(&mut self, nr21: u8) {
        self.ch2.nrx1 = Nrx1::from(nr21);
    }

    pub fn nr22(&self) -> u8 {
        self.ch2.nrx2.into()
    }

    pub fn set_nr22(&mut self, nr22: u8) {
        self.ch2.nrx2 = Nrx2::from(nr22);
    }

    pub fn nr23(&self) -> u8 {
        self.ch2.nrx3.into()
    }

    pub fn set_nr23(&mut self, nr23: u8) {
        self.ch2.nrx3 = nr23;
    }

    pub fn nr24(&self) -> u8 {
        let mut nrx4 = Nrx4::default();
        nrx4.set_sound_length_enabled(self.ch2.nrx4.sound_length_enabled());
        nrx4.into()
    }

    pub fn set_nr24(&mut self, nr24: u8) {
        self.ch2.nrx4 = Nrx4::from(nr24);
    }

    pub fn nr30(&self) -> u8 {
        self.ch3.nr30.into()
    }

    pub fn set_nr30(&mut self, nr30: u8) {
        self.ch3.nr30 = nr30.into();
        self.ch3.enabled &= self.ch3.dac_enabled();
    }

    pub fn nr31(&self) -> u8 {
        self.ch3.nr31
    }

    pub fn set_nr31(&mut self, nr31: u8) {
        self.ch3.nr31 = nr31;
    }

    pub fn nr32(&self) -> u8 {
        self.ch3.nr32.into()
    }

    pub fn set_nr32(&mut self, nr32: u8) {
        self.ch3.nr32 = nr32.into();
    }

    pub fn nr33(&self) -> u8 {
        self.ch3.nr33
    }

    pub fn set_nr33(&mut self, nr33: u8) {
        self.ch3.nr33 = nr33;
    }

    pub fn nr34(&self) -> u8 {
        let mut nr34 = Nrx4::default();
        nr34.set_sound_length_enabled(self.ch3.nr34.sound_length_enabled());
        nr34.into()
    }

    pub fn set_nr34(&mut self, nr34: u8) {
        self.ch3.nr34 = nr34.into();
    }

    pub fn set_nr41(&mut self, nr41: u8) {
        self.ch4.nr41 = Nr41::from(nr41);
    }

    pub fn nr42(&self) -> u8 {
        self.ch4.nr42.into()
    }

    pub fn set_nr42(&mut self, nr42: u8) {
        self.ch4.nr42 = Nrx2::from(nr42);
    }

    pub fn nr43(&self) -> u8 {
        self.ch4.nr43.into()
    }

    pub fn set_nr43(&mut self, nr43: u8) {
        self.ch4.nr43 = nr43.into();
    }

    pub fn nr44(&self) -> u8 {
        let mut nrx4 = Nrx4::default();
        nrx4.set_sound_length_enabled(self.ch4.nr44.sound_length_enabled());
        nrx4.into()
    }

    pub fn set_nr44(&mut self, nr44: u8) {
        self.ch4.nr44 = Nr44::from(nr44);
    }

    pub fn set_nr50(&mut self, nr50: u8) {
        self.nr50 = Nr50::from(nr50);
    }

    pub fn nr50(&self) -> u8 {
        self.nr50.into()
    }

    pub fn set_nr51(&mut self, nr51: u8) {
        self.nr51 = Nr51::from(nr51);
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
        nr52.set_channel_1_enabled(self.ch1.enabled);
        nr52.set_channel_2_enabled(self.ch2.enabled);
        nr52.set_channel_2_enabled(self.ch3.enabled);
        nr52.into()
    }

    pub fn read_wave_ram(&self, addr: u16) -> u8 {
        let offset = if self.ch3.enabled {
            (self.ch3.index.0 >> 1) as usize
        } else {
            addr as usize
        } & 0xf;
        self.ch3.wave_ram[offset]
    }

    pub fn write_wave_ram(&mut self, addr: u16, val: u8) {
        let offset = if self.ch3.enabled {
            (self.ch3.index.0 >> 1) as usize
        } else {
            addr as usize
        } & 0xf;
        self.ch3.wave_ram[offset] = val;
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
