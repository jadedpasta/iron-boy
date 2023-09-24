// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023 Robert Hrusecky <jadedpastabowl@gmail.com>

#![allow(clippy::new_without_default)]

mod audio;
mod emulator;
mod engine;
mod gui;
mod options;

use engine::Engine;
use options::Options;
use winit::event_loop::{ControlFlow, EventLoop};

async fn run(options: Options) {
    let event_loop = EventLoop::new();

    let mut engine = Engine::new(&event_loop, options)
        .await
        .expect("Error while initializing");

    event_loop.run(move |event, _, control_flow| {
        if let Err(e) = engine.handle_event(event, control_flow) {
            eprintln!("Error while running event loop: {e:?}");
            *control_flow = ControlFlow::ExitWithCode(1);
        }
    });
}

fn main() {
    #[cfg(target_arch = "wasm32")]
    {
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        console_log::init_with_level(log::Level::Warn).expect("error initalizing logger");
        wasm_bindgen_futures::spawn_local(run(Default::default()));
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        use clap::Parser;
        let options = Options::parse();
        env_logger::init();
        pollster::block_on(run(options));
    }
}
