// Terminal.rs - Low level mangement of the terminal
use termion::raw::{IntoRawMode, RawTerminal};
use termion::terminal_size;
use std::io::{stdout, Write};

// Holds the information on the terminal
pub struct Terminal {
    _stdout: RawTerminal<std::io::Stdout>,
    pub width: u16,
    pub height: u16,
}

impl Terminal {
    pub fn new() -> Self {
        // Create a new terminal instance and enter raw mode
        let _stdout = stdout().into_raw_mode().unwrap();
        let (w, h) = terminal_size().unwrap();
        Self {
            _stdout,
            width: w,
            height: h,
        }
    }
    pub fn clear_all(&self) {
        // Clear the entire screen
        print!("{}", termion::clear::All);
    }
    pub fn clear_line(&self) {
        // Clear the current line
        print!("{}", termion::clear::CurrentLine);
    }
    pub fn move_cursor(&self, mut x: u16, mut y: u16) {
        // Move the cursor to a specific point
        x = x.saturating_add(1);
        y = y.saturating_add(1);
        print!("{}", termion::cursor::Goto(x, y));
    }
    pub fn flush(&self) {
        // Flush the terminal
        stdout().flush().unwrap();
    }
    pub fn check_resize(&mut self) {
        // Check if the terminal has resized
        let (w, h) = terminal_size().unwrap();
        if self.height != h || self.width != w {
            self.height = h;
            self.width = w;
        }
    }
}
