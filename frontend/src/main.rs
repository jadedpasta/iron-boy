// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023 Robert Hrusecky <jadedpastabowl@gmail.com>

#![allow(clippy::new_without_default)]

mod audio;
mod background;
mod emulator;
mod engine;
mod event;
mod gui;
mod options;

use engine::Engine;
use event::FrontendEvent;
use options::Options;
use winit::event_loop::{EventLoop, EventLoopBuilder};

async fn init(options: Options) -> (EventLoop<FrontendEvent>, Engine) {
    let event_loop = EventLoopBuilder::with_user_event().build();

    let engine = Engine::new(&event_loop, options)
        .await
        .expect("Error while initializing");
    (event_loop, engine)
}

fn run(event_loop: EventLoop<FrontendEvent>, mut engine: Engine) {
    event_loop.run(move |event, _, control_flow| engine.handle_event(event, control_flow));
}

fn main() {
    #[cfg(target_arch = "wasm32")]
    {
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        console_log::init_with_level(log::Level::Warn).expect("error initalizing logger");
        wasm_bindgen_futures::spawn_local(async {
            let (event_loop, engine) = init(Default::default()).await;
            run(event_loop, engine)
        });
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        use clap::Parser;
        let options = Options::parse();
        env_logger::init();
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .worker_threads(1)
            .build()
            .expect("Error initializing tokio runtime");
        let (event_loop, engine) = rt.block_on(init(options));
        let _guard = rt.enter();
        run(event_loop, engine);
    }
}
