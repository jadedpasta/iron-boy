// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023 Robert Hrusecky <jadedpastabowl@gmail.com>

use anyhow::{Context as _, Result};
use egui::{Align, Context, Layout, Ui};
use file_dialog::{FileDialog, FileHandle};
use winit::event_loop::EventLoopProxy;

use crate::event::FrontendEvent;

use super::util;

pub struct RomChooser {
    file_dialog: FileDialog,
    file: Option<FileHandle>,
}

impl RomChooser {
    pub fn new() -> Result<Self> {
        Ok(Self {
            file_dialog: FileDialog::new().context("Failed to initalize file dialog")?,
            file: None,
        })
    }

    pub fn show_dialog(&mut self, ctx: &Context, proxy: &EventLoopProxy<FrontendEvent>) {
        self.file_dialog.show(ctx);

        if let Some(file) = self.file_dialog.file() {
            self.file = Some(file.clone());
            util::spawn_file_read(file, proxy);
        }
    }

    pub fn show(&mut self, ui: &mut Ui, proxy: &EventLoopProxy<FrontendEvent>) -> Result<()> {
        let mut result = Ok(());

        let row = (ui.available_size().x, ui.spacing().interact_size.y).into();

        ui.allocate_ui_with_layout(row, Layout::right_to_left(Align::Center), |ui| {
            if ui.button("Reset").clicked() {
                if let Some(file) = &self.file {
                    util::spawn_file_read(file.clone(), proxy);
                }
            }
            if ui.button("Browse...").clicked() {
                result = self
                    .file_dialog
                    .open()
                    .context("Failed to open file dialog");
            }
            let mut text = self.file.as_ref().map(|f| f.name()).unwrap_or_default();
            ui.centered_and_justified(|ui| ui.text_edit_singleline(&mut text));
        });

        result
    }
}
