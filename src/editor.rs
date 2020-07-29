// Editor.rs - For controling the current editor
use termion::event::Key;
use crate::Terminal;
use crate::Buffer;
use std::env;

// Get the version of Ox
const VERSION: &str = env!("CARGO_PKG_VERSION");

// For holding the position and directions of the cursor
enum Direction {
    Left,
    Right,
    Up,
    Down,
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
                println!("Ox exited\r");
                break; 
            }
            // Check for and subsequently handle a resize event
            self.terminal.check_resize();
            // Clear our screen
            self.terminal.clear_all();
            // Render our interface
            self.render();
            // Get the current line that the cursor is on
            let current = self.get_line_length(0);
            // Read a key
            if let Ok(key) = self.terminal.read_key() {
                match key {
                    Key::Ctrl('q') => self.kill = true, // Exit
                    // Move the cursor
                    Key::Left => self.move_cursor(Direction::Left),
                    Key::Right => self.move_cursor(Direction::Right),
                    Key::Up => self.move_cursor(Direction::Up),
                    Key::Down => self.move_cursor(Direction::Down),
                    Key::PageUp => {
                        self.cursor.y = 0;
                        let current = self.get_line_length(0);
                        if self.cursor.x > current {
                            self.cursor.x = current;
                        }
                    }
                    Key::PageDown => {
                        self.cursor.y = self.get_line_count();
                        let current = self.get_line_length(0);
                        if self.cursor.x > current {
                            self.cursor.x = current;
                        }
                    }
                    Key::Home => self.cursor.x = 0,
                    Key::End => self.cursor.x = current,
                    _ => (), // Unbound key
                }
            } else {
                kill(); // There was an error reading the key
            }
        }
    }
    fn get_line(&self, offset: i32) -> usize {
        // Get the current line number
        (self.cursor.y as i32 + offset) as usize
    }
    fn get_line_length(&self, offset: i32) -> u16 {
        // Get the length of the current line that the cursor is on
        let lines = &self.buffer.lines;
        if lines.is_empty() {
            0
        } else {
            lines[self.get_line(offset)].len() as u16
        }
    }
    fn get_line_count(&self) -> u16 {
        self.buffer.lines.len().saturating_sub(1) as u16
    }
    fn render(&mut self) {
        // Render the rows
        for row in 0..self.terminal.height - 1 {
            self.terminal.move_cursor(0, row);
            self.terminal.clear_line();
            let l: String;
            if row == self.terminal.height / 5 && self.buffer.lines.is_empty() {
                let welcome = format!("Ox editor v{}", VERSION);
                let pad = " ".repeat(self.terminal.width as usize / 2 
                                     - welcome.len() / 2);
                l = format!("{}{}{}", "~", pad, welcome);
            } else if row < self.buffer.lines.len() as u16 {
                l = self.buffer.lines[row as usize].clone();
            } else {
                l = String::from("~");
            }
            println!("{}\r", l);
        }
        self.terminal.move_cursor(self.cursor.x, self.cursor.y);
        self.terminal.flush();
    }
    fn move_cursor(&mut self, direction: Direction) {
        // Move the cursor in a certain direction
        let current = self.get_line(0);
        match direction {
            Direction::Left => self.cursor.x = self.cursor.x.saturating_sub(1),
            Direction::Up => {
                if current == 0 {
                    return;
                }
                let up = self.get_line_length(-1);
                self.cursor.y = self.cursor.y.saturating_sub(1);
                if self.cursor.x > up {
                    self.cursor.x = up;
                }
            }
            Direction::Right => {
                let line = self.get_line_length(0);
                if self.cursor.x <= (line).saturating_sub(1) && line != 0 {
                    self.cursor.x = self.cursor.x.saturating_add(1);
                }
            }
            Direction::Down => {
                if self.cursor.y < self.get_line_count() {
                    let down = self.get_line_length(1);
                    self.cursor.y = self.cursor.y.saturating_add(1);
                    if self.cursor.x > down {
                        self.cursor.x = down;
                    }
                }
            }
        };
    }
}

fn kill() {
    // Kill the program
    panic!("Exited");
}
