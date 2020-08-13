// Editor.rs - For controling the current editor
use crate::{Buffer, Row, Terminal};
use std::cmp::min;
use std::io::Error;
use std::time::Duration;
use std::{env, thread};
use termion::event::Key;
use termion::input::TermRead;
use termion::{color, style};
use unicode_width::UnicodeWidthStr;

// Get Ox version
const VERSION: &str = env!("CARGO_PKG_VERSION");

// Get status bar colors
const STATUS_BG: color::Bg<color::Rgb> = color::Bg(color::Rgb(68, 71, 90));
const STATUS_FG: color::Fg<color::Rgb> = color::Fg(color::Rgb(80, 250, 123));

// Global colors
const BG: color::Bg<color::Rgb> = color::Bg(color::Rgb(40, 42, 54));

// For holding the position and directions of the cursor
enum Direction {
    Up,
    Down,
    Left,
    Right,
}

pub struct Cursor {
    x: u16,
    y: u16,
}

// For holding our editor information
pub struct Editor {
    dirty: bool,
    stdin: termion::AsyncReader,
    terminal: Terminal,
    kill: bool,
    raw_cursor: u16,
    cursor: Cursor,
    buffer: Buffer,
    offset: u64,
    command_bar: String,
    show_welcome: bool,
}

impl Editor {
    pub fn new() -> Result<Self, Error> {
        // Create a new editor instance
        let args: Vec<String> = env::args().collect();
        let stdin = termion::async_stdin();
        let buffer: Buffer;
        let show_welcome: bool;
        if args.len() <= 1 {
            show_welcome = true;
            buffer = Buffer::new();
            Ok(Self {
                dirty: false,
                stdin,
                show_welcome,
                terminal: Terminal::new(),
                kill: false,
                cursor: Cursor { x: 0, y: 0 },
                raw_cursor: 0,
                buffer,
                offset: 0,
                command_bar: String::from("Welcome to Ox!"),
            })
        } else if let Some(buffer) = Buffer::open(args[1].trim()) {
            show_welcome = false;
            Ok(Self {
                dirty: false,
                stdin,
                show_welcome,
                terminal: Terminal::new(),
                kill: false,
                cursor: Cursor { x: 0, y: 0 },
                raw_cursor: 0,
                buffer,
                offset: 0,
                command_bar: String::from("Welcome to Ox!"),
            })
        } else {
            show_welcome = true;
            buffer = Buffer::from(args[1].trim());
            Ok(Self {
                dirty: true,
                stdin,
                show_welcome,
                terminal: Terminal::new(),
                kill: false,
                cursor: Cursor { x: 0, y: 0 },
                raw_cursor: 0,
                buffer,
                offset: 0,
                command_bar: String::from("Welcome to Ox!"),
            })
        }
    }
    pub fn run(&mut self) {
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
            let key = self.loop_until_keypress();
            match key {
                Key::Char(c) => self.insert(c),  // Insert character
                Key::Backspace => self.delete(), // Delete character
                Key::Left => self.move_cursor(Direction::Left),
                Key::Right => self.move_cursor(Direction::Right),
                Key::Up => self.move_cursor(Direction::Up),
                Key::Down => self.move_cursor(Direction::Down),
                Key::Ctrl('q') => self.close(), // Exit
                Key::Ctrl('n') => self.new_buffer(),
                Key::Ctrl('o') => self.open_buffer(),
                Key::Ctrl('w') => {
                    // Save as
                    let filename = self.prompt("Save as");
                    if let Some(filename) = filename {
                        if self.buffer.save_as(&filename).is_ok() {
                            self.command_bar = format!("Saved to file: {}", filename);
                            self.buffer.path = filename.clone();
                            self.buffer.filename = filename.clone();
                            self.dirty = false;
                        } else {
                            self.command_bar = format!("Failed to save file: {}", filename);
                        }
                    } else {
                        self.command_bar = String::from("Save as cancelled");
                    }
                }
                Key::Ctrl('s') => {
                    // Save the current file
                    if self.buffer.save().is_ok() {
                        self.command_bar = format!("Saved to file: {}", self.buffer.path);
                        self.dirty = false;
                    } else {
                        self.command_bar = format!("Failed to save file: {}", self.buffer.path);
                    }
                }
                Key::PageUp => {
                    // Move the cursor to the top of the terminal
                    self.cursor.y = 0;
                    self.correct_line();
                }
                Key::PageDown => {
                    // Move the cursor to the bottom of the buffer / terminal
                    let t = self.terminal.height.saturating_sub(3) as usize;
                    let b = self.buffer.lines.len().saturating_sub(1) as usize;
                    self.cursor.y = min(t, b) as u16;
                    self.correct_line();
                }
                Key::Home => {
                    // Move to the start of the current line
                    self.cursor.x = 0;
                    self.raw_cursor = 0;
                }
                Key::End => {
                    // Move to the end of the current line
                    self.cursor.x = self.terminal.width.saturating_sub(1);
                    self.raw_cursor = self.terminal.width.saturating_sub(1);
                    self.correct_line();
                }
                _ => (), // Unbound key
            }
        }
    }
    fn loop_until_keypress(&mut self) -> Key {
        loop {
            let keys = &mut self.stdin;
            if let Some(key) = keys.keys().next() {
                return key.unwrap();
            } else {
                if self.terminal.check_resize() {
                    self.render();
                }
                thread::sleep(Duration::from_millis(12));
            }
        }
    }
    fn insert(&mut self, character: char) {
        self.dirty = true;
        self.show_welcome = false;
        let index = self.cursor.y + self.offset as u16;
        let current = &self.buffer.lines[index as usize];
        if character == '\n' {
            // Handle return key
            if self.cursor.x == 0 {
                // Cursor is at the beginning of the line
                self.buffer
                    .lines
                    .insert(index as usize, Row::new(String::new()));
                self.buffer.lines[index as usize].update_jumps();
                self.move_cursor(Direction::Down);
            } else if self.cursor.x == current.length() as u16 {
                // Cursor is at the end of the line
                self.buffer
                    .lines
                    .insert((index + 1) as usize, Row::new(String::new()));
                self.buffer.lines[(index + 1) as usize].update_jumps();
                self.move_cursor(Direction::Down);
                self.correct_line();
            } else {
                // Cursor is in the middle of the line
                let before: String = current.chars().take(self.cursor.x as usize).collect();
                let after: String = current.chars().skip(self.cursor.x as usize).collect();
                self.buffer.lines[index as usize] = Row::new(before);
                self.buffer.lines[index as usize].update_jumps();
                self.buffer
                    .lines
                    .insert((index + 1) as usize, Row::new(after));
                self.buffer.lines[(index + 1) as usize].update_jumps();
                self.move_cursor(Direction::Down);
                self.cursor.x = 0;
                self.raw_cursor = 0;
            }
        } else {
            // Not the return key
            let mut before: String = current.chars().take(self.cursor.x as usize).collect();
            let after: String = current.chars().skip(self.cursor.x as usize).collect();
            before.push(character);
            before.push_str(&after);
            self.buffer.lines[index as usize] = Row::new(before);
            self.move_cursor(Direction::Right);
        }
    }
    fn delete(&mut self) {
        self.dirty = true;
        let index = self.cursor.y + self.offset as u16;
        if self.buffer.lines.is_empty() {
            return;
        }
        if self.cursor.x == 0 && index != 0 {
            // Cursor is at the beginning of a line
            let current = &self.buffer.lines[index as usize];
            let up = &self.buffer.lines[(index - 1) as usize];
            let old_line = current.string.clone();
            let old_raw_len = up.raw_length() as u16;
            let old_len = up.length() as u16;
            self.buffer.lines.remove(index as usize);
            if self.offset > 0 {
                self.offset -= 1;
            } else {
                self.cursor.y = self.cursor.y.saturating_sub(1);
            }
            self.correct_line();
            let new_index = self.cursor.y + self.offset as u16;
            self.buffer.lines[new_index as usize] =
                Row::new(self.buffer.lines[new_index as usize].string.clone() + &old_line);
            self.buffer.lines[new_index as usize].update_jumps();
            self.cursor.x = old_len;
            self.raw_cursor = old_raw_len;
        } else {
            // Cursor is ready to delete text
            let current = &self.buffer.lines[index as usize].clone();
            self.move_cursor(Direction::Left);
            let before: String = current.chars().take(self.cursor.x as usize).collect();
            let after: String = current.chars().skip(1 + self.cursor.x as usize).collect();
            self.buffer.lines[index as usize] = Row::new(before + &after);
            self.buffer.lines[index as usize].update_jumps();
        }
    }
    fn move_cursor(&mut self, direction: Direction) {
        match direction {
            Direction::Up => {
                // Move cursor up
                if self.cursor.y == 0 {
                    self.offset = self.offset.saturating_sub(1);
                } else {
                    self.cursor.y = self.cursor.y.saturating_sub(1);
                }
                self.correct_line();
            }
            Direction::Down => {
                // Move cursor down
                let buff_len = self.buffer.lines.len() as u64;
                let proposed = u64::from(self.cursor.y.saturating_add(1));
                let max = self.terminal.height.saturating_sub(3);
                if proposed.saturating_add(self.offset) < buff_len {
                    if self.cursor.y < max {
                        self.cursor.y = proposed as u16;
                    } else {
                        self.offset = self.offset.saturating_add(1);
                    }
                    self.correct_line();
                }
            }
            Direction::Left => {
                // Move cursor to the left
                let index = self.cursor.y + self.offset as u16;
                let current = &self.buffer.lines[index as usize];
                if self.raw_cursor == 0 && index != 0 {
                    if self.cursor.y == 0 {
                        self.offset = self.offset.saturating_sub(1);
                    }
                    self.cursor.x = self.terminal.width;
                    self.raw_cursor = self.terminal.width;
                    self.cursor.y = self.cursor.y.saturating_sub(1);
                    self.correct_line();
                } else if self.cursor.x != 0 {
                    self.raw_cursor = self
                        .raw_cursor
                        .saturating_sub(current.jumps[(self.cursor.x - 1) as usize] as u16);
                    self.cursor.x = self.cursor.x.saturating_sub(1);
                }
            }
            Direction::Right => {
                // Move cursor to the right
                let index = self.cursor.y + self.offset as u16;
                if self.buffer.lines.is_empty() {
                    return;
                }
                let current = &self.buffer.lines[index as usize];
                let size = [&self.terminal.width, &self.terminal.height];
                if current.raw_length() as u16 == self.raw_cursor
                    && self.buffer.lines.len() as u16 != index + 1
                {
                    if self.cursor.y == size[1] - 3 {
                        self.offset = self.offset.saturating_add(1);
                    } else {
                        self.cursor.y = self.cursor.y.saturating_add(1);
                    }
                    self.cursor.x = 0;
                    self.raw_cursor = 0;
                } else if self.raw_cursor < current.raw_length() as u16 {
                    self.raw_cursor = self
                        .raw_cursor
                        .saturating_add(current.jumps[self.cursor.x as usize] as u16);
                    self.cursor.x = self.cursor.x.saturating_add(1);
                    self.correct_line();
                }
            }
        }
        self.update_cursor();
    }
    fn correct_line(&mut self) {
        // Ensure that the cursor isn't out of bounds
        if self.buffer.lines.is_empty() {
            self.cursor.x = 0;
            self.raw_cursor = 0;
        } else {
            let current = &self.buffer.lines[(self.cursor.y + self.offset as u16) as usize];
            if self.raw_cursor >= current.raw_length() as u16 {
                self.raw_cursor = current.raw_length() as u16;
                self.cursor.x = current.length() as u16;
            }
        }
    }
    fn update_cursor(&mut self) {
        let index = self.cursor.y + self.offset as u16;
        let current = &self.buffer.lines[index as usize];
        let mut raw_count = i64::from(self.raw_cursor);
        let mut count = 0;
        for jump in &current.jumps {
            if raw_count <= 0 {
                break;
            }
            count += 1;
            raw_count -= *jump as i64;
        }
        if raw_count < 0 {
            self.raw_cursor = self.raw_cursor.saturating_sub(1);
        }
        self.cursor.x = count;
    }
    fn prompt(&mut self, prompt: &str) -> Option<String> {
        // Create a new prompt
        self.command_bar = format!("{}: ", prompt);
        self.render();
        let mut result = String::new();
        'p: loop {
            let key = self.loop_until_keypress();
            match key {
                Key::Char(c) => {
                    if c == '\n' {
                        break 'p;
                    } else {
                        result.push(c);
                    }
                }
                Key::Backspace => {
                    result.pop();
                }
                Key::Esc => return None,
                _ => (),
            }
            self.command_bar = format!("{}: {}", prompt, result);
            self.render();
        }
        Some(result)
    }
    fn welcome_message(&self, welcome: &str, fg: color::Fg<color::Rgb>) -> String {
        let pad = " ".repeat(self.terminal.width as usize / 2 - welcome.len() / 2);
        let pad_right =
            " ".repeat(self.terminal.width.saturating_sub(1) as usize - welcome.len() - pad.len());
        format!(
            "{}{}{}{}{}{}{}{}",
            BG,
            "~",
            pad,
            fg,
            welcome,
            color::Fg(color::Reset),
            pad_right,
            color::Bg(color::Reset),
        )
    }
    fn new_buffer(&mut self) {
        // Creating buffer
        if self.dirty {
            if self
                .prompt("Edited file! Enter to confirm, Esc to cancel")
                .is_some()
            {
                self.command_bar = "New buffer created".to_string();
                self.buffer = Buffer::new();
                self.render();
                self.cursor.y = 0;
                self.correct_line();
            } else {
                self.command_bar = "New buffer cancelled".to_string();
            }
        } else {
            self.buffer = Buffer::new();
            self.render();
            self.cursor.y = 0;
            self.correct_line();
        }
    }
    fn open_buffer(&mut self) {
        // Open file into buffer
        if self.dirty {
            if self
                .prompt("Edited file! Enter to confirm, Esc to cancel")
                .is_none()
            {
                self.command_bar = String::new();
                return;
            }
        }
        if let Some(filename) = self.prompt("File to open") {
            if let Some(buffer) = Buffer::open(&filename[..]) {
                self.buffer = buffer;
            } else {
                self.command_bar = "Failed to open file".to_string();
            }
        }
    }
    fn close(&mut self) {
        // Close the editor
        if self.dirty {
            if self
                .prompt("Edited file! Enter to confirm, Esc to cancel")
                .is_some()
            {
                self.kill = true;
            }
            self.command_bar = String::new();
        } else {
            self.kill = true;
        }
    }
    fn render(&mut self) {
        // Render the rows
        self.buffer.update_line_offset();
        let max_line = self.buffer.lines.len().to_string().len();
        let term_length = self.terminal.height;
        let mut frame: Vec<String> = Vec::new();
        for row in 0..self.terminal.height {
            if row == (self.terminal.height / 3) - 3 && self.show_welcome {
                frame.push(self.welcome_message(
                    &format!("Ox editor v{}", VERSION)[..],
                    color::Fg(color::Rgb(255, 255, 255)),
                ));
            } else if row == (self.terminal.height / 3) - 1 && self.show_welcome {
                frame.push(self.welcome_message(
                    "A speedy editor built with Rust",
                    color::Fg(color::Rgb(255, 255, 255)),
                ));
            } else if row == (self.terminal.height / 3) && self.show_welcome {
                frame.push(
                    self.welcome_message("by curlpipe", color::Fg(color::Rgb(255, 255, 255))),
                );
            } else if row == (self.terminal.height / 3) + 2 && self.show_welcome {
                frame.push(self.welcome_message("Ctrl + N: New    ", STATUS_FG));
            } else if row == (self.terminal.height / 3) + 3 && self.show_welcome {
                frame.push(self.welcome_message("Ctrl + O: Open   ", STATUS_FG));
            } else if row == (self.terminal.height / 3) + 4 && self.show_welcome {
                frame.push(self.welcome_message("Ctrl + S: Save   ", STATUS_FG));
            } else if row == (self.terminal.height / 3) + 5 && self.show_welcome {
                frame.push(self.welcome_message("Ctrl + W: Save As", STATUS_FG));
            } else if row == (self.terminal.height / 3) + 6 && self.show_welcome {
                frame.push(self.welcome_message("Ctrl + Q: Quit   ", STATUS_FG));
            } else if row == term_length - 2 {
                let index = self.cursor.y + self.offset as u16;
                let left = format!(
                    " {}{} \u{2502} {} \u{f1c9} ",
                    self.buffer.filename,
                    if self.dirty {
                        "[+] \u{fb12} "
                    } else {
                        " \u{f723} "
                    },
                    self.buffer.identify()
                );
                let right = format!(
                    "\u{fa70} {} / {} \u{2502} \u{fae6}({}, {}) ",
                    index + 1,
                    self.buffer.lines.len(),
                    self.cursor.x,
                    self.cursor.y
                );
                let pad = self.terminal.width as usize
                    - UnicodeWidthStr::width(&left[..])
                    - UnicodeWidthStr::width(&right[..]);
                let pad = " ".repeat(pad);
                frame.push(format!(
                    "{}{}{}{}{}{}{}{}{}",
                    STATUS_FG,
                    STATUS_BG,
                    style::Bold,
                    left,
                    pad,
                    right,
                    color::Fg(color::Reset),
                    color::Bg(color::Reset),
                    style::Reset,
                ));
            } else if row == term_length - 1 {
                let line = self.command_bar.clone();
                let pad = " ".repeat((self.terminal.width - line.len() as u16) as usize);
                frame.push(format!("{}{}{}{}", BG, line, pad, color::Bg(color::Reset)));
            } else if row < self.buffer.lines.len() as u16 {
                let index = self.offset as usize + row as usize;
                let mut line = self.buffer.lines[index].clone();
                let length = line.raw_length() + self.buffer.line_number_offset;
                if (self.terminal.width as usize) < length {
                    line = Row::new(
                        line.string[..self
                            .terminal
                            .width
                            .saturating_sub(self.buffer.line_number_offset as u16)
                            as usize]
                            .to_string(),
                    );
                }
                let post_padding =
                    max_line.saturating_sub(index.saturating_add(1).to_string().len());
                let line_number = format!(
                    "{}{}{}",
                    " ".repeat(post_padding),
                    index.saturating_add(1),
                    " ",
                );
                let pad = " ".repeat(
                    (self.terminal.width as usize)
                        .saturating_sub(line.raw_length() + line_number.len())
                        as usize,
                );
                frame.push(format!(
                    "{}{}{}{}{}",
                    BG,
                    line_number,
                    line.string,
                    pad,
                    color::Bg(color::Reset)
                ));
            } else {
                frame.push(format!(
                    "{}~{}{}",
                    BG,
                    " ".repeat(self.terminal.width.saturating_sub(1) as usize),
                    color::Bg(color::Reset),
                ));
            }
        }
        self.terminal.move_cursor(0, 0);
        self.terminal
            .write(&format!("{}{}{}", BG, frame.join("\r\n"), color::Bg(color::Reset),)[..]);
        self.terminal.move_cursor(
            self.raw_cursor + self.buffer.line_number_offset as u16,
            self.cursor.y,
        );
        self.terminal.flush();
    }
}
