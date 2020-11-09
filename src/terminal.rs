// Terminal.rs - Handling low level terminal operations
use crate::util::Exp;
use crate::Position;
use crossterm::terminal;
use crossterm::{execute, ErrorKind};
use std::io::{stdout, Write};
use unicode_width::UnicodeWidthStr;

// Struct to hold size
pub struct Size {
    pub width: usize,
    pub height: usize,
}

// The terminal struct
pub struct Terminal {
    pub size: Size, // For holding the size of the terminal
    regex: Exp,     // For holding the regex
}

// Implement methods into the terminal struct / class
impl Terminal {
    pub fn new() -> Result<Self, ErrorKind> {
        // Create a new terminal and switch into raw mode
        let size = terminal::size()?;
        terminal::enable_raw_mode()?;
        execute!(stdout(), terminal::EnterAlternateScreen)?;
        Ok(Self {
            size: Size {
                width: size.0 as usize,
                height: size.1 as usize,
            },
            regex: Exp::new(),
        })
    }
    pub fn exit() {
        execute!(stdout(), terminal::LeaveAlternateScreen).unwrap();
        terminal::disable_raw_mode().unwrap();
    }
    pub fn goto(&mut self, p: &Position) {
        // Move the cursor to a position
        write!(
            stdout(),
            "{}",
            crossterm::cursor::MoveTo(p.x as u16, p.y as u16)
        )
        .unwrap();
    }
    pub fn flush(&mut self) {
        // Flush the screen to prevent weird behaviour
        stdout().flush().unwrap();
    }
    pub fn hide_cursor(&mut self) {
        write!(stdout(), "{}", crossterm::cursor::Hide).unwrap();
    }
    pub fn show_cursor(&mut self) {
        write!(stdout(), "{}", crossterm::cursor::Show).unwrap();
    }
    pub fn align_break(&self, l: &str, r: &str) -> String {
        // Align two items to the left and right
        let left_length = UnicodeWidthStr::width(l);
        let right_length = UnicodeWidthStr::width(r);
        let padding = (self.size.width as usize).saturating_sub(left_length + right_length);
        " ".repeat(padding as usize)
    }
    pub fn align_left(&self, text: &str) -> String {
        // Align items to the left
        let length = self.regex.ansi_len(text);
        let padding = (self.size.width as usize).saturating_sub(length);
        " ".repeat(padding as usize)
    }
}
