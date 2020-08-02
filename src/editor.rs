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
enum Direction {
    Up, Down, Left, Right
}

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
    offset: u64,
    command_bar: String,
    show_welcome: bool,
}

impl Editor {
    pub fn new() -> Self {
        // Create a new editor instance
        let args: Vec<String> = env::args().collect();
        let buffer: Buffer;
        let show_welcome: bool;
        if args.len() <= 1 { 
            buffer = Buffer::new();
            show_welcome = true;
        } else {
            show_welcome = false;
            buffer = Buffer::open(args[1].trim());
        }
        Self {
            show_welcome,
            terminal: Terminal::new(),
            kill: false,
            cursor: Cursor { x: 0, y: 0 },
            buffer,
            offset: 0,
            command_bar: String::from("Welcome to Ox!"),
        }

    }
    pub fn run(&mut self) {
        let mut stdin = termion::async_stdin().keys();
        // Run our editor
        loop {
            // Exit if required
            if self.kill {
                self.terminal.clear_all();
                self.terminal.move_cursor(0, 0);
                break; 
            }
            // Render our interface
            self.render();
            // Read a key
            match stdin.next() {
                Some(key) => match key.unwrap() {
                    Key::Ctrl('q') => self.kill = true, // Exit
                    Key::Char(c) => self.insert(c), // Insert character
                    Key::Backspace => self.delete(), // Delete character
                    Key::Left => self.move_cursor(Direction::Left),
                    Key::Right => self.move_cursor(Direction::Right),
                    Key::Up => self.move_cursor(Direction::Up),
                    Key::Down => self.move_cursor(Direction::Down),
                    Key::Ctrl('s') => {
                        // Save the current file
                        self.buffer.save();
                        self.command_bar = format!(
                            "Saved to file: {}", 
                            self.buffer.path
                        );
                    }
                    Key::PageUp => {
                        // Move the cursor to the top of the terminal
                        self.cursor.y = 0;
                        self.correct_line();
                    }
                    Key::PageDown => {
                        // Move the cursor to the bottom of the buffer / terminal
                        let t = self.terminal.height.saturating_sub(3) as u16;
                        let b = self.buffer.lines.len().saturating_sub(2) as u16;
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
                None => {
                    self.terminal.check_resize(); // Check for resize
                    // FPS cap to stop greedy CPU usage
                    thread::sleep(Duration::from_millis(24));
                }
            }
        }
    }
    fn insert(&mut self, character: char) {
        self.show_welcome = false;
        let index = self.cursor.y + self.offset as u16;
        let line = &self.buffer.lines[index as usize];
        if character == '\n' {
            if self.cursor.x == 0 {
                // Cursor is on the start of the line
                self.buffer.lines.insert(self.cursor.y as usize, String::new());
                self.cursor.y = self.cursor.y.saturating_add(1);
            } else if self.cursor.x == line.len() as u16 {
                // Cursor is on the end of the line
                self.cursor.y = self.cursor.y.saturating_add(1);
                self.buffer.lines.insert(self.cursor.y as usize, String::new());
                self.correct_line();
            } else {
                // Cursor is in the middle of the line
                let result: String = line.chars()
                  .take(self.cursor.x as usize)
                  .collect();
                let remainder: String = line.chars()
                  .skip(self.cursor.x as usize)
                  .collect();
                self.buffer.lines[index as usize] = remainder;
                self.buffer.lines.insert(self.cursor.y as usize, result);
                self.cursor.y = self.cursor.y.saturating_add(1);
                self.cursor.x = 0;
            }
        } else {
            let mut result: String = line.chars()
              .take(self.cursor.x as usize)
              .collect();
            let remainder: String = line.chars()
              .skip(self.cursor.x as usize)
              .collect();
            result.push(character);
            result.push_str(&remainder);
            self.buffer.lines[index as usize] = result;
            self.cursor.x = self.cursor.x.saturating_add(1);
        }
    }
    fn delete(&mut self) {
        let index = self.cursor.y + self.offset as u16;
        let line = self.buffer.lines[index as usize].clone();
        let max = (self.buffer.lines.len() - 1) as u16;
        if !(self.cursor.x == 0 && index == 0) && index != max { 
            if self.cursor.x == 0 {
                let old_len = self.buffer.lines[
                    (index - 1) as usize
                ].len() as u16;
                if !line.is_empty() {
                    self.buffer.lines[(index - 1) as usize] += &line.clone();
                }
                self.buffer.lines.remove(index as usize);
                if self.offset > 0 { self.offset -= 1; } 
                else { self.cursor.y = self.cursor.y.saturating_sub(1); }
                self.cursor.x = old_len;
            } else {
                self.cursor.x = self.cursor.x.saturating_sub(1);
                let mut result: String = line.chars()
                    .take(self.cursor.x as usize)
                    .collect();
                let remainder: String = line.chars()
                    .skip((self.cursor.x + 1) as usize)
                    .collect();
                result.push_str(&remainder);
                self.buffer.lines[index as usize] = result;
            }
        } 
    }
    fn correct_line(&mut self) {
        // Ensure that the cursor isn't out of bounds
        if self.buffer.lines.is_empty() { 
            self.cursor.x = 0;
        } else {
            let current = self.buffer.lines[
                (self.cursor.y + self.offset as u16) as usize
            ].clone();
            if self.cursor.x > current.len() as u16 {
                self.cursor.x = current.len() as u16;
            }
        }
    }
    fn move_cursor(&mut self, direction: Direction) {
        match direction {
            Direction::Up => {
                // Move cursor up
                if self.cursor.y != 0 {
                    self.cursor.y = self.cursor.y.saturating_sub(1);
                    self.correct_line();
                } else {
                    self.offset = self.offset.saturating_sub(1);
                }
            }
            Direction::Down => {
                // Move cursor down
                let buff_len = (self.buffer.lines.len() - 1) as u64;
                let proposed = self.cursor.y.saturating_add(1) as u64;
                let max = self.terminal.height.saturating_sub(3);
                if proposed.saturating_add(self.offset) < buff_len {
                    if self.cursor.y < max {
                        self.cursor.y = proposed as u16;
                        self.correct_line();
                    } else {
                        self.offset = self.offset.saturating_add(1);
                    }
                }
            }
            Direction::Left => {
                // Move cursor to the left
                let current = self.cursor.y + self.offset as u16;
                if self.cursor.x == 0 && current != 0 {
                    if self.cursor.y == 0 { 
                        self.offset = self.offset.saturating_sub(1); 
                    }
                    self.cursor.x = self.terminal.width;
                    self.cursor.y = self.cursor.y.saturating_sub(1);
                    self.correct_line();
                } else {
                    self.cursor.x = self.cursor.x.saturating_sub(1);
                }
            }
            Direction::Right => {
                // Move cursor to the right
                let index = self.cursor.y + self.offset as u16;
                if self.buffer.lines.is_empty() { return; }
                let current = &self.buffer.lines[index as usize];
                let size = [
                    &self.terminal.width,
                    &self.terminal.height,
                ];
                if current.len() as u16 == self.cursor.x && 
                   (self.buffer.lines.len() - 1) as u16 != index + 1 {
                    if self.cursor.y == size[1] - 3 { 
                        self.offset = self.offset.saturating_add(1); 
                    } else {
                        self.cursor.y = self.cursor.y.saturating_add(1);
                    }
                    self.cursor.x = 0;
                } else if self.cursor.x < size[0].saturating_sub(1) {
                    self.cursor.x = self.cursor.x.saturating_add(1);
                    self.correct_line();
                }
            }
        }
    }
    fn render(&mut self) {
        // Render the rows
        let term_length = self.terminal.height;
        let mut frame: Vec<String> = Vec::new();
        self.terminal.clear_all();
        for row in 0..self.terminal.height {
            if row == self.terminal.height / 3 && self.show_welcome {
                let welcome = format!("Ox editor v{}", VERSION);
                let pad = " ".repeat(self.terminal.width as usize / 2 
                                     - welcome.len() / 2);
                frame.push(format!("{}{}{}", "~", pad, welcome));
            } else if row == (self.terminal.height / 3) + 2 && 
                self.show_welcome  {
                let welcome = "A speedy editor built with Rust";
                let pad = " ".repeat(self.terminal.width as usize / 2 
                                     - welcome.len() / 2);
                frame.push(format!("{}{}{}", "~", pad, welcome));
            } else if row == (self.terminal.height / 3) + 3 && 
                self.show_welcome  {
                let welcome = "by curlpipe";
                let pad = " ".repeat(self.terminal.width as usize / 2 
                                     - welcome.len() / 2);
                frame.push(format!("{}{}{}", "~", pad, welcome));
            } else if row == (self.terminal.height / 3) + 5 && 
                self.show_welcome {
                let welcome = "Ctrl + Q:  Exit";
                let pad = " ".repeat(self.terminal.width as usize / 2 
                                     - welcome.len() / 2);
                frame.push(format!(
                    "{}{}{}{}{}", "~", 
                    pad, 
                    color::Fg(color::Blue),
                    welcome,
                    color::Fg(color::Reset),
                ));
            } else if row == term_length - 2 {
                let status_line = format!(
                    " Ox: {} | x: {} | y: {}", 
                    VERSION,
                    self.cursor.x, 
                    self.cursor.y,
                );
                let pad = self.terminal.width as usize - status_line.len();
                let pad = " ".repeat(pad);
                frame.push(format!(
                    "{}{}{}{}{}{}{}{}", 
                    FG, BG, style::Bold,
                    status_line, pad,
                    color::Fg(color::Reset), color::Bg(color::Reset), style::Reset,
                ));
            } else if row == term_length - 1 {
                frame.push(self.command_bar.clone());
            } else if row < (self.buffer.lines.len() - 1) as u16 {
                let index = self.offset as usize + row as usize;
                frame.push(self.buffer.lines[index].clone());
            } else {
                frame.push(String::from("~"));
            }
        }
        self.terminal.move_cursor(0, 0);
        self.terminal.write(format!("{}", frame.join("\r\n")));
        self.terminal.move_cursor(self.cursor.x, self.cursor.y);
        self.terminal.flush();
    }
}

