mod buffer;
mod editor;
mod terminal;
use buffer::Buffer;
use editor::Editor;
use terminal::Terminal;

use std::io::ErrorKind;

fn main() {
    match Editor::new() {
        Ok(mut editor) => editor.run(),
        Err(err) => match err.kind() {
            ErrorKind::NotFound => println!("File not found"),
            _ => println!("An error occured"),
        },
    }
}
