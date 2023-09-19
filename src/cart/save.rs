// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023 Robert Hrusecky <jadedpastabowl@gmail.com>

use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};

use super::{Cart, Mbc};

#[derive(Serialize, Deserialize)]
pub struct RtcSave {
    pub base: SystemTime,
    pub latched: Duration,
    pub day_carry: bool,
    pub halted: Option<SystemTime>,
}

#[derive(Serialize, Deserialize)]
pub enum MbcSave {
    None,
    Rtc(RtcSave),
}

#[derive(Serialize, Deserialize)]
pub struct CartSave {
    pub mbc: MbcSave,
    pub ram: Box<[u8]>,
}

impl From<Cart> for CartSave {
    fn from(cart: Cart) -> Self {
        Self {
            mbc: cart.mbc.save(),
            ram: cart.mem.ram.raw(),
        }
    }
}
