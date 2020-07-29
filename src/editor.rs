// Editor.rs - For controling the current editor
use termion::{color, style};
use termion::event::Key;
use crate::Terminal;
use crate::Buffer;
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
        }

    }
    pub fn run(&mut self) {
        // Run our editor
        loop {
            // Exit if required
            if self.kill { 
                self.terminal.clear_all();
                self.terminal.move_cursor(0, 0);
                // println!("Ox exited\r");
                break; 
            }
            // Check for and subsequently handle a resize event
            self.terminal.check_resize();
            // Render our interface
            self.render();
            // Read a key
            if let Ok(key) = self.terminal.read_key() {
                match key {
                    Key::Ctrl('q') => self.kill = true, // Exit
                    Key::Left => {
                        // Move cursor to the left
                        self.cursor.x = self.cursor.x.saturating_sub(1);
                    }
                    Key::Right => {
                        // Move cursor to the right
                        self.cursor.x = self.cursor.x.saturating_add(1);
                        self.correct_line();
                    }
                    Key::Up => {
                        // Move cursor up
                        self.cursor.y = self.cursor.y.saturating_sub(1);
                        self.correct_line();
                    }
                    Key::Down => {
                        // Move cursor down
                        if self.cursor.y < self.terminal.height.saturating_sub(3) {
                            self.cursor.y = self.cursor.y.saturating_add(1);
                            self.correct_line();
                        } else {
                            self.buffer.offset += 1;
                        }
                    }
                    Key::PageUp => {
                        // Move the cursor to the top of the terminal
                        self.cursor.y = 0;
                        self.correct_line();
                    }
                    Key::PageDown => {
                        // Move the cursor to the bottom of the terminal
                        self.cursor.y = self.terminal.height.saturating_sub(3);
                        self.correct_line();
                    }
                    Key::Home => {
                        // Move to the start of the current line
                        self.cursor.x = 0;
                    }
                    Key::End => {
                        // Move to the end of the current line
                        self.cursor.x = self.terminal.width;
                        self.correct_line();
                    }
                    _ => (), // Unbound key
                }
            } else {
                kill(); // There was an error reading the key
            }
        }
    }
    fn correct_line(&mut self) {
        // Ensure that the cursor isn't out of bounds
        let current = self.buffer.lines[self.cursor.y as usize].clone();
        if self.cursor.x > current.len() as u16 {
            self.cursor.x = current.len() as u16;
        }
    }
    fn render(&mut self) {
        // Render the rows
        let buf_length = self.buffer.lines.len() as u16;
        let term_length = self.terminal.height;
        for row in 0..self.terminal.height {
            self.terminal.move_cursor(0, row);
            self.terminal.clear_line();
            let l: String;
            if row == self.terminal.height / 5 && self.buffer.lines.is_empty() {
                let welcome = format!("Ox editor v{}", VERSION);
                let pad = " ".repeat(self.terminal.width as usize / 2 
                                     - welcome.len() / 2);
                l = format!("{}{}{}", "~", pad, welcome);
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
                l = format!("CTRL + Q to quit");
            } else if buf_length > term_length {
                l = self.buffer.lines[row as usize].clone();
            } else {
                l = String::from("~");
            }
            print!("{}", l);
        }
        self.terminal.move_cursor(self.cursor.x, self.cursor.y);
        self.terminal.flush();
    }
}

fn kill() {
    // Kill the program
    panic!("Exited");
}
