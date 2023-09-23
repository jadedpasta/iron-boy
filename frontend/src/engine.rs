// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023 Robert Hrusecky <jadedpastabowl@gmail.com>

use std::{error::Error, time::Instant};

use pixels::{
    wgpu::{PresentMode, TextureFormat},
    Pixels, PixelsBuilder, SurfaceTexture,
};
use winit::{
    dpi::LogicalSize,
    event::{Event, KeyboardInput, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

use crate::{
    audio::{self, Audio},
    emulator::{self, Cgb},
    gui::GuiEngine,
    options::Options,
};

pub struct Engine {
    gui: GuiEngine,
    audio: Audio,
    pixels: Pixels,
    cgb: Option<Cgb>,
    window: Window,
    options: Options,
}

impl Engine {
    pub fn new<T>(event_loop: &EventLoop<T>, options: Options) -> Result<Self, Box<dyn Error>> {
        let size = LogicalSize::new(
            emulator::SCREEN_WIDTH as u16,
            emulator::SCREEN_HEIGHT as u16,
        );
        let window = WindowBuilder::new()
            .with_title("Iron Boy")
            .with_inner_size(size)
            .with_min_inner_size(size)
            .build(event_loop)?;

        let window_size = window.inner_size();
        let scale_factor = window.scale_factor() as f32;
        let pixels = {
            let surface_texture =
                SurfaceTexture::new(window_size.width, window_size.height, &window);
            PixelsBuilder::new(
                emulator::SCREEN_WIDTH as u32,
                emulator::SCREEN_HEIGHT as u32,
                surface_texture,
            )
            .texture_format(TextureFormat::Rgba8Unorm)
            .surface_texture_format(TextureFormat::Bgra8Unorm)
            .present_mode(PresentMode::Fifo)
            .build()?
        };

        let gui = GuiEngine::new(
            event_loop,
            window_size.width,
            window_size.height,
            scale_factor,
            pixels.device(),
            pixels.render_texture_format(),
        );

        Ok(Self {
            gui,
            window,
            audio: audio::init()?,
            pixels,
            cgb: Cgb::new(&options).ok(),
            options,
        })
    }

    pub fn handle_event<T>(
        &mut self,
        event: Event<T>,
        control_flow: &mut ControlFlow,
    ) -> Result<(), Box<dyn Error>> {
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
                    return Ok(());
                }
                let Some(cgb) = &mut self.cgb else {
                    return Ok(());
                };
                self.window.request_redraw();
                let wakeup = last + cgb.compute_next_frame(&mut self.pixels, &mut self.audio);
                *control_flow = ControlFlow::WaitUntil(wakeup);
            }
            Event::RedrawRequested(window_id) if window_id == self.window.id() => {
                self.gui.update(&self.window);
                self.pixels
                    .render_with(|encoder, render_target, context| {
                        context.scaling_renderer.render(encoder, render_target);

                        self.gui
                            .render(encoder, render_target, &context.device, &context.queue);

                        Ok(())
                    })
                    .unwrap();
            }
            Event::WindowEvent { window_id, event }
                if window_id == self.window.id() && !self.gui.handle_event(&event) =>
            {
                match event {
                    WindowEvent::CloseRequested => {
                        if let Some(cgb) = &mut self.cgb {
                            cgb.handle_close(&self.options)?;
                        }
                        *control_flow = ControlFlow::Exit;
                        return Ok(());
                    }
                    WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                        self.gui.set_scale_factor(scale_factor);
                    }
                    WindowEvent::Resized(size) => {
                        self.pixels.resize_surface(size.width, size.height)?;
                        self.gui.resize(size.into());
                    }
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                virtual_keycode: Some(key),
                                state,
                                ..
                            },
                        ..
                    } => {
                        if let Some(cgb) = &mut self.cgb {
                            cgb.handle_key(key, state)
                        }
                    }
                    _ => (),
                }
            }
            _ => (),
        }
        Ok(())
    }
}
