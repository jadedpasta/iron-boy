// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023 Robert Hrusecky <jadedpastabowl@gmail.com>

use std::{
    convert::Infallible,
    io::{self, SeekFrom},
    path::Path,
};

use egui::Context;
use thiserror::Error;
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncSeekExt},
};

#[derive(Error, Debug)]
pub enum Error {}

pub struct FileHandle {
    path: Box<Path>,
}

pub type ReadError = io::Error;

impl FileHandle {
    fn new(path: &Path) -> Self {
        Self { path: path.into() }
    }

    // pub fn progress(&self) -> f64 {
    //     self.progress.get()
    // }

    pub async fn read(&self) -> Result<Box<[u8]>, ReadError> {
        let mut file = File::open(&self.path).await?;
        let size = file.seek(SeekFrom::End(0)).await?;
        file.seek(SeekFrom::Start(0)).await?;
        let mut buf = Vec::with_capacity(size.try_into().unwrap());
        while file.read_buf(&mut buf).await? != 0 {
            // TODO: progress
        }
        Ok(buf.into_boxed_slice())
    }
}

pub struct FileDialog {
    dialog: egui_file::FileDialog,
}

pub type NewDialogError = Infallible;
pub type OpenDialogError = Infallible;

impl FileDialog {
    pub fn new() -> Result<Self, NewDialogError> {
        Ok(Self {
            dialog: egui_file::FileDialog::open_file(None),
        })
    }

    pub fn open(&mut self) -> Result<(), OpenDialogError> {
        Ok(self.dialog.open())
    }

    pub fn file(&mut self) -> Option<FileHandle> {
        if !self.dialog.selected() {
            return None;
        }
        let file = self.dialog.path().map(FileHandle::new);
        *self = unsafe { Self::new().unwrap_unchecked() };
        file
    }

    pub fn show(&mut self, ctx: &Context) {
        self.dialog.show(ctx);
    }

    // pub async fn file_async(&self) -> Result<Option<FileHandle>, Error> {
    // }

    // pub fn close(&self) {
    // }
}
