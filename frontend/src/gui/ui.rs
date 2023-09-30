// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023 Robert Hrusecky <jadedpastabowl@gmail.com>

use anyhow::{Context as _, Error, Result};
use egui::{Context, Id, Window};
use file_dialog::FileDialog;
use winit::event_loop::EventLoopProxy;

use crate::{background, event::FrontendEvent};

struct ErrorWindow {
    open: bool,
    error: Error,
}

pub struct Ui {
    window_open: bool,
    file_dialog: FileDialog,
    errors: Vec<ErrorWindow>,
}

impl Ui {
    pub fn new() -> Result<Self> {
        Ok(Self {
            window_open: true,
            file_dialog: FileDialog::new().context("Failed to initalize file dialog")?,
            errors: Vec::new(),
        })
    }

    pub fn add_error_popup(&mut self, error: Error) {
        self.errors.push(ErrorWindow { open: true, error });
    }

    fn show_errors(&mut self, ctx: &Context) {
        let mut i = 0;
        while i < self.errors.len() {
            let ErrorWindow { error, open } = &mut self.errors[i];
            let id = Id::new(&**error as *const _);
            Window::new("⚠ Error").id(id).open(open).show(ctx, |ui| {
                ui.label(format!("{error:#}"));
            });

            // HACK: If the window is closed, it still needs to show the close animation before we remove
            // it from the list. Grab the progress of the internal close animation and check if it
            // has completed.
            if *open || ctx.animate_bool(id.with("close_animation"), false) > 0.0 {
                i += 1;
            } else {
                // Error window was closed, remove it from the list. Replace the current error with
                // the "last" one.
                let last_error = self.errors.pop().unwrap();
                if let Some(error) = self.errors.get_mut(i) {
                    *error = last_error;
                }
            }
        }
    }

    pub fn update(&mut self, ctx: &Context, proxy: &EventLoopProxy<FrontendEvent>) -> Result<()> {
        let mut result = Ok(());
        Window::new("Hello egui!")
            .open(&mut self.window_open)
            .show(ctx, |ui| {
                ui.label("This is a test program for egui.");
                if ui.button("Load ROM").clicked() {
                    result = self
                        .file_dialog
                        .open()
                        .context("Failed to open file dialog");
                }
            });

        self.file_dialog.show(ctx);

        if let Some(file) = self.file_dialog.file() {
            let proxy = proxy.clone();
            background::spawn(async move {
                let event = match file.read().await.context("Failed to read ROM file") {
                    Ok(data) => FrontendEvent::NewRom(data),
                    Err(error) => FrontendEvent::Error(error),
                };
                let _ = proxy.send_event(event);
            });
        }

        self.show_errors(ctx);

        result.map_err(From::from)
    }
}
