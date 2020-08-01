mod terminal;
mod buffer;
mod editor;
use terminal::Terminal;
use editor::Editor;
use buffer::Buffer;

use std::panic;

fn main() {
    let result = panic::catch_unwind(|| {
        let mut editor = Editor::new();
        editor.run();
    });
    if result.is_err() {
        std::thread::sleep(std::time::Duration::from_millis(5000));
    }
}
