mod terminal;
mod buffer;
mod editor;

use terminal::Terminal;
use editor::Editor;
use buffer::Buffer;

fn main() {
    let mut editor = Editor::new();
    editor.run();
}
