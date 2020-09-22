// Terminal.rs - Handling low level terminal operations
use crate::Position; // Allow use and handling of positions
use regex::Regex; // Regular expression engine
use std::io::{stdout, Stdout, Write}; // For writing to the stdout
use termion::raw::{IntoRawMode, RawTerminal}; // To access raw mode
use termion::screen::AlternateScreen; // To render to a separate screen
use termion::{async_stdin, AsyncReader}; // To read keys asynchronously
use unicode_width::UnicodeWidthStr; // To find the width of unicode strings

// Our terminal struct
pub struct Terminal {
    screen: AlternateScreen<std::io::Stdout>, // Holds the screen
    _stdout: RawTerminal<Stdout>,             // Ensures we're in raw mode for total control
    pub stdin: AsyncReader,                   // Asynchronous stdin
    pub width: u16,                           // Width of the terminal
    pub height: u16,                          // Height of the terminal
    ansi_regex: Regex,                        // For holding the regex expression
}

// Implement methods into the terminal struct / class
impl Terminal {
    pub fn new() -> Self {
        // Create a new terminal and switch into raw mode
        let size = termion::terminal_size().unwrap();
        Self {
            screen: AlternateScreen::from(stdout()),
            _stdout: stdout().into_raw_mode().unwrap(),
            stdin: async_stdin(),
            width: size.0,
            height: size.1,
            ansi_regex: Regex::new(r"\u{1b}\[[0-?]*[ -/]*[@-~]").unwrap(),
        }
    }
    pub fn goto(&mut self, p: &Position) {
        // Move the cursor to a position
        write!(
            self.screen,
            "{}",
            termion::cursor::Goto(p.x.saturating_add(1) as u16, p.y.saturating_add(1) as u16)
        )
        .unwrap();
    }
    pub fn flush(&mut self) {
        // Flush the screen to prevent weird behaviour
        self.screen.flush().unwrap();
    }
    pub fn align_break(&self, l: &str, r: &str) -> String {
        // Align two items to the left and right
        let left_length = UnicodeWidthStr::width(l);
        let right_length = UnicodeWidthStr::width(r);
        let padding = (self.width as usize).saturating_sub(left_length + right_length);
        " ".repeat(padding as usize)
    }
    pub fn align_left(&self, text: &str) -> String {
        // Align items to the left
        let length = self.no_ansi_len(text);
        let padding = (self.width as usize).saturating_sub(length);
        " ".repeat(padding as usize)
    }
    pub fn check_resize(&mut self) -> bool {
        // Check for and handle resize events
        let size = termion::terminal_size().unwrap();
        if size == (self.width, self.height) {
            false
        } else {
            self.width = size.0;
            self.height = size.1;
            true
        }
    }
    fn no_ansi_len(&self, data: &str) -> usize {
        // Find the length of a string without ANSI values
        let data = self.ansi_regex.replacen(data, 2, "");
        UnicodeWidthStr::width(&*data)
    }
}
