// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023 Robert Hrusecky <jadedpastabowl@gmail.com>

use egui::{Context, Window};
use file_dialog::FileDialog;
use winit::event_loop::EventLoopProxy;

use crate::{background, event::FrontendEvent};

pub struct Ui {
    window_open: bool,
    file_dialog: FileDialog,
}

impl Ui {
    pub fn new() -> Self {
        Self {
            window_open: true,
            file_dialog: FileDialog::new().unwrap(),
        }
    }

    pub fn update(&mut self, ctx: &Context, proxy: &EventLoopProxy<FrontendEvent>) {
        Window::new("Hello egui!")
            .open(&mut self.window_open)
            .show(ctx, |ui| {
                ui.label("This is a test program for egui.");
                if ui.button("Load ROM").clicked() {
                    self.file_dialog.open().unwrap();
                }
            });

        self.file_dialog.show(ctx);

        if let Some(file) = self.file_dialog.file() {
            let proxy = proxy.clone();
            background::spawn(async move {
                let data = file.read().await.unwrap();
                let _ = proxy.send_event(FrontendEvent::NewRom(data));
            });
        }
    }
}
