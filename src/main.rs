// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023 Robert Hrusecky <jadedpastabowl@gmail.com>

#![allow(clippy::new_without_default)]

mod apu;
mod audio;
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

use audio::Audio;
use cart::Cart;
use joypad::{Button, ButtonState};
use pixels::wgpu::{PresentMode, TextureFormat};
use pixels::{PixelsBuilder, SurfaceTexture};
use std::fs::File;
use std::path::Path;
use std::time::{Duration, Instant};
use std::{env, fs, mem};
use winit::dpi::LogicalSize;
use winit::event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;

use system::{CgbSystem, FrameBuffer};

struct Cgb {
    system: Box<CgbSystem>,
    audio: Audio,
}

impl Cgb {
    fn new(rom_file_name: impl AsRef<str>, audio: Audio) -> Self {
        let rom = fs::read(rom_file_name.as_ref()).unwrap();

        let mut cart = Cart::from_rom(rom.into_boxed_slice());
        if cart.battery_backed() {
            let save_path = Path::new(rom_file_name.as_ref());
            let save_path = save_path.with_extension("cart");
            if save_path.exists() {
                let save_file = File::open(save_path).unwrap();
                let save = bincode::deserialize_from(save_file).unwrap();
                cart.load_from_save(save);
            }
        }

        Self {
            system: CgbSystem::new(cart),
            audio,
        }
    }

    fn cart(&self) -> &Cart {
        self.system.cart()
    }

    fn compute_next_frame(&mut self, frame_buff: &mut FrameBuffer) -> Duration {
        self.audio.update_ratio();
        self.system
            .execute(frame_buff, |f| self.audio.push_frame(f))
            .into()
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
    let Some(button) = Button::from_keycode(key) else {
        return;
    };
    let state = ButtonState::from_state(state);
    cgb.handle_joypad(button, state);
}

fn main() {
    let file_name = env::args().nth(1).unwrap();

    let audio = audio::init().unwrap();
    let mut cgb = Cgb::new(file_name.clone(), audio);

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
                        if let Some(save) = cgb.cart().save() {
                            let path = Path::new(&file_name);
                            let path = path.with_extension("cart");
                            let save_file = File::create(path).unwrap();
                            bincode::serialize_into(save_file, &save).unwrap();
                        }
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
