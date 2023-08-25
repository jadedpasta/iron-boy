// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023 Robert Hrusecky <jadedpastabowl@gmail.com>

use bilge::prelude::*;

use std::time::{Duration, Instant};

const SECONDS_PER_MINUTE: u64 = 60;
const MINUTES_PER_HOUR: u64 = 60;
const HOURS_PER_DAY: u64 = 24;
const SECONDS_PER_HOUR: u64 = SECONDS_PER_MINUTE * MINUTES_PER_HOUR;
const SECONDS_PER_DAY: u64 = SECONDS_PER_HOUR * HOURS_PER_DAY;

struct Counter {
    base: Instant,
    halted: Option<Instant>,
}

impl Default for Counter {
    fn default() -> Self {
        Self {
            base: Instant::now(),
            halted: None,
        }
    }
}

impl Counter {
    fn halt(&mut self) {
        if self.halted.is_none() {
            self.halted = Some(Instant::now());
        }
    }

    fn resume(&mut self) {
        if let Some(halted) = self.halted {
            self.base += Instant::now() - halted;
            self.halted = None;
        }
    }

    fn halted(&self) -> bool {
        self.halted.is_some()
    }

    fn set(&mut self, time: Duration) {
        let now = Instant::now();
        self.base = now - time;
        if let Some(halted) = &mut self.halted {
            *halted = now;
        }
    }

    fn get(&self) -> Duration {
        let end = self.halted.unwrap_or_else(Instant::now);
        end - self.base
    }
}

#[bitsize(8)]
#[derive(FromBits, DebugBits, DefaultBits, Clone, Copy)]
pub struct RtcFlags {
    day_msb: bool,
    __: u5,
    halt: bool,
    day_carry: bool,
}

#[derive(Default)]
pub struct Rtc {
    counter: Counter,
    latched: Duration,
    latch_signal: bool,
    day_carry: bool,
}

impl Rtc {
    pub fn seconds(&self) -> u64 {
        self.latched.as_secs() % SECONDS_PER_MINUTE
    }

    pub fn minutes(&self) -> u64 {
        self.latched.as_secs() / SECONDS_PER_MINUTE % MINUTES_PER_HOUR
    }

    pub fn hours(&self) -> u64 {
        self.latched.as_secs() / SECONDS_PER_HOUR % HOURS_PER_DAY
    }

    pub fn days(&self) -> u64 {
        self.latched.as_secs() / SECONDS_PER_DAY
    }

    pub fn flags(&self) -> RtcFlags {
        RtcFlags::new(
            self.days() & 0x100 != 0,
            u5::new(0),
            self.counter.halted(),
            self.day_carry,
        )
    }

    fn set<const SECS_PER_UNIT: u64, const MAX_UNIT: u64>(&mut self, units: u8) {
        if (units as u64) < MAX_UNIT {
            let current = self.counter.get();
            let current_units =
                Duration::from_secs(current.as_secs() / SECS_PER_UNIT % MAX_UNIT * SECS_PER_UNIT);
            let units = Duration::from_secs(units as u64 * SECS_PER_UNIT);
            self.counter.set(current - current_units + units);
        }
    }

    pub fn set_seconds(&mut self, seconds: u8) {
        self.set::<1, SECONDS_PER_MINUTE>(seconds);
    }

    pub fn set_minutes(&mut self, minutes: u8) {
        self.set::<SECONDS_PER_MINUTE, MINUTES_PER_HOUR>(minutes);
    }

    pub fn set_hours(&mut self, hours: u8) {
        self.set::<SECONDS_PER_HOUR, HOURS_PER_DAY>(hours);
    }

    pub fn set_days(&mut self, days: u8) {
        self.set::<SECONDS_PER_DAY, 0x100>(days);
    }

    pub fn set_flags(&mut self, flags: RtcFlags) {
        self.day_carry = flags.day_carry();
        if flags.halt() {
            self.counter.halt();
        } else {
            self.counter.resume();
        }

        let current = self.counter.get();
        let current_days = current.as_secs() / SECONDS_PER_DAY;
        let current_days_msb = ((current_days >> 8) & 0x1) as u32;
        let days256 = Duration::from_secs(SECONDS_PER_DAY * 256);
        self.counter
            .set(current + days256 * ((flags.day_msb() as u32) - current_days_msb))
    }

    pub fn latch(&mut self, high: bool) {
        if !self.latch_signal && high {
            self.latched = self.counter.get();
            if self.days() >= 512 {
                self.day_carry = true;
                // Move the base forward so we have the opportunity to overflow again
                self.counter.base += Duration::from_secs(SECONDS_PER_DAY * 512);
            }
        }
        self.latch_signal = high;
    }
}
