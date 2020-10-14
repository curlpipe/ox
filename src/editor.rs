// Editor.rs - Controls the editor and brings everything together
use crate::config::{Reader, Status};
use crate::util::{is_ahead, is_behind, raw_to_grapheme, title, trim_end}; // Bring in the utils
use crate::{Document, Event, Row, Terminal, VERSION}; // Bringing in all the structs
use clap::App; // For a nice command line interface
use regex::Regex; // Regex for replacement
use std::collections::HashMap;
use std::time::{Duration, Instant}; // For implementing an FPS cap and measuring time
use std::{cmp, io::Error, thread}; // Managing threads, arguments and comparisons.
use termion::event::Key; // For reading Keys and shortcuts
use termion::input::{Keys, TermRead}; // To allow reading from the terminal
use termion::{async_stdin, color, style, AsyncReader}; // For managing the terminal
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr}; // For calculating unicode character widths

// Set up color resets
pub const RESET_BG: color::Bg<color::Reset> = color::Bg(color::Reset);
pub const RESET_FG: color::Fg<color::Reset> = color::Fg(color::Reset);

// Enum for the kinds of status messages
enum Type {
    Error,
    Warning,
    Info,
}

// Enum for holding prompt events
enum PromptEvent {
    Update,
    CharPress,
    KeyPress(Key),
}

// For holding the info in the command line
struct CommandLine {
    msg: Type,
    text: String,
}

// For representing positions
#[derive(Clone, Copy, Debug)]
pub struct Position {
    pub x: usize,
    pub y: usize,
}

// The main editor struct
pub struct Editor {
    pub config: Reader,             // Storage for configuration
    quit: bool,                     // Toggle for cleanly quitting the editor
    show_welcome: bool,             // Toggle for showing the welcome message
    dirty: bool,                    // True if the current document has been edited
    graphemes: usize,               // For holding the special grapheme cursor
    command_line: CommandLine,      // For holding the command line
    term: Terminal,                 // For the handling of the terminal
    cursor: Position,               // For holding the raw cursor location
    doc: Document,                  // For holding our document
    offset: Position,               // For holding the offset on the X and Y axes
    last_keypress: Option<Instant>, // For holding the time of the last input event
    stdin: Keys<AsyncReader>,       // Asynchronous stdin
    pub regex: HashMap<String, Vec<Regex>>,
}

