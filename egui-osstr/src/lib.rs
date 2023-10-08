// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023 Robert Hrusecky <jadedpastabowl@gmail.com>

#[cfg(unix)]
mod utf8;
#[cfg(unix)]
pub use utf8::*;
