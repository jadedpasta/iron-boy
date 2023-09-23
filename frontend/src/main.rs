// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023 Robert Hrusecky <jadedpastabowl@gmail.com>

#![allow(clippy::new_without_default)]

mod audio;
mod emulator;
mod engine;
mod gui;
mod options;

use clap::Parser;
use engine::Engine;
use options::Options;
use winit::event_loop::{ControlFlow, EventLoop};

fn main() {
    let options = Options::parse();
    env_logger::init();

    let event_loop = EventLoop::new();

    let mut engine = Engine::new(&event_loop, options).expect("Error while initializing");

    event_loop.run(move |event, _, control_flow| {
        if let Err(e) = engine.handle_event(event, control_flow) {
            eprintln!("Error while running event loop: {e:?}");
            *control_flow = ControlFlow::Exit;
        }
    });
}
