// Editor.rs - For controling the current editor
use termion::{color, style};
use termion::input::TermRead;
use termion::event::Key;
use crate::Terminal;
use crate::Buffer;
use std::time::Duration;
use std::cmp::min;
use std::thread;
use std::env;

// Get the version of Ox
const VERSION: &str = env!("CARGO_PKG_VERSION");
const BG: color::Bg<color::Rgb> = color::Bg(color::Rgb(0, 175, 135));
const FG: color::Fg<color::Rgb> = color::Fg(color::Rgb(38, 38, 38));

// For holding the position and directions of the cursor
pub struct Cursor {
    x: u16,
    y: u16,
}

// For holding our editor information
pub struct Editor {
    terminal: Terminal,
    kill: bool,
    cursor: Cursor,
    buffer: Buffer,
    offset: u64, // NOTE: Up to u128 in future for longer files?
}

impl Editor {
    pub fn new() -> Self {
        // Create a new editor instance
        let args: Vec<String> = env::args().collect();
        let buffer: Buffer;
        if args.len() <= 1 { 
            buffer = Buffer::new();
        } else {
            buffer = Buffer::open(args[1].trim());
        }
        Self {
            terminal: Terminal::new(),
            kill: false,
            cursor: Cursor { x: 0, y: 0 },
            buffer,
            offset: 0,
        }

    }
    pub fn run(&mut self) {
        let mut stdin = termion::async_stdin().keys();
        // Run our editor
        loop {
            // Exit if required
            if self.kill { break; }
            // Render our interface
            self.render();
            // FPS cap to stop greedy CPU usage
            thread::sleep(Duration::from_millis(40));
            // Read a key
            match stdin.next() {
                Some(key) => match key.unwrap() {
                    Key::Ctrl('q') => self.kill = true, // Exit
                    Key::Left => {
                        // Move cursor to the left
                        self.cursor.x = self.cursor.x.saturating_sub(1);
                    }
                    Key::Right => {
                        // Move cursor to the right
                        if self.cursor.x < self.terminal.width.saturating_sub(1) {
                            self.cursor.x = self.cursor.x.saturating_add(1);
                            self.correct_line();
                        }
                    }
                    Key::Up => {
                        // Move cursor up
                        if self.cursor.y != 0 {
                            self.cursor.y = self.cursor.y.saturating_sub(1);
                            self.correct_line();
                        } else {
                            self.offset = self.offset.saturating_sub(1);
                        }
                    }
                    Key::Down => {
                        // Move cursor down
                        let buff_len = self.buffer.lines.len() as u64;
                        let mut proposed = self.cursor.y.saturating_add(1) as u64;
                        proposed += self.offset;
                        let max = self.terminal.height.saturating_sub(3);
                        if proposed < buff_len {
                            if self.cursor.y < max {
                                self.cursor.y = proposed as u16;
                                self.correct_line();
                            } else {
                                self.offset = self.offset.saturating_add(1);
                            }
                        }
                    }
                    Key::PageUp => {
                        // Move the cursor to the top of the terminal
                        self.cursor.y = 0;
                        self.correct_line();
                    }
                    Key::PageDown => {
                        // Move the cursor to the bottom of the buffer / terminal
                        let t = self.terminal.height.saturating_sub(3) as u16;
                        let b = self.buffer.lines.len().saturating_sub(1) as u16;
                        self.cursor.y = min(t, b);
                        self.correct_line();
                    }
                    Key::Home => {
                        // Move to the start of the current line
                        self.cursor.x = 0;
                    }
                    Key::End => {
                        // Move to the end of the current line
                        self.cursor.x = self.terminal.width.saturating_sub(1);
                        self.correct_line();
                    }
                    _ => (), // Unbound key
                }
                None => self.terminal.check_resize(), // Check for resize
            }
        }
    }
    fn correct_line(&mut self) {
        // Ensure that the cursor isn't out of bounds
        if self.buffer.lines.is_empty() { return; }
        let current = self.buffer.lines[self.cursor.y as usize].clone();
        if self.cursor.x > current.len() as u16 {
            self.cursor.x = current.len() as u16;
        }
    }
    fn render(&mut self) {
        // Render the rows
        let term_length = self.terminal.height;
        for row in 0..self.terminal.height {
            self.terminal.move_cursor(0, row);
            self.terminal.clear_line();
            let l: String;
            if row == self.terminal.height / 3 && self.buffer.lines.is_empty() {
                let welcome = format!("Ox editor v{}", VERSION);
                let pad = " ".repeat(self.terminal.width as usize / 2 
                                     - welcome.len() / 2);
                l = format!("{}{}{}", "~", pad, welcome);
            } else if row == (self.terminal.height / 3) + 2 && 
                self.buffer.lines.is_empty()  {
                let welcome = "A speedy editor built with Rust";
                let pad = " ".repeat(self.terminal.width as usize / 2 
                                     - welcome.len() / 2);
                l = format!("{}{}{}", "~", pad, welcome);
            } else if row == (self.terminal.height / 3) + 3 && 
                self.buffer.lines.is_empty()  {
                let welcome = "by curlpipe";
                let pad = " ".repeat(self.terminal.width as usize / 2 
                                     - welcome.len() / 2);
                l = format!("{}{}{}", "~", pad, welcome);
            } else if row == (self.terminal.height / 3) + 5 && 
                self.buffer.lines.is_empty()  {
                let welcome = "Ctrl + Q:  Exit";
                let pad = " ".repeat(self.terminal.width as usize / 2 
                                     - welcome.len() / 2);
                l = format!(
                    "{}{}{}{}{}", "~", 
                    pad, 
                    color::Fg(color::Blue),
                    welcome,
                    color::Fg(color::Reset),
                );
            } else if row == term_length - 2 {
                let status_line = format!(
                    " Ox: {} | x: {} | y: {}", 
                    VERSION,
                    self.cursor.x, 
                    self.cursor.y,
                );
                let pad = self.terminal.width as usize - status_line.len();
                let pad = " ".repeat(pad);
                l = format!(
                    "{}{}{}{}{}{}{}{}", 
                    FG, BG, style::Bold,
                    status_line, pad,
                    color::Fg(color::Reset), color::Bg(color::Reset), style::Reset,
                );
            } else if row == term_length - 1 {
                l = format!("DEBUG: {}", self.offset);
            } else if row < self.buffer.lines.len() as u16 {
                let index = self.offset as usize + row as usize;
                l = self.buffer.lines[index].clone();
            } else {
                l = String::from("~");
            }
            print!("{}", l);
        }
        self.terminal.move_cursor(self.cursor.x, self.cursor.y);
        self.terminal.flush();
    }
}

