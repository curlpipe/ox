#![warn(clippy::all, clippy::pedantic)]
#![allow(clippy::cast_possible_truncation)]

mod buffer;
mod editor;
mod row;
mod terminal;

use buffer::Buffer;
use editor::Editor;
use row::Row;
use std::io::ErrorKind;
use std::panic;
use std::thread;
use std::time::Duration;
use terminal::Terminal;

fn main() {
    let result = panic::catch_unwind(|| match Editor::new() {
        Ok(mut editor) => editor.run(),
        Err(err) => match err.kind() {
            ErrorKind::NotFound => println!("File not found"),
            _ => println!("An error occured"),
        },
    });
    if result.is_err() {
        thread::sleep(Duration::from_secs(5));
    }
}
