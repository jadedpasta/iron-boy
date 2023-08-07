mod cpu;
mod dma;
mod interrupt;
mod joypad;
mod memory;
mod ppu;
mod reg;
mod system;
mod timer;

use joypad::{Button, ButtonState};
use pixels::wgpu::TextureFormat;
use pixels::{PixelsBuilder, SurfaceTexture};
use std::time::{Duration, Instant};
use std::{env, fs, mem};
use winit::dpi::LogicalSize;
use winit::event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;

use system::CgbSystem;

struct Cgb {
    system: Box<CgbSystem>,
}
type FrameBuffer = [[[u8; 4]; Cgb::SCREEN_WIDTH]; Cgb::SCREEN_HEIGHT];

impl Cgb {
    const FRAME_TIME: Duration = Duration::from_nanos(16742706);
    const SCREEN_WIDTH: usize = 160;
    const SCREEN_HEIGHT: usize = 144;
    const VBLANK_LINES: usize = 10;
    const FRAME_LINES: usize = Self::SCREEN_HEIGHT + Self::VBLANK_LINES;
    const DOTS_PER_LINE: usize = 456;
    const DOTS_PER_FRAME: usize = Self::FRAME_LINES * Self::DOTS_PER_LINE;

    fn new(rom_file_name: impl AsRef<str>) -> Self {
        let rom = fs::read(rom_file_name.as_ref()).unwrap();
        Self { system: CgbSystem::new(rom) }
    }

    fn compute_next_frame(&mut self, frame_buff: &mut FrameBuffer) {
        let lcd_on = self.system.lcd_on();
        for _ in 0..Self::DOTS_PER_FRAME / 4 {
            let (ppu, bus) = self.system.split_ppu();
            ppu.execute(frame_buff, bus);
            let (dma, bus) = self.system.split_dma();
            dma.execute(bus);
            let (cpu, bus) = self.system.split_cpu();
            cpu.execute(bus);
            let (timer, bus) = self.system.split_timer();
            timer.execute(bus);
            if !lcd_on && self.system.lcd_on() {
                break;
            }
        }

        if !lcd_on {
            *frame_buff = unsafe { std::mem::MaybeUninit::zeroed().assume_init() };
        }
    }

    fn into_frame_buffer_ref(buff: &mut [u8]) -> Option<&mut FrameBuffer> {
        let buff: &mut [u8; 4 * Self::SCREEN_WIDTH * Self::SCREEN_HEIGHT] = buff.try_into().ok()?;
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

    let size = LogicalSize::new(Cgb::SCREEN_WIDTH as u16, Cgb::SCREEN_HEIGHT as u16);

    let window = WindowBuilder::new()
        .with_title("Iron Boy")
        .with_inner_size(size)
        .with_min_inner_size(size)
        .build(&event_loop)
        .unwrap();

    let mut pixels = {
        let window_size = window.inner_size();
        let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
        PixelsBuilder::new(Cgb::SCREEN_WIDTH as u32, Cgb::SCREEN_HEIGHT as u32, surface_texture)
            .texture_format(TextureFormat::Rgba8Unorm)
            .surface_texture_format(TextureFormat::Bgra8Unorm)
            .build()
            .unwrap()
    };

    event_loop.run(move |event, _, control_flow| {
        let now = Instant::now();
        let last = if let ControlFlow::WaitUntil(instant) = *control_flow { instant } else { now };

        match event {
            Event::MainEventsCleared => {
                if last > now {
                    // Not enough time has elapsed yet; nothing to do
                    return;
                }
                let frame_buffer = Cgb::into_frame_buffer_ref(pixels.frame_mut()).unwrap();
                cgb.compute_next_frame(frame_buffer);
                *control_flow = ControlFlow::WaitUntil(last + Cgb::FRAME_TIME);
                window.request_redraw();
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
                    input: KeyboardInput { virtual_keycode: Some(virtual_keycode), state, .. },
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
