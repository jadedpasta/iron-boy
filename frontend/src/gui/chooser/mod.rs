// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023 Robert Hrusecky <jadedpastabowl@gmail.com>

#[cfg(target_family = "wasm")]
mod web;
#[cfg(target_family = "wasm")]
pub use web::*;

#[cfg(not(target_family = "wasm"))]
mod desktop;
#[cfg(not(target_family = "wasm"))]
pub use desktop::*;

mod util {
    use anyhow::Context;
    use file_dialog::FileHandle;
    use winit::event_loop::EventLoopProxy;

    use crate::{background, event::FrontendEvent};

    pub fn spawn_file_read(file: FileHandle, proxy: &EventLoopProxy<FrontendEvent>) {
        let proxy = proxy.clone();
        background::spawn(async move {
            let event = match file.read().await.context("Failed to read ROM file") {
                Ok(data) => FrontendEvent::NewRom(data),
                Err(error) => FrontendEvent::Error(error),
            };
            let _ = proxy.send_event(event);
        });
    }
}
