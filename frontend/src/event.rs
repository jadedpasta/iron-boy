// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023 Robert Hrusecky <jadedpastabowl@gmail.com>

pub enum FrontendEvent {
    NewRom(Box<[u8]>),
}
