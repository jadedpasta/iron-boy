// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023 Robert Hrusecky <jadedpastabowl@gmail.com>

use std::path::PathBuf;

use anyhow::{Context as _, Result};
use egui::{Align, Layout, Ui};
use egui_osstr::OsStrTextBuffer;
use file_dialog::FileDialog;
use winit::event_loop::EventLoopProxy;

use crate::event::FrontendEvent;

use super::util;

pub struct RomChooser {
    file_dialog: FileDialog,
    rom_path: OsStrTextBuffer,
}

impl RomChooser {
    pub fn new() -> Result<Self> {
        Ok(Self {
            file_dialog: FileDialog::new().context("Failed to initalize file dialog")?,
            rom_path: Default::default(),
        })
    }

    pub fn show_dialog(&mut self, ctx: &egui::Context, proxy: &EventLoopProxy<FrontendEvent>) {
        self.file_dialog.show(ctx);
        if let Some(file) = self.file_dialog.file() {
            self.rom_path = file.name().into();
            util::spawn_file_read(file, proxy);
        }
    }

    pub fn show(&mut self, ui: &mut Ui, proxy: &EventLoopProxy<FrontendEvent>) -> Result<()> {
        let mut result = Ok(());

        let row = (ui.available_size().x, ui.spacing().interact_size.y).into();

        ui.allocate_ui_with_layout(row, Layout::right_to_left(Align::Center), |ui| {
            if ui.button("Reset").clicked() {
                let path = PathBuf::from(self.rom_path.clone_as_os_string());
                util::spawn_file_read(path.into(), proxy);
            }
            if ui.button("Browse...").clicked() {
                result = self
                    .file_dialog
                    .open()
                    .context("Failed to open file dialog");
            }
            ui.centered_and_justified(|ui| ui.text_edit_singleline(&mut self.rom_path));
        });

        result
    }
}
