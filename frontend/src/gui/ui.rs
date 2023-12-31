// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023 Robert Hrusecky <jadedpastabowl@gmail.com>

use anyhow::{Error, Result};
use egui::{Context, Frame, Grid, Id, InnerResponse, Margin, SidePanel, TopBottomPanel, Window};
use winit::event_loop::EventLoopProxy;

use crate::event::FrontendEvent;

use super::chooser::RomChooser;

struct ErrorWindow {
    open: bool,
    error: Error,
}

pub struct Ui {
    panel_open: bool,
    rom_chooser: RomChooser,
    errors: Vec<ErrorWindow>,
}

impl Ui {
    pub fn new() -> Result<Self> {
        Ok(Self {
            panel_open: true,
            rom_chooser: RomChooser::new()?,
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
        if let Some(pos) = ctx.input(|i| i.pointer.interact_pos()) {
            if pos.x < ctx.screen_rect().width() * 0.05 {
                self.panel_open = true;
            }
        }
        let resp = SidePanel::left("options panel")
            .frame(Frame::side_top_panel(&ctx.style()).inner_margin(Margin::same(10.0)))
            .show_animated(ctx, self.panel_open, |ui| {
                ui.heading("Iron Boy");
                ui.separator();

                result = self.rom_chooser.show(ui, proxy);

                TopBottomPanel::bottom("controls panel")
                    .frame(Frame::none())
                    .show_separator_line(false)
                    .resizable(false)
                    .show_inside(ui, |ui| {
                        ui.heading("Controls");
                        ui.separator();
                        Grid::new("controls table")
                            .striped(true)
                            .num_columns(2)
                            .show(ui, |ui| {
                                ui.monospace("WASD");
                                ui.horizontal(|ui| {
                                    ui.label("Joy Pad");
                                    // Force stripes to take up the whole width
                                    ui.add_space(ui.available_width());
                                });
                                ui.end_row();
                                ui.monospace("<");
                                ui.label("A");
                                ui.end_row();
                                ui.monospace(">");
                                ui.label("B");
                                ui.end_row();
                                ui.monospace("[");
                                ui.label("Start");
                                ui.end_row();
                                ui.monospace("]");
                                ui.label("Select");
                            });
                    });
            });

        if let (Some(InnerResponse { response, .. }), Some(pos)) = (
            resp,
            ctx.input(|i| {
                if i.pointer.any_click() {
                    i.pointer.interact_pos()
                } else {
                    None
                }
            }),
        ) {
            if !response.rect.contains(pos) {
                self.panel_open = false;
            }
        }

        self.rom_chooser.show_dialog(ctx, proxy);

        self.show_errors(ctx);

        result.map_err(From::from)
    }
}
