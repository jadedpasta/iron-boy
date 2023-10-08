// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023 Robert Hrusecky <jadedpastabowl@gmail.com>

use std::{
    char,
    ffi::{OsStr, OsString},
    mem,
    ops::Range,
    os::unix::{ffi::OsStrExt, prelude::OsStringExt},
    str,
};

use egui::TextBuffer;

struct Utf8Chunks<'a> {
    remaining: &'a [u8],
}

impl<'a> Utf8Chunks<'a> {
    fn has_remaining(self) -> bool {
        !self.remaining.is_empty()
    }
}

impl<'a> Iterator for Utf8Chunks<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        while !self.remaining.is_empty() {
            let (s, skip) = match str::from_utf8(self.remaining) {
                Ok(s) => (s, s.len()),
                Err(e) => {
                    let valid = e.valid_up_to();
                    let mut skip = valid + e.error_len().unwrap_or_default();
                    if skip == self.remaining.len() {
                        // Don't skip over the last error
                        skip = valid;
                    }
                    (
                        unsafe { str::from_utf8_unchecked(&self.remaining[..valid]) },
                        skip,
                    )
                }
            };

            if skip == 0 {
                // We're stuck at an ending error
                return None;
            }

            self.remaining = &self.remaining[skip..];

            if !s.is_empty() {
                return Some(s);
            }
        }
        None
    }
}

#[derive(Default)]
pub struct OsStrTextBuffer {
    str: Vec<u8>,
    view: String,
}

impl OsStrTextBuffer {
    fn update_view(&mut self) {
        self.view.clear();

        let mut chunks = Utf8Chunks {
            remaining: &self.str,
        };

        for chunk in chunks.by_ref() {
            self.view.push_str(chunk);
            self.view.push(char::REPLACEMENT_CHARACTER);
        }

        if !chunks.has_remaining() {
            // We don't have an error at the end; remove the trailing replacement char.
            self.view.pop();
        }
    }

    pub fn clone_as_os_string(&mut self) -> OsString {
        OsString::from_vec(self.str.clone())
    }
}

impl TextBuffer for OsStrTextBuffer {
    fn is_mutable(&self) -> bool {
        true
    }

    fn as_str(&self) -> &str {
        &self.view
    }

    fn byte_index_from_char_index(&self, char_index: usize) -> usize {
        let chunks = Utf8Chunks {
            remaining: &self.str,
        };

        let mut i = 0;
        for chunk in chunks {
            for (byte_index, _) in chunk.char_indices() {
                if i == char_index {
                    return unsafe { chunk.as_ptr().offset_from(self.str.as_ptr()) } as usize
                        + byte_index;
                }
                i += 1;
            }
            // The error counts as a char
            if i == char_index {
                return unsafe { chunk.as_ptr().offset_from(self.str.as_ptr()) } as usize
                    + chunk.len();
            }
            i += 1;
        }

        self.str.len()
    }

    fn insert_text(&mut self, text: &str, char_index: usize) -> usize {
        let i = self.byte_index_from_char_index(char_index);
        let text = text.as_bytes();
        self.str.splice(i..i, text.iter().copied());
        self.update_view();
        text.len()
    }

    fn delete_char_range(&mut self, char_range: Range<usize>) {
        assert!(char_range.start <= char_range.end);
        let start = self.byte_index_from_char_index(char_range.start);
        let end = self.byte_index_from_char_index(char_range.end);
        self.str.splice(start..end, []);
        self.update_view();
    }

    fn char_range(&self, char_range: Range<usize>) -> &str {
        assert!(char_range.start <= char_range.end);
        let view = self.as_str();
        let start_byte = view.byte_index_from_char_index(char_range.start);
        let end_byte = view.byte_index_from_char_index(char_range.end);
        &view[start_byte..end_byte]
    }

    fn clear(&mut self) {
        self.str.clear();
        self.view.clear();
    }

    fn replace(&mut self, text: &str) {
        self.str.clear();
        self.str.extend_from_slice(text.as_bytes());
        self.update_view();
    }

    fn take(&mut self) -> String {
        self.str.clear();
        mem::take(&mut self.view)
    }
}

impl<T> From<T> for OsStrTextBuffer
where
    T: AsRef<OsStr>,
{
    fn from(str: T) -> Self {
        let str = str.as_ref();
        let mut result = Self {
            str: str.as_bytes().into(),
            view: String::new(),
        };
        result.update_view();
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty() {
        assert_eq!(OsStrTextBuffer::default().as_str(), "");
    }

    #[test]
    fn insert() {
        let mut buff = OsStrTextBuffer::default();
        buff.insert_text("Hello", 0);
        assert_eq!(buff.as_str(), "Hello");
        let pos = "Hello".len();
        buff.insert_text("world!", pos);
        assert_eq!(buff.as_str(), "Helloworld!");
        buff.insert_text(", ", pos);
        assert_eq!(buff.as_str(), "Hello, world!");
    }

    #[test]
    fn delete() {
        let mut buff = OsStrTextBuffer::from(OsStr::from_bytes(b"Hello, world!"));
        let pos = "Hello".len();
        buff.delete_char_range(pos..pos + 2);
        assert_eq!(buff.as_str(), "Helloworld!");
    }

    #[test]
    fn bad_unicode() {
        let mut buff = OsStrTextBuffer::from(OsStr::from_bytes(b"Hello\xff\xffworld!"));
        assert_eq!(buff.as_str(), "Hello\u{fffd}world!");
        let pos = b"Hello\xff".len();
        buff.insert_text(", ", pos);
        assert_eq!(buff.as_str(), "Hello\u{fffd}, world!");
        buff.delete_char_range(pos - 1..pos + 2);
        assert_eq!(buff.as_str(), "Helloworld!");
    }

    #[test]
    fn bad_unicode_end() {
        let mut buff = OsStrTextBuffer::from(OsStr::from_bytes(b"Hello, world!\xff\xff"));
        assert_eq!(buff.as_str(), "Hello, world!\u{fffd}");
        buff.insert_text(" hey", buff.as_str().len());
        assert_eq!(buff.as_str(), "Hello, world!\u{fffd} hey");
        buff = OsStrTextBuffer::from(OsStr::from_bytes(b"Hello, world!\xff\xff"));
        let len = "Hello, world!".len();
        buff.delete_char_range(len..len + 1);
        assert_eq!(buff.as_str(), "Hello, world!");
    }

    #[test]
    fn bad_unicode_continuation_end() {
        let mut buff = OsStrTextBuffer::from(OsStr::from_bytes(b"Hello, world!\xdf"));
        assert_eq!(buff.as_str(), "Hello, world!\u{fffd}");
        buff.insert_text(" hey", buff.as_str().len());
        assert_eq!(buff.as_str(), "Hello, world!\u{fffd} hey");
        buff = OsStrTextBuffer::from(OsStr::from_bytes(b"Hello, world!\xdf"));
        let len = "Hello, world!".len();
        buff.delete_char_range(len..len + 1);
        assert_eq!(buff.as_str(), "Hello, world!");
    }
}
