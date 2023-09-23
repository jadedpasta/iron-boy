// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023 Robert Hrusecky <jadedpastabowl@gmail.com>

use std::{
    error::Error,
    fs::{self, File},
    mem,
    time::Duration,
};

pub use iron_boy_core::system::{SCREEN_HEIGHT, SCREEN_WIDTH};

use iron_boy_core::{
    cart::Cart,
    joypad::{Button, ButtonState},
    system::{CgbSystem, FrameBuffer},
};
use pixels::Pixels;
use winit::event::{ElementState, VirtualKeyCode};

use crate::{audio::Audio, options::Options};

pub struct Cgb {
    system: Box<CgbSystem>,
}

impl Cgb {
    pub fn new(options: &Options) -> Result<Self, Box<dyn Error>> {
        let rom_file_name = options.rom_file_name.as_ref().ok_or("No ROM file")?;
        let rom = fs::read(rom_file_name)?;

        let mut cart = Cart::from_rom(rom.into_boxed_slice());
        if cart.battery_backed() {
            let save_path = rom_file_name.with_extension("cart");
            if save_path.exists() {
                let save_file = File::open(save_path)?;
                let save = bincode::deserialize_from(save_file)?;
                cart.load_from_save(save);
            }
        }

        Ok(Self {
            system: Box::new(CgbSystem::new(cart)),
        })
    }

    pub fn compute_next_frame(&mut self, pixels: &mut Pixels, audio: &mut Audio) -> Duration {
        let frame_buff = pixels.frame_mut();
        let frame_buff: &mut [u8; mem::size_of::<FrameBuffer>()] =
            frame_buff.try_into().ok().unwrap();
        let frame_buff = unsafe { mem::transmute(frame_buff) };
        audio.update_ratio();
        self.system
            .execute(frame_buff, |f| audio.push_frame(f))
            .into()
    }

    fn handle_joypad(&mut self, button: Button, state: ButtonState) {
        self.system.handle_joypad(button, state);
    }

    pub fn handle_key(&mut self, key: VirtualKeyCode, state: ElementState) {
        use VirtualKeyCode as VK;
        let button = match key {
            VK::W => Button::Up,
            VK::A => Button::Left,
            VK::S => Button::Down,
            VK::D => Button::Right,
            VK::LBracket => Button::Start,
            VK::RBracket => Button::Select,
            VK::Comma => Button::A,
            VK::Period => Button::B,
            _ => return,
        };
        let state = match state {
            ElementState::Pressed => ButtonState::Pressed,
            ElementState::Released => ButtonState::Released,
        };
        self.handle_joypad(button, state);
    }

    pub fn handle_close(&self, options: &Options) -> Result<(), Box<dyn Error>> {
        if let Some(save) = self.system.cart().save() {
            let path = options
                .rom_file_name
                .as_ref()
                .ok_or("No ROM file")?
                .with_extension("cart");
            let save_file = File::create(path)?;
            bincode::serialize_into(save_file, &save)?;
        }
        Ok(())
    }
}
