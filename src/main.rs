// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023 Robert Hrusecky <jadedpastabowl@gmail.com>

#![allow(clippy::new_without_default)]

mod cart;
mod cpu;
mod dma;
mod interrupt;
mod joypad;
mod memory;
mod ppu;
mod reg;
mod system;
mod timer;

use cart::Cart;
use joypad::{Button, ButtonState};
use pixels::wgpu::{PresentMode, TextureFormat};
use pixels::{PixelsBuilder, SurfaceTexture};
use std::time::{Duration, Instant};
use std::{env, fs, mem};
use winit::dpi::LogicalSize;
use winit::event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;

use system::{CgbSystem, FrameBuffer};

struct Cgb {
    system: Box<CgbSystem>,
}

impl Cgb {
    fn new(rom_file_name: impl AsRef<str>) -> Self {
        let rom = fs::read(rom_file_name.as_ref()).unwrap();
        let cart = Cart::from_rom(rom.into_boxed_slice());
        Self {
            system: CgbSystem::new(cart),
        }
    }

    fn compute_next_frame(&mut self, frame_buff: &mut FrameBuffer) -> Duration {
        self.system.execute(frame_buff).into()
    }

    fn into_frame_buffer_ref(buff: &mut [u8]) -> Option<&mut FrameBuffer> {
        let buff: &mut [u8; mem::size_of::<FrameBuffer>()] = buff.try_into().ok()?;
        Some(unsafe { mem::transmute(buff) })
    }

    fn handle_joypad(&mut self, button: Button, state: ButtonState) {
        self.system.handle_joypad(button, state);
    }
}

fn handle_key(cgb: &mut Cgb, key: VirtualKeyCode, state: ElementState) {
    let Some(button) = Button::from_keycode(key) else { return; };
    let state = ButtonState::from_state(state);
    cgb.handle_joypad(button, state);
}

fn main() {
    let file_name = env::args().nth(1).unwrap();

    let mut cgb = Cgb::new(file_name);

    let event_loop = EventLoop::new();

    let size = LogicalSize::new(system::SCREEN_WIDTH as u16, system::SCREEN_HEIGHT as u16);

    let window = WindowBuilder::new()
        .with_title("Iron Boy")
        .with_inner_size(size)
        .with_min_inner_size(size)
        .build(&event_loop)
        .unwrap();

    let mut pixels = {
        let window_size = window.inner_size();
        let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
        PixelsBuilder::new(
            system::SCREEN_WIDTH as u32,
            system::SCREEN_HEIGHT as u32,
            surface_texture,
        )
        .texture_format(TextureFormat::Rgba8Unorm)
        .surface_texture_format(TextureFormat::Bgra8Unorm)
        .present_mode(PresentMode::Fifo)
        .build()
        .unwrap()
    };

    event_loop.run(move |event, _, control_flow| {
        let now = Instant::now();
        let last = if let ControlFlow::WaitUntil(instant) = *control_flow {
            instant
        } else {
            now
        };

        match event {
            Event::MainEventsCleared => {
                if last > now {
                    // Not enough time has elapsed yet; nothing to do
                    return;
                }
                window.request_redraw();
                let frame_buffer = Cgb::into_frame_buffer_ref(pixels.frame_mut()).unwrap();
                let wakeup = last + cgb.compute_next_frame(frame_buffer);
                *control_flow = ControlFlow::WaitUntil(wakeup);
            }
            Event::RedrawRequested(window_id) if window_id == window.id() => {
                pixels.render().unwrap();
            }
            Event::WindowEvent { window_id, event } if window_id == window.id() => match event {
                WindowEvent::Resized(size) => {
                    pixels.resize_surface(size.width, size.height).unwrap()
                }
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            virtual_keycode: Some(virtual_keycode),
                            state,
                            ..
                        },
                    ..
                } => match (virtual_keycode, state) {
                    (VirtualKeyCode::Escape, ElementState::Released) => {
                        *control_flow = ControlFlow::Exit
                    }
                    (key, state) => handle_key(&mut cgb, key, state),
                },
                _ => (),
            },
            _ => (),
        }
    });
}
