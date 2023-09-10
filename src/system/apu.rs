// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023 Robert Hrusecky <jadedpastabowl@gmail.com>

use super::CgbSystem;
use crate::apu::ApuBus;
use partial_borrow::prelude::*;

impl ApuBus for partial!(CgbSystem ! apu) {
    fn div(&self) -> u8 {
        self.timer.div()
    }
}
