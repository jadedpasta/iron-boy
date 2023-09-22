// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023 Robert Hrusecky <jadedpastabowl@gmail.com>
use super::CgbSystem;
use crate::{interrupt::Interrupt, timer::TimerBus};
use partial_borrow::prelude::*;

impl TimerBus for partial!(CgbSystem ! timer, mut mem interrupt) {
    fn request_timer_interrupt(&mut self) {
        self.interrupt.request(Interrupt::Timer);
    }
}
