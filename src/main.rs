mod terminal;
mod buffer;
mod editor;
use terminal::Terminal;
use editor::Editor;
use buffer::Buffer;

use std::io::ErrorKind;

fn main() {
    match Editor::new() {
        Ok(mut editor) => editor.run(),
        Err(err) => match err.kind() {
            ErrorKind::NotFound => println!("File not found"),
            _ => println!("An error occured"),
        }
    }
}
