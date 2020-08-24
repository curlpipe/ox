#![warn(clippy::all, clippy::pedantic)]
#![allow(clippy::cast_possible_truncation, clippy::used_underscore_binding)]

/*
    Ox editor is a text editor written in the Rust programming language.

    It runs in the terminal and provides keyboard shortcuts to interact.
    This removes the need for a mouse which can slow down editing files.
    I have documented this code where necessary and it has been formatted
    with Rustfmt to ensure clean and consistent style throughout.

    More information here:
    https://rust-lang.org
    https://github.com/rust-lang/rustfmt
    https://github.com/curlpipe/ox
*/

// Bring in the external modules
mod document;
mod editor;
mod row;
mod terminal;

use document::Document;
use editor::{Editor, Position};
use row::Row;
use std::time::Duration;
use std::{panic, thread};
use terminal::Terminal;

fn main() {
    // Attempt to start an editor instance
    let result = panic::catch_unwind(|| {
        let mut editor = Editor::new();
        editor.run();
    });
    // Check to see if the editor exited because of a runtime issue
    if result.is_err() {
        // Pause for a few seconds to catch debug information
        thread::sleep(Duration::from_secs(5));
    }
}
