// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023 Robert Hrusecky <jadedpastabowl@gmail.com>

use std::{
    cell::Cell,
    rc::Rc,
    sync::atomic::{AtomicBool, Ordering},
};

use egui::Context;
use futures::future::FutureExt;
use js_sys::{Promise, Uint8Array};
use thiserror::Error;
use wasm_bindgen::{prelude::Closure, JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    Document, DomException, File, FileReader, HtmlButtonElement, HtmlDialogElement, HtmlElement,
    HtmlFormElement, HtmlInputElement, ProgressEvent,
};

const DEFAULT_STYLE_CSS: &str = include_str!("../style.css");

#[derive(Error, Debug)]
#[error("JavaScript exception: {0}")]
pub struct JsError(Box<str>);

impl From<JsValue> for JsError {
    fn from(value: JsValue) -> Self {
        Self(format!("{value:?}").into_boxed_str())
    }
}

impl From<DomException> for JsError {
    fn from(e: DomException) -> Self {
        Self(e.message().into_boxed_str())
    }
}

#[derive(Debug)]
pub struct FileHandle {
    file: File,
    progress: Rc<Cell<f64>>,
}

pub type ReadError = JsError;

impl FileHandle {
    fn new(file: File) -> Self {
        Self {
            file,
            progress: Rc::new(Cell::new(0.0)),
        }
    }

    pub fn progress(&self) -> f64 {
        self.progress.get()
    }

    pub async fn read(&self) -> Result<Box<[u8]>, ReadError> {
        let reader = FileReader::new()?;

        let progress = Rc::clone(&self.progress);
        let progress_callback = Closure::<dyn FnMut(_)>::new(move |e: ProgressEvent| {
            progress.set(e.loaded() / e.total());
        });

        reader.set_onprogress(Some(progress_callback.as_ref().unchecked_ref()));

        let load: JsFuture = Promise::new(&mut |resolve, _| {
            reader.set_onload(Some(&resolve));
        })
        .into();
        let error: JsFuture = Promise::new(&mut |resolve, _| {
            reader.set_onerror(Some(&resolve));
        })
        .into();

        reader.read_as_array_buffer(&self.file)?;

        futures::select! { _ = load.fuse() => (), _ = error.fuse() => () }

        if let Some(error) = reader.error() {
            return Err(error.into());
        }

        let result = Uint8Array::new(&reader.result()?);
        let result = result.to_vec().into_boxed_slice();

        self.progress.set(1.0);

        Ok(result)
    }
}

static STYLESHEET_INJECTED: AtomicBool = AtomicBool::new(false);

pub struct FileDialog {
    dialog: HtmlDialogElement,
    input: HtmlInputElement,
}

#[derive(Error, Debug)]
pub enum NewDialogError {
    #[error("failed to access DOM")]
    Dom,
    #[error("{0}")]
    Js(#[from] JsError),
}

pub type OpenDialogError = JsError;
pub type FileAsyncDialogError = JsError;

impl FileDialog {
    fn new_on_element(document: Document, body: HtmlElement) -> Result<Self, JsError> {
        // <dialog class="file-dialog">
        let dialog: HtmlDialogElement = document.create_element("dialog")?.unchecked_into();
        dialog.set_class_name("file-dialog");

        // <form method="dialog">
        let form: HtmlFormElement = document.create_element("form")?.unchecked_into();
        form.set_method("dialog");

        // <label>Choose a file</label>
        let title = document.create_element("label")?;
        title.set_inner_html("Choose a file");
        form.append_child(&title)?;

        // <input type="file">
        let input: HtmlInputElement = document.create_element("input")?.unchecked_into();
        input.set_type("file");
        form.append_child(&input)?;

        // <div>
        let div = document.create_element("div")?;

        // <button value="cancel">Cancel</button>
        let cancel: HtmlButtonElement = document.create_element("button")?.unchecked_into();
        cancel.set_value("cancel");
        cancel.set_inner_html("Cancel");
        div.append_child(&cancel)?;

        // <button value="confirm">Confirm</button>
        let confirm: HtmlButtonElement = document.create_element("button")?.unchecked_into();
        confirm.set_value("confirm");
        confirm.set_inner_html("Confirm");
        div.append_child(&confirm)?;

        form.append_child(&div)?;
        // </div>

        dialog.append_child(&form)?;
        // </form>

        body.append_child(&dialog)?;
        // </dialog>

        Ok(FileDialog { dialog, input })
    }

    pub fn new() -> Result<Self, NewDialogError> {
        let window = web_sys::window().ok_or(NewDialogError::Dom)?;
        let document = window.document().ok_or(NewDialogError::Dom)?;
        let body = document.body().ok_or(NewDialogError::Dom)?;

        if !STYLESHEET_INJECTED.swap(true, Ordering::Relaxed) {
            let head = document.head().ok_or(NewDialogError::Dom)?;
            let style = document.create_element("style").map_err(JsError::from)?;
            style.set_inner_html(DEFAULT_STYLE_CSS);
            head.append_child(&style).map_err(JsError::from)?;
        }

        Ok(Self::new_on_element(document, body)?)
    }

    pub fn open(&self) -> Result<(), OpenDialogError> {
        self.dialog.show_modal()?;
        Ok(())
    }

    pub fn file(&self) -> Option<FileHandle> {
        if self.dialog.open() || self.dialog.return_value() == "cancel" {
            return None;
        }
        // Ensure that subsequent calls return `None`; we want to "consume" the file
        self.dialog.set_return_value("cancel");
        self.input.files()?.item(0).map(FileHandle::new)
    }

    pub async fn file_async(&self) -> Result<Option<FileHandle>, FileAsyncDialogError> {
        self.open()?;
        let promise = Promise::new(&mut |resolve, reject| {
            if let Err(e) = self
                .dialog
                .add_event_listener_with_callback("close", &resolve)
            {
                reject.call1(&JsValue::undefined(), &e).unwrap();
            }
        });
        JsFuture::from(promise).await?;
        Ok(self.file())
    }

    pub fn show(&mut self, _ctx: &Context) {
        // Nothing to do, browser does rendering
    }

    pub fn close(&self) {
        self.dialog.close();
    }
}

impl Drop for FileDialog {
    fn drop(&mut self) {
        self.dialog.remove();
    }
}
