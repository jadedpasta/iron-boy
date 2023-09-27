// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023 Robert Hrusecky <jadedpastabowl@gmail.com>

#[cfg(target_arch = "wasm32")]
pub use wasm_bindgen_futures::spawn_local as spawn;

#[cfg(not(target_arch = "wasm32"))]
mod desktop {
    use std::future::Future;

    #[inline]
    pub fn spawn(future: impl Future<Output = ()> + Send + 'static) {
        tokio::spawn(future);
    }
}
#[cfg(not(target_arch = "wasm32"))]
pub use desktop::*;