// Implementing methods for our editor struct / class
impl Editor {
    pub fn new(args: App) -> Result<Self, Error> {
        // Create a new editor instance
        let args = args.get_matches();
        let files: Vec<&str> = args.values_of("files").unwrap_or_default().collect();
        let config = Reader::read(args.value_of("config").unwrap_or_default());
        Ok(Self {
            quit: false,
            show_welcome: files.is_empty(),
            dirty: false,
            command_line: CommandLine {
                text: match &config.1 {
                    Status::Success => "Welcome to Ox".to_string(),
                    Status::File => "Config file not found, using default values".to_string(),
                    Status::Parse(error) => format!("Failed to parse: {:?}", error),
                },
                msg: match &config.1 {
                    Status::Success => Type::Info,
                    Status::File => Type::Warning,
                    Status::Parse(_) => Type::Error,
                },
            },
            term: Terminal::new()?,
            graphemes: 0,
            cursor: Position { x: 0, y: 0 },
            offset: Position { x: 0, y: 0 },
            doc: if files.is_empty() {
                Document::new(&config.0)
            } else {
                Document::from(&config.0, files[0])
            },
            last_keypress: None,
            stdin: async_stdin().keys(),
            config: config.0.clone(),
            regex: Reader::get_syntax_regex(&config.0),
        })
    }
    pub fn run(&mut self) {
        // Run the editor instance
        // TODO: Render entire document row here
        while !self.quit {
            self.update();
            self.process_input();
        }
    }
    fn read_key(&mut self) -> Key {
        // Wait until a key is pressed and then return it
        loop {
            if let Some(key) = self.stdin.next() {
                // When a keypress was detected
                self.last_keypress = Some(Instant::now());
                if let Ok(key) = key {
                    return key;
                } else {
                    continue;
                }
            } else {
                // Run code that we want to run when the key isn't pressed
                if self.term.check_resize() {
                    // The terminal has changed in size
                    if self.cursor.y > self.term.height.saturating_sub(3) as usize {
                        // Prevent cursor going off the screen and breaking everything
                        self.cursor.y = self.term.height.saturating_sub(3) as usize;
                    }
                    // Re-render everything to the new size
                    self.update();
                }
                // Check for a period of inactivity
                if let Some(time) = self.last_keypress {
                    if time.elapsed().as_secs() >= self.config.general.undo_period {
                        self.doc.undo_stack.commit();
                        self.last_keypress = None;
                    }
                }
                // FPS cap to stop using the entire CPU
                thread::sleep(Duration::from_millis(16));
            }
        }
    }
    fn process_input(&mut self) {
        // Read a key and act on it
        let key = self.read_key();
        match key {
            Key::Char(c) => self.character(c),
            Key::Backspace => self.backspace(),
            Key::Ctrl('q') => self.quit(),
            Key::Ctrl('s') => self.save(),
            Key::Ctrl('w') => self.save_as(),
            Key::Ctrl('n') => self.new_document(),
            Key::Ctrl('o') => self.open_document(),
            Key::Ctrl('f') => self.search(),
            Key::Ctrl('u') => self.undo(),
            Key::Ctrl('y') => self.redo(),
            Key::Ctrl('r') => self.replace(),
            Key::Ctrl('a') => self.replace_all(),
            Key::Left | Key::Right | Key::Up | Key::Down => self.move_cursor(key),
            Key::PageDown | Key::PageUp | Key::Home | Key::End => self.leap_cursor(key),
            _ => (),
        }
    }
    fn redo(&mut self) {
        if let Some(events) = self.doc.redo_stack.pop() {
            for event in events.iter().rev() {
                match event {
                    // TODO: Update relavent lines here
                    Event::InsertTab(pos) => {
                        self.cursor.y = pos.y - self.offset.y;
                        self.cursor.x =
                            pos.x.saturating_sub(self.config.general.tab_width) - self.offset.x;
                        self.recalculate_graphemes();
                        self.tab();
                    }
                    Event::InsertMid(pos, c) => {
                        let c_len = UnicodeWidthChar::width(*c).map_or(0, |c| c);
                        self.cursor.y = pos.y - self.offset.y;
                        self.cursor.x = pos.x.saturating_add(c_len) - self.offset.x;
                        self.recalculate_graphemes();
                        self.doc.rows[pos.y].insert(*c, pos.x);
                    }
                    Event::BackspaceMid(pos, _) => {
                        self.cursor.y = pos.y - self.offset.y;
                        self.cursor.x = pos.x - self.offset.x;
                        self.recalculate_graphemes();
                        self.doc.rows[pos.y].delete(pos.x);
                    }
                    Event::ReturnEnd(pos) => {
                        self.cursor.y = pos.y - self.offset.y;
                        self.cursor.x = pos.x - self.offset.x;
                        self.recalculate_graphemes();
                        self.doc.rows.insert(pos.y + 1, Row::from(""));
                        self.move_cursor(Key::Down);
                    }
                    Event::ReturnStart(pos) => {
                        self.cursor.y = pos.y - self.offset.y;
                        self.cursor.x = pos.x - self.offset.x;
                        self.recalculate_graphemes();
                        self.doc.rows.insert(pos.y, Row::from(""));
                        self.move_cursor(Key::Down);
                    }
                    Event::ReturnMid(pos, breakpoint) => {
                        self.cursor.y = pos.y - self.offset.y;
                        self.cursor.x = pos.x - self.offset.x;
                        self.recalculate_graphemes();
                        let current = self.doc.rows[pos.y].string.clone();
                        let before = Row::from(&current[..*breakpoint]);
                        let after = Row::from(&current[*breakpoint..]);
                        self.doc.rows.insert(pos.y + 1, after);
                        self.doc.rows[pos.y] = before;
                        self.move_cursor(Key::Down);
                        self.leap_cursor(Key::Home);
                    }
                    Event::BackspaceStart(pos) => {
                        self.cursor.y = pos.y - self.offset.y;
                        self.recalculate_graphemes();
                        let current = self.doc.rows[pos.y + 1].string.clone();
                        let prev = self.doc.rows[pos.y].clone();
                        self.doc.rows[pos.y + 1] = Row::from(&(prev.string.clone() + &current)[..]);
                        self.doc.rows.remove(pos.y);
                        self.move_cursor(Key::Up);
                        self.cursor.x = prev.length();
                        self.recalculate_graphemes();
                    }
                    Event::UpdateLine(pos, _, after) => {
                        self.doc.rows[*pos] = after.clone();
                        self.snap_cursor();
                        self.prevent_unicode_hell();
                        self.recalculate_graphemes();
                    }
                }
                self.dirty = true;
                self.show_welcome = false;
            }
            self.doc.undo_stack.append(events);
        } else {
            self.set_command_line("Empty Redo Stack".to_string(), Type::Error);
        }
    }
    fn undo(&mut self) {
        self.doc.undo_stack.commit();
        if let Some(events) = self.doc.undo_stack.pop() {
            for event in &events {
                match event {
                    // TODO: Update relavent lines here
                    Event::InsertTab(pos) => {
                        for i in 1..=self.config.general.tab_width {
                            self.doc.rows[pos.y].delete(pos.x - i);
                            self.move_cursor(Key::Left);
                        }
                    }
                    Event::InsertMid(pos, c) => {
                        let c_len = UnicodeWidthChar::width(*c).map_or(0, |c| c);
                        self.cursor.y = pos.y - self.offset.y;
                        self.cursor.x = pos.x.saturating_add(c_len) - self.offset.x;
                        self.recalculate_graphemes();
                        let string = self.doc.rows[pos.y].string.clone();
                        self.doc.rows[pos.y].delete(raw_to_grapheme(pos.x, &string));
                        for _ in 0..c_len {
                            self.move_cursor(Key::Left);
                        }
                    }
                    Event::BackspaceMid(pos, c) => {
                        self.doc.rows[pos.y].insert(*c, pos.x);
                        self.move_cursor(Key::Right);
                    }
                    Event::ReturnEnd(pos) => {
                        self.doc.rows.remove(pos.y + 1);
                        self.move_cursor(Key::Up);
                        self.leap_cursor(Key::End);
                    }
                    Event::ReturnStart(pos) => {
                        self.doc.rows.remove(pos.y);
                        self.move_cursor(Key::Up);
                    }
                    Event::ReturnMid(pos, breakpoint) => {
                        let current = self.doc.rows[pos.y].string.clone();
                        let after = self.doc.rows[pos.y + 1].string.clone();
                        self.doc.rows.remove(pos.y);
                        self.doc.rows[pos.y] = Row::from(&(current + &after)[..]);
                        self.move_cursor(Key::Up);
                        self.leap_cursor(Key::Home);
                        for _ in 0..*breakpoint {
                            self.move_cursor(Key::Right);
                        }
                    }
                    Event::BackspaceStart(pos) => {
                        let before = Row::from(&self.doc.rows[pos.y].string[..pos.x]);
                        let after = Row::from(&self.doc.rows[pos.y].string[pos.x..]);
                        self.doc.rows[pos.y] = after;
                        self.doc.rows.insert(pos.y, before);
                        self.move_cursor(Key::Down);
                        self.leap_cursor(Key::Home);
                    }
                    Event::UpdateLine(pos, before, _) => {
                        self.doc.rows[*pos] = before.clone();
                        self.snap_cursor();
                        self.prevent_unicode_hell();
                        self.recalculate_graphemes();
                    }
                }
                self.dirty = true;
                self.show_welcome = false;
            }
            self.doc.redo_stack.append(events);
        } else {
            self.set_command_line("Empty Undo Stack".to_string(), Type::Error);
        }
    }
    fn set_command_line(&mut self, text: String, msg: Type) {
        self.command_line = CommandLine { text, msg };
    }
    fn character(&mut self, c: char) {
        // The user pressed a character key
        self.dirty = true;
        self.show_welcome = false;
        match c {
            '\n' => self.return_key(), // The user pressed the return key
            '\t' => {
                // The user pressed the tab key
                self.tab();
                self.doc.undo_stack.push(Event::InsertTab(Position {
                    x: self.cursor.x + self.offset.x,
                    y: self.cursor.y + self.offset.y,
                }));
            }
            _ => {
                // Other characters
                // TODO: Update relavent lines here
                self.dirty = true;
                self.show_welcome = false;
                self.doc.rows[self.cursor.y + self.offset.y].insert(c, self.graphemes);
                self.doc.undo_stack.push(Event::InsertMid(
                    Position {
                        x: self.cursor.x + self.offset.x,
                        y: self.cursor.y + self.offset.y,
                    },
                    c,
                ));
                if c == ' ' {
                    self.doc.undo_stack.commit();
                }
                self.move_cursor(Key::Right);
            }
        }
        self.doc.redo_stack.empty();
    }
    fn tab(&mut self) {
        // Insert a tab
        // TODO: Update relavent lines here
        for _ in 0..self.config.general.tab_width {
            self.doc.rows[self.cursor.y + self.offset.y].insert(' ', self.graphemes);
            self.move_cursor(Key::Right);
        }
    }
    fn return_key(&mut self) {
        // Return key
        self.dirty = true;
        self.show_welcome = false;
        // TODO: Update relavent lines here
        if self.cursor.x + self.offset.x == 0 {
            // Return key pressed at the start of the line
            self.doc
                .rows
                .insert(self.cursor.y + self.offset.y, Row::from(""));
            self.doc.undo_stack.push(Event::ReturnStart(Position {
                x: self.cursor.x + self.offset.x,
                y: self.cursor.y + self.offset.y,
            }));
            self.move_cursor(Key::Down);
        } else if self.cursor.x + self.offset.x
            == self.doc.rows[self.cursor.y + self.offset.y].length()
        {
            // Return key pressed at the end of the line
            self.doc
                .rows
                .insert(self.cursor.y + self.offset.y + 1, Row::from(""));
            self.doc.undo_stack.push(Event::ReturnEnd(Position {
                x: self.cursor.x + self.offset.x,
                y: self.cursor.y + self.offset.y,
            }));
            self.move_cursor(Key::Down);
            self.leap_cursor(Key::Home);
            self.recalculate_graphemes();
        } else {
            // Return key pressed in the middle of the line
            let current = self.doc.rows[self.cursor.y + self.offset.y].chars();
            let before = Row::from(&current[..self.graphemes].join("")[..]);
            let after = Row::from(&current[self.graphemes..].join("")[..]);
            self.doc
                .rows
                .insert(self.cursor.y + self.offset.y + 1, after);
            self.doc.rows[self.cursor.y + self.offset.y] = before.clone();
            self.doc.undo_stack.push(Event::ReturnMid(
                Position {
                    x: self.cursor.x + self.offset.x,
                    y: self.cursor.y + self.offset.y,
                },
                before.length(),
            ));
            self.move_cursor(Key::Down);
            self.leap_cursor(Key::Home);
        }
        self.doc.undo_stack.commit();
    }
    fn backspace(&mut self) {
        // Handling the backspace key
        self.dirty = true;
        self.show_welcome = false;
        // TODO: Update relavent lines here
        if self.cursor.x + self.offset.x == 0 && self.cursor.y + self.offset.y != 0 {
            // Backspace at the start of a line
            let current = self.doc.rows[self.cursor.y + self.offset.y].string.clone();
            let prev = self.doc.rows[self.cursor.y + self.offset.y - 1].clone();
            self.doc.rows[self.cursor.y + self.offset.y - 1] =
                Row::from(&(prev.string.clone() + &current)[..]);
            self.doc.rows.remove(self.cursor.y + self.offset.y);
            self.move_cursor(Key::Up);
            self.cursor.x = prev.length();
            self.recalculate_graphemes();
            self.doc.undo_stack.push(Event::BackspaceStart(Position {
                x: self.cursor.x + self.offset.x,
                y: self.cursor.y + self.offset.y,
            }));
            self.doc.undo_stack.commit();
        } else {
            // Backspace in the middle of a line
            self.move_cursor(Key::Left);
            let ch = self.doc.rows[self.cursor.y + self.offset.y].clone();
            self.doc.rows[self.cursor.y + self.offset.y].delete(self.graphemes);
            if let Some(ch) = ch.chars().get(self.graphemes) {
                if let Ok(ch) = ch.parse() {
                    self.doc.undo_stack.push(Event::BackspaceMid(
                        Position {
                            x: self.cursor.x + self.offset.x,
                            y: self.cursor.y + self.offset.y,
                        },
                        ch,
                    ));
                }
            }
        }
    }
    fn quit(&mut self) {
        // For handling a quit event
        if self.dirty_prompt('q', "quit") {
            self.quit = true;
        }
    }
    fn new_document(&mut self) {
        // Handle new document event
        if self.dirty_prompt('n', "new") {
            self.doc = Document::new(&self.config);
            self.dirty = false;
            self.cursor.y = 0;
            self.offset.y = 0;
            self.leap_cursor(Key::Home);
        }
    }
    fn open_document(&mut self) {
        // Handle new document event
        // TODO: Highlight entire file here
        if self.dirty_prompt('o', "open") {
            if let Some(result) = self.prompt("Open", &|_, _, _| {}) {
                if let Some(doc) = Document::open(&self.config, &result[..]) {
                    self.doc = doc;
                    self.dirty = false;
                    self.show_welcome = false;
                    self.cursor.y = 0;
                    self.offset.y = 0;
                    self.leap_cursor(Key::Home);
                } else {
                    self.set_command_line("File couldn't be opened".to_string(), Type::Error);
                }
            }
        } else {
            self.set_command_line("Open cancelled".to_string(), Type::Info);
        }
    }
    fn save(&mut self) {
        // Handle save event
        if self.doc.save().is_ok() {
            self.dirty = false;
            self.set_command_line(
                format!("File saved to {} successfully", self.doc.path),
                Type::Info,
            );
        } else {
            self.set_command_line(
                format!("Failed to save file to {}", self.doc.path),
                Type::Error,
            );
        }
        self.doc.undo_stack.commit();
    }
    fn save_as(&mut self) {
        // Handle save as event
        if let Some(result) = self.prompt("Save as", &|_, _, _| {}) {
            if self.doc.save_as(&result[..]).is_ok() {
                self.dirty = false;
                self.set_command_line(format!("File saved to {} successfully", result), Type::Info);
                self.doc.name = result.clone();
                self.doc.path = result;
            } else {
                self.set_command_line(format!("Failed to save file to {}", result), Type::Error);
            }
        } else {
            self.set_command_line("Save as cancelled".to_string(), Type::Info);
        }
        self.doc.undo_stack.commit();
    }
    fn search(&mut self) {
        // For searching the file
        let initial_cursor = self.cursor;
        let initial_offset = self.offset;
        self.prompt("Search", &|s, e, t| {
            let search_points = s.doc.scan(t);
            match e {
                PromptEvent::KeyPress(k) => match k {
                    Key::Left | Key::Up => {
                        for p in search_points.iter().rev() {
                            if is_behind(&s.cursor, &s.offset, &p) {
                                s.goto(&p);
                                s.recalculate_graphemes();
                                break;
                            }
                        }
                    }
                    Key::Right | Key::Down => {
                        for p in search_points {
                            if is_ahead(&s.cursor, &s.offset, &p) {
                                s.goto(&p);
                                s.recalculate_graphemes();
                                break;
                            }
                        }
                    }
                    Key::Esc => {
                        s.cursor = initial_cursor;
                        s.offset = initial_offset;
                        s.recalculate_graphemes();
                    }
                    _ => (),
                },
                PromptEvent::CharPress => {
                    s.cursor = initial_cursor;
                    s.offset = initial_offset;
                    if t != "" {
                        for p in search_points {
                            if is_ahead(&s.cursor, &s.offset, &p) {
                                s.goto(&p);
                                s.recalculate_graphemes();
                                break;
                            }
                        }
                    }
                }
                PromptEvent::Update => (),
            }
        });
        self.set_command_line("Search exited".to_string(), Type::Info);
    }
    fn replace(&mut self) {
        let initial_cursor = self.cursor;
        let initial_offset = self.offset;
        if let Some(target) = self.prompt("Replace", &|_, _, _| {}) {
            if let Some(arrow) = self.prompt("With", &|_, _, _| {}) {
                let re = Regex::new(&target).unwrap();
                let mut search_points = self.doc.scan(&target);
                for p in &search_points {
                    if is_ahead(&self.cursor, &self.offset, &p) {
                        self.goto(&p);
                        self.recalculate_graphemes();
                        self.update();
                        break;
                    }
                }
                loop {
                    let key = self.read_key();
                    match key {
                        Key::Up | Key::Left => {
                            for p in (&search_points).iter().rev() {
                                if is_behind(&self.cursor, &self.offset, &p) {
                                    self.goto(&p);
                                    self.recalculate_graphemes();
                                    self.update();
                                    break;
                                }
                            }
                        }
                        Key::Down | Key::Right => {
                            for p in &search_points {
                                if is_ahead(&self.cursor, &self.offset, &p) {
                                    self.goto(&p);
                                    self.recalculate_graphemes();
                                    self.update();
                                    break;
                                }
                            }
                        }
                        Key::Char('\n') | Key::Char('y') | Key::Char(' ') => {
                            self.doc.undo_stack.commit();
                            let line = self.doc.rows[self.cursor.y + self.offset.y].clone();
                            let before = self.doc.rows[self.cursor.y + self.offset.y].clone();
                            let after = Row::from(&*re.replace_all(&line.string[..], &arrow[..]));
                            if before.string != after.string {
                                self.doc.undo_stack.push(Event::UpdateLine(
                                    self.cursor.y + self.offset.y,
                                    before.clone(),
                                    after.clone(),
                                ));
                                // TODO: Update relavent lines here
                                self.doc.rows[self.cursor.y + self.offset.y] = after;
                            }
                            self.update();
                            self.snap_cursor();
                            self.prevent_unicode_hell();
                            self.recalculate_graphemes();
                            search_points = self.doc.scan(&target);
                        }
                        Key::Esc => break,
                        _ => (),
                    }
                }
                self.cursor = initial_cursor;
                self.offset = initial_offset;
                self.set_command_line("Replace finished".to_string(), Type::Info);
            }
        }
    }
    fn replace_all(&mut self) {
        if let Some(target) = self.prompt("Replace", &|_, _, _| {}) {
            if let Some(arrow) = self.prompt("With", &|_, _, _| {}) {
                self.doc.undo_stack.commit();
                let re = Regex::new(&target).unwrap();
                let lines = self.doc.rows.clone();
                for (c, line) in lines.iter().enumerate() {
                    let before = self.doc.rows[c].clone();
                    let after = Row::from(&*re.replace_all(&line.string[..], &arrow[..]));
                    if before.string != after.string {
                        self.doc.undo_stack.push(Event::UpdateLine(
                            c,
                            before.clone(),
                            after.clone(),
                        ));
                        // TODO: Update relavent lines here
                        self.doc.rows[c] = after;
                    }
                }
            }
        }
        self.snap_cursor();
        self.prevent_unicode_hell();
        self.recalculate_graphemes();
        self.set_command_line("Replaced targets".to_string(), Type::Info);
    }
    fn dirty_prompt(&mut self, key: char, subject: &str) -> bool {
        // For events that require changes to the document
        if self.dirty {
            // Handle unsaved changes
            self.set_command_line(
                format!(
                    "Unsaved Changes! Ctrl + {} to force {}",
                    key.to_uppercase(),
                    subject
                ),
                Type::Warning,
            );
            self.update();
            match self.read_key() {
                Key::Char('\n') => return true,
                Key::Ctrl(k) => {
                    if k == key {
                        return true;
                    } else {
                        self.set_command_line(format!("{} cancelled", title(subject)), Type::Info);
                    }
                }
                _ => self.set_command_line(format!("{} cancelled", title(subject)), Type::Info),
            }
        } else {
            return true;
        }
        false
    }
    fn prompt(
        &mut self,
        prompt: &str,
        func: &dyn Fn(&mut Self, PromptEvent, &str),
    ) -> Option<String> {
        // Create a new prompt
        self.set_command_line(format!("{}: ", prompt), Type::Info);
        self.update();
        let mut result = String::new();
        'p: loop {
            let key = self.read_key();
            match key {
                Key::Char(c) => {
                    if c == '\n' {
                        break 'p;
                    } else {
                        result.push(c);
                    }
                    func(self, PromptEvent::CharPress, &result)
                }
                Key::Backspace => {
                    result.pop();
                    func(self, PromptEvent::CharPress, &result)
                }
                Key::Esc => {
                    func(self, PromptEvent::KeyPress(key), &result);
                    return None;
                }
                _ => func(self, PromptEvent::KeyPress(key), &result),
            }
            self.set_command_line(format!("{}: {}", prompt, result), Type::Info);
            func(self, PromptEvent::Update, &result);
            self.update();
        }
        Some(result)
    }
    fn leap_cursor(&mut self, action: Key) {
        match action {
            Key::PageUp => {
                self.cursor.y = 0;
                self.snap_cursor();
                self.prevent_unicode_hell();
                self.recalculate_graphemes();
            }
            Key::PageDown => {
                self.cursor.y = cmp::min(
                    self.doc.rows.len().saturating_sub(1),
                    self.term.height.saturating_sub(3) as usize,
                );
                self.snap_cursor();
                self.prevent_unicode_hell();
                self.recalculate_graphemes();
            }
            Key::Home => {
                self.offset.x = 0;
                self.cursor.x = 0;
                self.graphemes = 0;
            }
            Key::End => {
                let line = &self.doc.rows[self.cursor.y + self.offset.y];
                if line.length()
                    >= self.term.width.saturating_sub(self.doc.line_offset as u16) as usize
                {
                    // Work out the width of the character to traverse
                    let mut jump = 1;
                    if let Some(chr) = line.ext_chars().get(line.length()) {
                        jump = UnicodeWidthStr::width(*chr);
                    }
                    self.offset.x = line
                        .length()
                        .saturating_add(jump + self.doc.line_offset + 1)
                        .saturating_sub(self.term.width as usize);
                    self.cursor.x = self
                        .term
                        .width
                        .saturating_sub((jump + self.doc.line_offset + 1) as u16)
                        as usize;
                } else {
                    self.cursor.x = line.length();
                }
                self.graphemes = line.chars().len();
            }
            _ => (),
        }
    }
    fn move_cursor(&mut self, direction: Key) {
        // Move the cursor around the editor
        match direction {
            Key::Down => {
                if self.cursor.y + self.offset.y + 1 < self.doc.rows.len() {
                    // If the proposed move is within the length of the document
                    if self.cursor.y == self.term.height.saturating_sub(3) as usize {
                        self.offset.y = self.offset.y.saturating_add(1);
                    } else {
                        self.cursor.y = self.cursor.y.saturating_add(1);
                    }
                    self.snap_cursor();
                    self.prevent_unicode_hell();
                    self.recalculate_graphemes();
                }
            }
            Key::Up => {
                if self.cursor.y == 0 {
                    self.offset.y = self.offset.y.saturating_sub(1);
                } else {
                    self.cursor.y = self.cursor.y.saturating_sub(1);
                }
                self.snap_cursor();
                self.prevent_unicode_hell();
                self.recalculate_graphemes();
            }
            Key::Right => {
                let line = &self.doc.rows[self.cursor.y + self.offset.y];
                // Work out the width of the character to traverse
                let mut jump = 1;
                if let Some(chr) = line.ext_chars().get(self.cursor.x + self.offset.x) {
                    jump = UnicodeWidthStr::width(*chr);
                }
                // Check the proposed move is within the current line length
                if line.length() > self.cursor.x + self.offset.x {
                    // Check for normal width character
                    let indicator1 = self.cursor.x
                        == self
                            .term
                            .width
                            .saturating_sub((self.doc.line_offset + jump + 1) as u16)
                            as usize;
                    // Check for half broken unicode character
                    let indicator2 = self.cursor.x
                        == self
                            .term
                            .width
                            .saturating_sub((self.doc.line_offset + jump) as u16)
                            as usize;
                    if indicator1 || indicator2 {
                        self.offset.x = self.offset.x.saturating_add(jump);
                    } else {
                        self.cursor.x = self.cursor.x.saturating_add(jump);
                    }
                    self.graphemes = self.graphemes.saturating_add(1);
                }
            }
            Key::Left => {
                let line = &self.doc.rows[self.cursor.y + self.offset.y];
                // Work out the width of the character to traverse
                let mut jump = 1;
                if let Some(chr) = line
                    .ext_chars()
                    .get((self.cursor.x + self.offset.x).saturating_sub(1))
                {
                    jump = UnicodeWidthStr::width(*chr);
                }
                if self.cursor.x == 0 {
                    self.offset.x = self.offset.x.saturating_sub(jump);
                } else {
                    self.cursor.x = self.cursor.x.saturating_sub(jump);
                }
                self.graphemes = self.graphemes.saturating_sub(1);
            }
            _ => (),
        }
    }
    fn snap_cursor(&mut self) {
        // Snap the cursor to the end of the row when outside
        let current = self.doc.rows[self.cursor.y + self.offset.y].clone();
        if current.length() <= self.cursor.x + self.offset.x {
            // If the cursor is out of bounds
            self.leap_cursor(Key::Home);
            self.leap_cursor(Key::End);
        }
    }
    fn prevent_unicode_hell(&mut self) {
        // Make sure that the cursor isn't inbetween a unicode character
        let line = &self.doc.rows[self.cursor.y + self.offset.y];
        if line.length() > self.cursor.x + self.offset.x {
            // As long as the cursor is within range
            let boundaries = line.boundaries();
            let mut index = self.cursor.x + self.offset.x;
            if !boundaries.contains(&index) && index != 0 {}
            while !boundaries.contains(&index) && index != 0 {
                self.cursor.x = self.cursor.x.saturating_sub(1);
                self.graphemes = self.graphemes.saturating_sub(1);
                index = index.saturating_sub(1);
            }
        }
    }
    fn recalculate_graphemes(&mut self) {
        // Recalculate the grapheme cursor after moving up and down
        let current = self.doc.rows[self.cursor.y + self.offset.y].clone();
        let jumps = current.get_jumps();
        let mut counter = 0;
        for (mut counter2, i) in jumps.into_iter().enumerate() {
            if counter == self.cursor.x + self.offset.x {
                break;
            }
            counter2 += 1;
            self.graphemes = counter2;
            counter += i;
        }
    }
    fn goto(&mut self, pos: &Position) {
        // Move the cursor to a specific location
        let max_y = self.term.height.saturating_sub(3) as usize;
        let max_x = (self.term.width as usize).saturating_sub(self.doc.line_offset);
        let halfway_y = max_y / 2;
        let halfway_x = max_x / 2;
        if self.offset.x == 0 && pos.y < max_y && pos.x < max_x {
            // Cursor is on the screen
            self.offset = Position { x: 0, y: 0 };
            self.cursor = *pos;
        } else {
            // Cursor is off the screen, move to the Y position
            self.offset.y = pos.y.saturating_sub(halfway_y);
            self.cursor.y = halfway_y;
            if self.offset.y + self.cursor.y != pos.y {
                // Fix cursor misplacement
                self.offset = Position { x: 0, y: 0 };
                self.cursor = *pos;
                return;
            }
            // Change the X
            if pos.x >= max_x {
                // Move to the center
                self.offset.x = pos.x.saturating_sub(halfway_x);
                self.cursor.x = halfway_x;
            } else {
                // No offset
                self.offset.x = 0;
                self.cursor.x = pos.x;
            }
        }
    }
    fn update(&mut self) {
        // Move the cursor and render the screen
        self.term.goto(&Position { x: 0, y: 0 });
        self.doc.recalculate_offset(&self.config);
        self.render();
        self.term.goto(&Position {
            x: self.cursor.x.saturating_add(self.doc.line_offset),
            y: self.cursor.y,
        });
        self.term.flush();
    }
    fn welcome_message(&self, text: &str, colour: color::Fg<color::Rgb>) -> String {
        let pad = " ".repeat((self.term.width as usize / 2).saturating_sub(text.len() / 2));
        let pad_right = " ".repeat(
            (self.term.width.saturating_sub(1) as usize)
                .saturating_sub(text.len() + pad.len())
                .saturating_sub(self.config.general.line_number_padding_left),
        );
        format!(
            "{}{}{}~{}{}{}{}{}{}",
            Reader::rgb_bg(self.config.theme.editor_bg),
            Reader::rgb_fg(self.config.theme.line_number_fg),
            " ".repeat(self.config.general.line_number_padding_left),
            RESET_FG,
            colour,
            trim_end(
                &format!("{}{}", pad, text),
                self.term.width.saturating_sub(1) as usize
            ),
            pad_right,
            RESET_FG,
            RESET_BG,
        )
    }
    fn status_line(&mut self) -> String {
        // Produce the status line
        // Create the left part of the status line
        let left = format!(
            " {}{} \u{2502} {} ",
            self.doc.name,
            if self.dirty {
                "[+] \u{fb12} "
            } else {
                " \u{f723} "
            },
            self.doc.identify()
        );
        // Create the right part of the status line
        let right = format!(
            " \u{fa70} {} / {} \u{2502} \u{fae6}({}, {}) ",
            self.cursor.y + self.offset.y + 1,
            self.doc.rows.len(),
            self.cursor.x + self.offset.x,
            self.cursor.y + self.offset.y,
        );
        // Get the padding value
        let padding = self.term.align_break(&left, &right);
        // Generate it
        format!(
            "{}{}{}{}{}{}{}",
            style::Bold,
            Reader::rgb_fg(self.config.theme.status_fg),
            Reader::rgb_bg(self.config.theme.status_bg),
            trim_end(
                &format!("{}{}{}", left, padding, right),
                self.term.width as usize
            ),
            RESET_BG,
            RESET_FG,
            style::Reset,
        )
    }
    fn add_background(&self, text: &str) -> String {
        // Add a background colour to a line
        format!(
            "{}{}{}{}",
            Reader::rgb_bg(self.config.theme.editor_bg),
            text,
            self.term.align_left(&text),
            RESET_BG
        )
    }
    fn command_line(&self) -> String {
        // Render the command line
        let line = &self.command_line.text;
        // Add the correct styling
        match self.command_line.msg {
            Type::Error => self.add_background(&format!(
                "{}{}{}{}{}",
                style::Bold,
                color::Fg(color::Red),
                self.add_background(&trim_end(&line, self.term.width as usize)),
                color::Fg(color::Reset),
                style::Reset
            )),
            Type::Warning => self.add_background(&format!(
                "{}{}{}{}{}",
                style::Bold,
                color::Fg(color::Yellow),
                self.add_background(&trim_end(&line, self.term.width as usize)),
                color::Fg(color::Reset),
                style::Reset
            )),
            Type::Info => self.add_background(&trim_end(&line, self.term.width as usize)),
        }
    }
    fn render(&mut self) {
        // Draw the screen to the terminal
        let mut frame = vec![];
        for row in 0..self.term.height {
            if let Some(row) = self.doc.rows.get_mut(self.offset.y + row as usize) {
                row.update_syntax(&self.config, &self.regex);
            }
            if row == self.term.height - 1 {
                // Render command line
                frame.push(self.command_line());
            } else if row == self.term.height - 2 {
                // Render status line
                frame.push(self.status_line());
            } else if row == self.term.height / 4 && self.show_welcome {
                frame.push(self.welcome_message(
                    &format!("Ox editor  v{}", VERSION),
                    Reader::rgb_fg(self.config.theme.editor_fg),
                ));
            } else if row == (self.term.height / 4).saturating_add(1) && self.show_welcome {
                frame.push(self.welcome_message(
                    "A Rust powered editor by Luke",
                    Reader::rgb_fg(self.config.theme.editor_fg),
                ));
            } else if row == (self.term.height / 4).saturating_add(3) && self.show_welcome {
                frame.push(self.welcome_message(
                    "Ctrl + Q: Exit   ",
                    Reader::rgb_fg(self.config.theme.status_fg),
                ));
            } else if row == (self.term.height / 4).saturating_add(4) && self.show_welcome {
                frame.push(self.welcome_message(
                    "Ctrl + S: Save   ",
                    Reader::rgb_fg(self.config.theme.status_fg),
                ));
            } else if row == (self.term.height / 4).saturating_add(5) && self.show_welcome {
                frame.push(self.welcome_message(
                    "Ctrl + W: Save as",
                    Reader::rgb_fg(self.config.theme.status_fg),
                ));
            } else if let Some(line) = self.doc.rows.get(self.offset.y + row as usize) {
                // Render lines of code
                frame.push(self.add_background(&line.render(
                    self.offset.x,
                    self.term.width as usize,
                    self.offset.y + row as usize,
                    self.doc.line_offset,
                    &self.config,
                )));
            } else {
                // Render empty lines
                frame.push(format!(
                    "{}{}{}",
                    Reader::rgb_fg(self.config.theme.line_number_fg),
                    self.add_background(&format!(
                        "{}~",
                        " ".repeat(self.config.general.line_number_padding_left)
                    )),
                    RESET_FG
                ));
            }
        }
        print!("{}", frame.join("\r\n"));
    }
}
