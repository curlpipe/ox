/*
    Terminal.rs - Low level mangement of the terminal

    This contains structs (classes) which allow moving of the cursor
    and clearing of the screen as well as handling resize events.

*/

use std::io::{stdout, Write}; // For writing to the terminal
use termion::raw::{IntoRawMode, RawTerminal}; // To gain control of the terminal
use termion::screen::AlternateScreen; // To gain access to a seperate screen
use termion::terminal_size; // To obtain the terminal size

// Holds the information on the terminal
pub struct Terminal {
    _stdout: RawTerminal<std::io::Stdout>, // The stdout that keeps us in raw mode
    pub screen: AlternateScreen<std::io::Stdout>, // The screen that stores our stdout
    pub width: u16,                        // The width of the terminal
    pub height: u16,                       // The height of the terminal
}

impl Terminal {
    pub fn new() -> Self {
        // Create a new terminal instance and enter raw mode
        let _stdout = stdout().into_raw_mode().unwrap();
        let (w, h) = terminal_size().unwrap();
        Self {
            screen: AlternateScreen::from(stdout()),
            _stdout,
            width: w,
            height: h,
        }
    }
    pub fn write(&mut self, w: &str) {
        // Write to the screen
        write!(self.screen, "{}", w).unwrap();
    }
    pub fn clear_all(&mut self) {
        // Clear the entire screen
        write!(self.screen, "{}", termion::clear::All).unwrap();
    }
    pub fn move_cursor(&mut self, mut x: u16, mut y: u16) {
        // Move the cursor to a specific point
        x = x.saturating_add(1);
        y = y.saturating_add(1);
        write!(self.screen, "{}", termion::cursor::Goto(x, y)).unwrap();
    }
    pub fn flush(&mut self) {
        // Flush the terminal
        self.screen.flush().unwrap();
    }
    pub fn check_resize(&mut self) -> bool {
        // Check if the terminal has resized
        let (w, h) = terminal_size().unwrap();
        if self.height != h || self.width != w {
            self.height = h;
            self.width = w;
            true
        } else {
            false
        }
    }
}
