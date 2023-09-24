// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023 Robert Hrusecky <jadedpastabowl@gmail.com>

use std::error::Error;

use instant::Instant;
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

#[cfg(target_arch = "wasm32")]
mod wasm {
    use std::rc::Rc;

    use wasm_bindgen::{prelude::Closure, JsCast};
    use winit::platform::web::WindowExtWebSys;
    use winit::{dpi::LogicalSize, window::Window};

    pub type EngineWindow = Rc<Window>;

    fn window_size() -> LogicalSize<f64> {
        let client_window = web_sys::window().unwrap();
        LogicalSize::new(
            client_window.inner_width().unwrap().as_f64().unwrap(),
            client_window.inner_height().unwrap().as_f64().unwrap(),
        )
    }

    pub fn attach_window(window: Window) -> EngineWindow {
        let result = Rc::new(window);
        let window = Rc::clone(&result);

        // Initialize winit window with current dimensions of browser client
        window.set_inner_size(window_size());

        let client_window = web_sys::window().unwrap();

        // Attach winit canvas to body element
        web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| doc.body())
            .and_then(|body| {
                body.append_child(&web_sys::Element::from(window.canvas()))
                    .ok()
            })
            .expect("couldn't append canvas to document body");

        // Listen for resize event on browser client. Adjust winit window dimensions
        // on event trigger
        let closure = Closure::wrap(Box::new(move |_e: web_sys::Event| {
            let size = window_size();
            window.set_inner_size(size)
        }) as Box<dyn FnMut(_)>);
        client_window
            .add_event_listener_with_callback("resize", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
        result
    }
}

#[cfg(target_arch = "wasm32")]
use wasm::EngineWindow;
#[cfg(not(target_arch = "wasm32"))]
type EngineWindow = Window;

pub struct Engine {
    gui: GuiEngine,
    audio: Audio,
    pixels: Pixels,
    cgb: Option<Cgb>,
    window: EngineWindow,
    options: Options,
}

impl Engine {
    pub async fn new<T>(
        event_loop: &EventLoop<T>,
        options: Options,
    ) -> Result<Self, Box<dyn Error>> {
        let size = LogicalSize::new(
            emulator::SCREEN_WIDTH as u16,
            emulator::SCREEN_HEIGHT as u16,
        );
        let window = WindowBuilder::new()
            .with_title("Iron Boy")
            .with_inner_size(size)
            .with_min_inner_size(size)
            .build(event_loop)?;

        #[cfg(target_arch = "wasm32")]
        let window = wasm::attach_window(window);

        let window_size = window.inner_size();
        let scale_factor = window.scale_factor() as f32;
        let pixels = {
            #[cfg(target_arch = "wasm32")]
            let window = &*window;
            #[cfg(not(target_arch = "wasm32"))]
            let window = &window;

            let surface_texture =
                SurfaceTexture::new(window_size.width, window_size.height, window);
            PixelsBuilder::new(
                emulator::SCREEN_WIDTH as u32,
                emulator::SCREEN_HEIGHT as u32,
                surface_texture,
            )
            .texture_format(TextureFormat::Rgba8Unorm)
            // .surface_texture_format(TextureFormat::Bgra8Unorm)
            .surface_texture_format(TextureFormat::Rgba8Unorm)
            .present_mode(PresentMode::Fifo)
            .build_async()
            .await?
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
        match event {
            Event::MainEventsCleared => {
                let now = Instant::now();
                let target = if let ControlFlow::WaitUntil(target) = *control_flow {
                    target
                } else {
                    now
                };
                if target > now {
                    // Not enough time has elapsed yet; nothing to do
                    return Ok(());
                }
                self.gui.update(&self.window);
                self.window.request_redraw();
                let Some(cgb) = &mut self.cgb else {
                    *control_flow = ControlFlow::Poll;
                    return Ok(());
                };
                let wakeup = target + cgb.compute_next_frame(&mut self.pixels, &mut self.audio);
                *control_flow = ControlFlow::WaitUntil(wakeup);
            }
            Event::RedrawRequested(window_id) if window_id == self.window.id() => {
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
