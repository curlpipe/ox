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
*/

// Assemble the various modules
mod buffer;
mod editor;
mod row;
mod terminal;

// Bring in the relavent libraries
use buffer::Buffer;
use editor::Editor;
use row::Row;
use std::panic;
use std::thread;
use std::time::Duration;
use terminal::Terminal;

fn main() {
    // Create a new editor instance
    let result = panic::catch_unwind(|| match Editor::new() {
        Ok(mut editor) => editor.run(), // Start the editor when there's no error
        Err(err) => println!("{:?}", err.kind()), // Display error if there is an error.
    });

    if result.is_err() {
        // Wait 5 seconds after a runtime error to gather debug information.
        thread::sleep(Duration::from_secs(5));
    }
}
