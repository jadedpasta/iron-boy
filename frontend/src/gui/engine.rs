// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023 Robert Hrusecky <jadedpastabowl@gmail.com>

use std::mem;

use anyhow::Result;
use egui::{ClippedPrimitive, Context, TexturesDelta};
use egui_wgpu::{
    renderer::ScreenDescriptor,
    wgpu::{
        CommandEncoder, Device, LoadOp, Operations, Queue, RenderPassColorAttachment,
        RenderPassDescriptor, TextureFormat, TextureView,
    },
    Renderer,
};
use egui_winit::State;
use winit::{
    event::WindowEvent,
    event_loop::{EventLoop, EventLoopProxy},
    window::Window,
};

use crate::event::FrontendEvent;

use super::ui::Ui;

pub struct GuiEngine {
    egui_ctx: Context,
    egui_state: State,
    screen_descriptor: ScreenDescriptor,
    renderer: Renderer,
    textures: TexturesDelta,
    paint_jobs: Vec<ClippedPrimitive>,
    pub ui: Ui,
}

impl GuiEngine {
    pub fn new<T>(
        event_loop: &EventLoop<T>,
        width: u32,
        height: u32,
        scale_factor: f32,
        device: &Device,
        texture_format: TextureFormat,
    ) -> Result<GuiEngine> {
        let max_texture_size = device.limits().max_texture_dimension_2d as usize;

        let egui_ctx = Context::default();
        let mut egui_state = State::new(&event_loop);
        egui_state.set_max_texture_side(max_texture_size);
        egui_state.set_pixels_per_point(scale_factor);
        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [width, height],
            pixels_per_point: scale_factor,
        };
        let renderer = Renderer::new(device, texture_format, None, 1);

        Ok(Self {
            egui_ctx,
            egui_state,
            screen_descriptor,
            renderer,
            textures: Default::default(),
            paint_jobs: Vec::new(),
            ui: Ui::new()?,
        })
    }

    pub fn handle_event(&mut self, event: &WindowEvent) -> bool {
        self.egui_state.on_event(&self.egui_ctx, event).consumed
    }

    pub fn update(&mut self, window: &Window, proxy: &EventLoopProxy<FrontendEvent>) -> Result<()> {
        let raw_input = self.egui_state.take_egui_input(window);
        let mut result = Ok(());
        let output = self
            .egui_ctx
            .run(raw_input, |ctx| result = self.ui.update(ctx, proxy));
        result?;

        self.textures.append(output.textures_delta);
        self.egui_state
            .handle_platform_output(window, &self.egui_ctx, output.platform_output);
        self.paint_jobs = self.egui_ctx.tessellate(output.shapes);
        Ok(())
    }

    pub fn render(
        &mut self,
        encoder: &mut CommandEncoder,
        render_target: &TextureView,
        device: &Device,
        queue: &Queue,
    ) {
        for (id, image_delta) in &self.textures.set {
            self.renderer
                .update_texture(device, queue, *id, image_delta);
        }

        self.renderer.update_buffers(
            device,
            queue,
            encoder,
            &self.paint_jobs,
            &self.screen_descriptor,
        );

        {
            let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("egui"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: render_target,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Load,
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            self.renderer
                .render(&mut rpass, &self.paint_jobs, &self.screen_descriptor);
        }

        // Cleanup
        let textures = mem::take(&mut self.textures);
        for id in &textures.free {
            self.renderer.free_texture(id);
        }
    }

    pub fn set_scale_factor(&mut self, scale_factor: f64) {
        self.screen_descriptor.pixels_per_point = scale_factor as f32;
    }

    pub fn resize(&mut self, size: [u32; 2]) {
        if size.iter().all(|s| *s > 0) {
            self.screen_descriptor.size_in_pixels = size;
        }
    }
}
