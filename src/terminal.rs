// Terminal.rs - Low level mangement of the terminal
use termion::raw::{IntoRawMode, RawTerminal};
use termion::input::TermRead;
use termion::event::Key;
use termion::terminal_size;
use std::io::{stdin, stdout, Write};

pub struct Terminal {
    _stdout: RawTerminal<std::io::Stdout>,
    pub width: u16,
    pub height: u16,
}

impl Terminal {
    pub fn new() -> Self {
        let _stdout = stdout().into_raw_mode().unwrap();
        let (w, h) = terminal_size().unwrap();
        Self {
            _stdout,
            width: w,
            height: h,
        }
    }
    pub fn read_key(&self) -> Result<Key, std::io::Error> {
        loop { if let Some(key) = stdin().lock().keys().next() 
             { return key; } }
    }
    pub fn clear_all(&self) {
        print!("{}", termion::clear::All);
    }
    pub fn clear_line(&self) {
        print!("{}", termion::clear::CurrentLine);
    }
    pub fn move_cursor(&self, mut x: u16, mut y: u16) {
        x = x.saturating_add(1);
        y = y.saturating_add(1);
        print!("{}", termion::cursor::Goto(x, y));
    }
    pub fn flush(&self) {
        stdout().flush().unwrap();
    }
    pub fn check_resize(&mut self) {
        let (w, h) = terminal_size().unwrap();
        if self.height != h || self.width != w {
            self.height = h;
            self.width = w;
        }
    }
}
