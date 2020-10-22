// Terminal.rs - Handling low level terminal operations
use crate::util::Exp;
use crate::Position;
use std::io::{stdout, Error, Stdout, Write};
use termion::raw::{IntoRawMode, RawTerminal};
use termion::screen::AlternateScreen;
use unicode_width::UnicodeWidthStr;

// Struct to hold size
pub struct Size {
    pub width: usize,
    pub height: usize,
}

// The terminal struct
pub struct Terminal {
    screen: AlternateScreen<std::io::Stdout>, // Holds the screen
    _stdout: RawTerminal<Stdout>,             // Ensures we're in raw mode for total control
    pub size: Size,                           // For holding the size of the terminal
    regex: Exp,                               // For holding the regex
}

// Implement methods into the terminal struct / class
impl Terminal {
    pub fn new() -> Result<Self, Error> {
        // Create a new terminal and switch into raw mode
        let size = termion::terminal_size()?;
        Ok(Self {
            screen: AlternateScreen::from(stdout()),
            _stdout: stdout().into_raw_mode()?,
            size: Size {
                width: size.0 as usize,
                height: size.1 as usize,
            },
            regex: Exp::new(),
        })
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
    pub fn hide_cursor(&mut self) {
        write!(self.screen, "{}", termion::cursor::Hide).unwrap();
    }
    pub fn show_cursor(&mut self) {
        write!(self.screen, "{}", termion::cursor::Show).unwrap();
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
    pub fn check_resize(&mut self) -> bool {
        // Check for and handle resize events
        let size = termion::terminal_size().unwrap();
        if size == (self.size.width as u16, self.size.height as u16) {
            false
        } else {
            self.size = Size {
                width: size.0 as usize,
                height: size.1 as usize,
            };
            true
        }
    }
}
