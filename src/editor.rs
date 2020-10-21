// Editor.rs - Controls the editor and brings everything together
use crate::config::{Reader, Status};
use crate::util::{is_ahead, is_behind, raw_to_grapheme, title, trim_end, Exp};
use crate::{Document, Event, Row, Terminal, VERSION};
use clap::App;
use regex::Regex;
use std::time::{Duration, Instant};
use std::{cmp, io::Error, thread};
use termion::event::Key;
use termion::input::{Keys, TermRead};
use termion::{async_stdin, color, style, AsyncReader};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

// Set up color resets
pub const RESET_BG: color::Bg<color::Reset> = color::Bg(color::Reset);
pub const RESET_FG: color::Fg<color::Reset> = color::Fg(color::Reset);

// Set up offset rules
pub const OFFSET: usize = 1;

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
    command_line: CommandLine,      // For holding the command line
    term: Terminal,                 // For the handling of the terminal
    doc: Vec<Document>,             // For holding our document
    tab: usize,                     // Holds the number of the current tab
    last_keypress: Option<Instant>, // For holding the time of the last input event
    stdin: Keys<AsyncReader>,       // Asynchronous stdin
    exp: Exp,                       // For holding expressions
}

// Implementing methods for our editor struct / class
impl Editor {
    pub fn new(args: App) -> Result<Self, Error> {
        // Create a new editor instance
        let args = args.get_matches();
        // Set up the arguments
        let files: Vec<&str> = args.values_of("files").unwrap_or_default().collect();
        let config = Reader::read(args.value_of("config").unwrap_or_default());
        let mut documents = vec![];
        if files.is_empty() {
            documents.push(Document::new(&config.0));
        } else {
            for file in &files {
                documents.push(Document::from(&config.0, file))
            }
        }
        // Create the new editor instance
        Ok(Self {
            quit: false,
            // Display information about the config file into text for the status line
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
            tab: 0,
            doc: documents,
            last_keypress: None,
            stdin: async_stdin().keys(),
            config: config.0.clone(),
            exp: Exp::new(),
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
                    if self.doc[self.tab].cursor.y > self.term.height.saturating_sub(3) as usize {
                        // Prevent cursor going off the screen and breaking everything
                        self.doc[self.tab].cursor.y = self.term.height.saturating_sub(3) as usize;
                    }
                    // Re-render everything to the new size
                    self.update();
                }
                // Check for a period of inactivity
                if let Some(time) = self.last_keypress {
                    // Check to see if it's over the config undo period
                    if time.elapsed().as_secs() >= self.config.general.undo_period {
                        // Commit the undo changes to the stack
                        self.doc[self.tab].undo_stack.commit();
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
            Key::Ctrl('d') => self.prev_tab(),
            Key::Ctrl('h') => self.next_tab(),
            Key::Left | Key::Right | Key::Up | Key::Down => self.move_cursor(key),
            Key::PageDown | Key::PageUp | Key::Home | Key::End => self.leap_cursor(key),
            _ => (),
        }
    }
    fn next_tab(&mut self) {
        if self.tab.saturating_add(1) < self.doc.len() {
            self.tab = self.tab.saturating_add(1);
        }
    }
    fn prev_tab(&mut self) {
        self.tab = self.tab.saturating_sub(1);
    }
    fn redo(&mut self) {
        // Redo an action
        if let Some(events) = self.doc[self.tab].redo_stack.pop() {
            for event in events.iter().rev() {
                // Reverse the undo action
                match event {
                    // TODO: Update relavent lines here
                    Event::InsertTab(pos) => {
                        self.doc[self.tab].cursor.y = pos.y - self.doc[self.tab].offset.y + OFFSET;
                        self.doc[self.tab].cursor.x =
                            pos.x.saturating_sub(self.config.general.tab_width)
                                - self.doc[self.tab].offset.x;
                        self.recalculate_graphemes();
                        self.tab();
                    }
                    Event::InsertMid(pos, c) => {
                        let c_len = UnicodeWidthChar::width(*c).map_or(0, |c| c);
                        self.doc[self.tab].cursor.y = pos.y - self.doc[self.tab].offset.y + OFFSET;
                        self.doc[self.tab].cursor.x =
                            pos.x.saturating_add(c_len) - self.doc[self.tab].offset.x;
                        self.recalculate_graphemes();
                        self.doc[self.tab].rows[pos.y].insert(*c, pos.x);
                    }
                    Event::BackspaceMid(pos, _) => {
                        self.doc[self.tab].cursor.y = pos.y - self.doc[self.tab].offset.y + OFFSET;
                        self.doc[self.tab].cursor.x = pos.x - self.doc[self.tab].offset.x;
                        self.recalculate_graphemes();
                        self.doc[self.tab].rows[pos.y].delete(pos.x);
                    }
                    Event::ReturnEnd(pos) => {
                        self.doc[self.tab].cursor.y = pos.y - self.doc[self.tab].offset.y + OFFSET;
                        self.doc[self.tab].cursor.x = pos.x - self.doc[self.tab].offset.x;
                        self.recalculate_graphemes();
                        self.doc[self.tab].rows.insert(pos.y + 1, Row::from(""));
                        self.move_cursor(Key::Down);
                    }
                    Event::ReturnStart(pos) => {
                        self.doc[self.tab].cursor.y = pos.y - self.doc[self.tab].offset.y + OFFSET;
                        self.doc[self.tab].cursor.x = pos.x - self.doc[self.tab].offset.x;
                        self.recalculate_graphemes();
                        self.doc[self.tab].rows.insert(pos.y, Row::from(""));
                        self.move_cursor(Key::Down);
                    }
                    Event::ReturnMid(pos, breakpoint) => {
                        self.doc[self.tab].cursor.y = pos.y - self.doc[self.tab].offset.y + OFFSET;
                        self.doc[self.tab].cursor.x = pos.x - self.doc[self.tab].offset.x;
                        self.recalculate_graphemes();
                        let current = self.doc[self.tab].rows[pos.y].string.clone();
                        let before = Row::from(&current[..*breakpoint]);
                        let after = Row::from(&current[*breakpoint..]);
                        self.doc[self.tab].rows.insert(pos.y + 1, after);
                        self.doc[self.tab].rows[pos.y] = before;
                        self.move_cursor(Key::Down);
                        self.leap_cursor(Key::Home);
                    }
                    Event::BackspaceStart(pos) => {
                        self.doc[self.tab].cursor.y = pos.y - self.doc[self.tab].offset.y + OFFSET;
                        self.recalculate_graphemes();
                        let current = self.doc[self.tab].rows[pos.y + 1].string.clone();
                        let prev = self.doc[self.tab].rows[pos.y].clone();
                        self.doc[self.tab].rows[pos.y + 1] =
                            Row::from(&(prev.string.clone() + &current)[..]);
                        self.doc[self.tab].rows.remove(pos.y);
                        self.move_cursor(Key::Up);
                        self.doc[self.tab].cursor.x = prev.length();
                        self.recalculate_graphemes();
                    }
                    Event::UpdateLine(pos, _, after) => {
                        self.doc[self.tab].rows[*pos] = *after.clone();
                        self.snap_cursor();
                        self.prevent_unicode_hell();
                        self.recalculate_graphemes();
                    }
                }
                self.doc[self.tab].dirty = true;
                self.doc[self.tab].show_welcome = false;
            }
            self.doc[self.tab].undo_stack.append(events);
        } else {
            self.set_command_line("Empty Redo Stack".to_string(), Type::Error);
        }
    }
    fn undo(&mut self) {
        // Initiate an undo action
        self.doc[self.tab].undo_stack.commit();
        if let Some(events) = self.doc[self.tab].undo_stack.pop() {
            for event in &events {
                // Undo the previous action
                match event {
                    // TODO: Update relavent lines here
                    Event::InsertTab(pos) => {
                        for i in 1..=self.config.general.tab_width {
                            self.doc[self.tab].rows[pos.y].delete(pos.x - i);
                            self.move_cursor(Key::Left);
                        }
                    }
                    Event::InsertMid(pos, c) => {
                        let c_len = UnicodeWidthChar::width(*c).map_or(0, |c| c);
                        self.doc[self.tab].cursor.y = pos.y - self.doc[self.tab].offset.y + OFFSET;
                        self.doc[self.tab].cursor.x =
                            pos.x.saturating_add(c_len) - self.doc[self.tab].offset.x;
                        self.recalculate_graphemes();
                        let string = self.doc[self.tab].rows[pos.y].string.clone();
                        self.doc[self.tab].rows[pos.y].delete(raw_to_grapheme(pos.x, &string));
                        for _ in 0..c_len {
                            self.move_cursor(Key::Left);
                        }
                    }
                    Event::BackspaceMid(pos, c) => {
                        self.doc[self.tab].rows[pos.y].insert(*c, pos.x);
                        self.move_cursor(Key::Right);
                    }
                    Event::ReturnEnd(pos) => {
                        self.doc[self.tab].rows.remove(pos.y + 1);
                        self.move_cursor(Key::Up);
                        self.leap_cursor(Key::End);
                    }
                    Event::ReturnStart(pos) => {
                        self.doc[self.tab].rows.remove(pos.y);
                        self.move_cursor(Key::Up);
                    }
                    Event::ReturnMid(pos, breakpoint) => {
                        let current = self.doc[self.tab].rows[pos.y].string.clone();
                        let after = self.doc[self.tab].rows[pos.y + 1].string.clone();
                        self.doc[self.tab].rows.remove(pos.y);
                        self.doc[self.tab].rows[pos.y] = Row::from(&(current + &after)[..]);
                        self.move_cursor(Key::Up);
                        self.leap_cursor(Key::Home);
                        for _ in 0..*breakpoint {
                            self.move_cursor(Key::Right);
                        }
                    }
                    Event::BackspaceStart(pos) => {
                        let before = Row::from(&self.doc[self.tab].rows[pos.y].string[..pos.x]);
                        let after = Row::from(&self.doc[self.tab].rows[pos.y].string[pos.x..]);
                        self.doc[self.tab].rows[pos.y] = after;
                        self.doc[self.tab].rows.insert(pos.y, before);
                        self.move_cursor(Key::Down);
                        self.leap_cursor(Key::Home);
                    }
                    Event::UpdateLine(pos, before, _) => {
                        self.doc[self.tab].rows[*pos] = *before.clone();
                        self.snap_cursor();
                        self.prevent_unicode_hell();
                        self.recalculate_graphemes();
                    }
                }
                self.doc[self.tab].dirty = true;
                self.doc[self.tab].show_welcome = false;
            }
            self.doc[self.tab].redo_stack.append(events);
        } else {
            self.set_command_line("Empty Undo Stack".to_string(), Type::Error);
        }
    }
    fn set_command_line(&mut self, text: String, msg: Type) {
        // Function to update the command line
        self.command_line = CommandLine { text, msg };
    }
    fn character(&mut self, c: char) {
        // The user pressed a character key
        self.doc[self.tab].dirty = true;
        self.doc[self.tab].show_welcome = false;
        let cursor = self.doc[self.tab].cursor;
        let offset = self.doc[self.tab].offset;
        let graphemes = self.doc[self.tab].graphemes;
        match c {
            '\n' => self.return_key(), // The user pressed the return key
            '\t' => {
                // The user pressed the tab key
                self.tab();
                self.doc[self.tab]
                    .undo_stack
                    .push(Event::InsertTab(Position {
                        x: cursor.x + offset.x,
                        y: cursor.y + offset.y - OFFSET,
                    }));
            }
            _ => {
                // Other characters
                // TODO: Update relavent lines here
                self.doc[self.tab].dirty = true;
                self.doc[self.tab].show_welcome = false;
                self.doc[self.tab].rows[cursor.y + offset.y - OFFSET].insert(c, graphemes);
                self.doc[self.tab].undo_stack.push(Event::InsertMid(
                    Position {
                        x: cursor.x + offset.x,
                        y: cursor.y + offset.y - OFFSET,
                    },
                    c,
                ));
                // Commit to the undo stack if space key pressed
                if c == ' ' {
                    self.doc[self.tab].undo_stack.commit();
                }
                self.move_cursor(Key::Right);
            }
        }
        // Wipe the redo stack to avoid conflicts
        self.doc[self.tab].redo_stack.empty();
    }
    fn tab(&mut self) {
        // Insert a tab
        let cursor = self.doc[self.tab].cursor;
        let offset = self.doc[self.tab].offset;
        let graphemes = self.doc[self.tab].graphemes;
        // TODO: Update relavent lines here
        for _ in 0..self.config.general.tab_width {
            self.doc[self.tab].rows[cursor.y + offset.y - OFFSET].insert(' ', graphemes);
            self.move_cursor(Key::Right);
        }
    }
    fn return_key(&mut self) {
        // Return key
        self.doc[self.tab].dirty = true;
        self.doc[self.tab].show_welcome = false;
        let cursor = self.doc[self.tab].cursor;
        let offset = self.doc[self.tab].offset;
        // TODO: Update relavent lines here
        if cursor.x + offset.x == 0 {
            // Return key pressed at the start of the line
            self.doc[self.tab]
                .rows
                .insert(cursor.y + offset.y - OFFSET, Row::from(""));
            self.doc[self.tab]
                .undo_stack
                .push(Event::ReturnStart(Position {
                    x: cursor.x + offset.x,
                    y: cursor.y + offset.y - OFFSET,
                }));
            self.move_cursor(Key::Down);
        } else if cursor.x + self.doc[self.tab].offset.x
            == self.doc[self.tab].rows[cursor.y + offset.y - OFFSET].length()
        {
            // Return key pressed at the end of the line
            self.doc[self.tab]
                .rows
                .insert(cursor.y + offset.y + 1 - OFFSET, Row::from(""));
            self.doc[self.tab]
                .undo_stack
                .push(Event::ReturnEnd(Position {
                    x: cursor.x + offset.x,
                    y: cursor.y + offset.y - OFFSET,
                }));
            self.move_cursor(Key::Down);
            self.leap_cursor(Key::Home);
            self.recalculate_graphemes();
        } else {
            // Return key pressed in the middle of the line
            let current = self.doc[self.tab].rows
                [self.doc[self.tab].cursor.y + self.doc[self.tab].offset.y - OFFSET]
                .chars();
            let before = Row::from(&current[..self.doc[self.tab].graphemes].join("")[..]);
            let after = Row::from(&current[self.doc[self.tab].graphemes..].join("")[..]);
            self.doc[self.tab]
                .rows
                .insert(cursor.y + offset.y + 1 - OFFSET, after);
            self.doc[self.tab].rows[cursor.y + offset.y - OFFSET] = before.clone();
            self.doc[self.tab].undo_stack.push(Event::ReturnMid(
                Position {
                    x: cursor.x + offset.x,
                    y: cursor.y + offset.y - OFFSET,
                },
                before.length(),
            ));
            self.move_cursor(Key::Down);
            self.leap_cursor(Key::Home);
        }
        // Commit to undo stack when return key pressed
        self.doc[self.tab].undo_stack.commit();
    }
    fn backspace(&mut self) {
        // Handling the backspace key
        self.doc[self.tab].dirty = true;
        self.doc[self.tab].show_welcome = false;
        let cursor = self.doc[self.tab].cursor;
        let offset = self.doc[self.tab].offset;
        let graphemes = self.doc[self.tab].graphemes;
        // TODO: Update relavent lines here
        if self.doc[self.tab].cursor.x + self.doc[self.tab].offset.x == 0
            && self.doc[self.tab].cursor.y + self.doc[self.tab].offset.y - OFFSET != 0
        {
            // Backspace at the start of a line
            let current = self.doc[self.tab].rows
                [self.doc[self.tab].cursor.y + self.doc[self.tab].offset.y - OFFSET]
                .string
                .clone();
            let prev = self.doc[self.tab].rows
                [self.doc[self.tab].cursor.y + self.doc[self.tab].offset.y - 1 - OFFSET]
                .clone();
            self.doc[self.tab].rows[cursor.y + offset.y - 1 - OFFSET] =
                Row::from(&(prev.string.clone() + &current)[..]);
            self.doc[self.tab].rows.remove(cursor.y + offset.y - OFFSET);
            self.move_cursor(Key::Up);
            self.doc[self.tab].cursor.x = prev.length();
            self.recalculate_graphemes();
            self.doc[self.tab]
                .undo_stack
                .push(Event::BackspaceStart(Position {
                    x: cursor.x + offset.x,
                    y: cursor.y + offset.y - OFFSET,
                }));
            self.doc[self.tab].undo_stack.commit();
        } else {
            // Backspace in the middle of a line
            self.move_cursor(Key::Left);
            let ch = self.doc[self.tab].rows[cursor.y + offset.y - OFFSET].clone();
            self.doc[self.tab].rows[cursor.y + offset.y - OFFSET].delete(graphemes);
            if let Some(ch) = ch.chars().get(graphemes) {
                if let Ok(ch) = ch.parse() {
                    self.doc[self.tab].undo_stack.push(Event::BackspaceMid(
                        Position {
                            x: cursor.x + offset.x,
                            y: cursor.y + offset.y - OFFSET,
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
            if self.doc.len() <= 1 {
                // Quit Ox
                self.quit = true;
            } else if self.tab == self.doc.len().saturating_sub(1) {
                // Close current tab and move right
                self.doc.remove(self.tab);
                self.tab -= 1;
                self.set_command_line("Closed tab".to_string(), Type::Info);
            } else {
                // Close current tab and move left
                self.doc.remove(self.tab);
                self.set_command_line("Closed tab".to_string(), Type::Info);
            }
        }
    }
    fn new_document(&mut self) {
        // Handle new document event
        if self.dirty_prompt('n', "new") {
            self.doc.push(Document::new(&self.config));
            self.tab = self.doc.len().saturating_sub(1);
            self.doc[self.tab].dirty = false;
            self.doc[self.tab].show_welcome = true;
            self.doc[self.tab].cursor.y = OFFSET;
            self.doc[self.tab].offset.y = 0;
            self.leap_cursor(Key::Home);
        }
    }
    fn open_document(&mut self) {
        // Handle open document event
        // TODO: Highlight entire file here
        if let Some(result) = self.prompt("Open", &|_, _, _| {}) {
            if let Some(doc) = Document::open(&self.config, &result[..]) {
                // Overwrite the current document
                self.doc.push(doc);
                self.tab = self.doc.len().saturating_sub(1);
                self.doc[self.tab].dirty = false;
                self.doc[self.tab].show_welcome = false;
                self.doc[self.tab].cursor.y = OFFSET;
                self.doc[self.tab].offset.y = 0;
                self.leap_cursor(Key::Home);
            } else {
                self.set_command_line("File couldn't be opened".to_string(), Type::Error);
            }
        }
    }
    fn save(&mut self) {
        // Handle save event
        if self.doc[self.tab].save().is_ok() {
            // The document saved successfully
            self.doc[self.tab].dirty = false;
            self.set_command_line(
                format!("File saved to {} successfully", self.doc[self.tab].path),
                Type::Info,
            );
        } else {
            // The document couldn't save due to permission errors
            self.set_command_line(
                format!("Failed to save file to {}", self.doc[self.tab].path),
                Type::Error,
            );
        }
        // Commit to undo stack on document save
        self.doc[self.tab].undo_stack.commit();
    }
    fn save_as(&mut self) {
        // Handle save as event
        if let Some(result) = self.prompt("Save as", &|_, _, _| {}) {
            if self.doc[self.tab].save_as(&result[..]).is_ok() {
                // The document could save as
                self.doc[self.tab].dirty = false;
                self.set_command_line(format!("File saved to {} successfully", result), Type::Info);
                self.doc[self.tab].name = result.clone();
                self.doc[self.tab].path = result;
            } else {
                // The document couldn't save to the file
                self.set_command_line(format!("Failed to save file to {}", result), Type::Error);
            }
        } else {
            // User pressed the escape key
            self.set_command_line("Save as cancelled".to_string(), Type::Info);
        }
        // Commit to the undo stack on save as
        self.doc[self.tab].undo_stack.commit();
    }
    fn search(&mut self) {
        // For searching the file
        let initial_cursor = self.doc[self.tab].cursor;
        let initial_offset = self.doc[self.tab].offset;
        // Ask for a search term after saving the current cursor position
        self.prompt("Search", &|s, e, t| {
            // Find all occurances in the document
            let search_points = s.doc[s.tab].scan(t, OFFSET);
            let cursor = s.doc[s.tab].cursor;
            let offset = s.doc[s.tab].offset;
            match e {
                PromptEvent::KeyPress(k) => match k {
                    Key::Left | Key::Up => {
                        // User wants to search backwards
                        for p in search_points.iter().rev() {
                            if is_behind(
                                &Position {
                                    x: cursor.x + offset.x,
                                    y: cursor.y + offset.y,
                                },
                                &p,
                            ) {
                                s.goto(&p);
                                s.recalculate_graphemes();
                                break;
                            }
                        }
                    }
                    Key::Right | Key::Down => {
                        // User wants to search forwards
                        for p in search_points {
                            if is_ahead(
                                &Position {
                                    x: cursor.x + offset.x,
                                    y: cursor.y + offset.y,
                                },
                                &p,
                            ) {
                                s.goto(&p);
                                s.recalculate_graphemes();
                                break;
                            }
                        }
                    }
                    Key::Esc => {
                        // Restore cursor and offset position
                        s.doc[s.tab].cursor = initial_cursor;
                        s.doc[s.tab].offset = initial_offset;
                        s.recalculate_graphemes();
                    }
                    _ => (),
                },
                PromptEvent::CharPress => {
                    // When the user is typing the search query
                    s.doc[s.tab].cursor = initial_cursor;
                    s.doc[s.tab].offset = initial_offset;
                    if t != "" {
                        // Search forward as the user searches
                        for p in search_points {
                            if is_ahead(
                                &Position {
                                    x: cursor.x + offset.x,
                                    y: cursor.y + offset.y,
                                },
                                &p,
                            ) {
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
        // User cancelled or found what they were looking for
        self.set_command_line("Search exited".to_string(), Type::Info);
    }
    fn replace(&mut self) {
        // Replace text within the document
        let initial_cursor = self.doc[self.tab].cursor;
        let initial_offset = self.doc[self.tab].offset;
        // After saving the cursor position, ask the user for the information
        if let Some(target) = self.prompt("Replace", &|_, _, _| {}) {
            if let Some(arrow) = self.prompt("With", &|_, _, _| {}) {
                // Construct a regular expression for searching
                let re = Regex::new(&target).unwrap();
                let mut search_points = self.doc[self.tab].scan(&target, OFFSET);
                // Search forward as the user types
                for p in &search_points {
                    if is_ahead(
                        &Position {
                            x: self.doc[self.tab].cursor.x + self.doc[self.tab].offset.x,
                            y: self.doc[self.tab].cursor.y + self.doc[self.tab].offset.y,
                        },
                        &p,
                    ) {
                        self.goto(&p);
                        self.recalculate_graphemes();
                        self.update();
                        break;
                    }
                }
                loop {
                    // Handle key press events while in replace mode
                    let key = self.read_key();
                    match key {
                        Key::Up | Key::Left => {
                            // User wishes to search backwards
                            for p in (&search_points).iter().rev() {
                                if is_behind(
                                    &Position {
                                        x: self.doc[self.tab].cursor.x
                                            + self.doc[self.tab].offset.x,
                                        y: self.doc[self.tab].cursor.y
                                            + self.doc[self.tab].offset.y,
                                    },
                                    &p,
                                ) {
                                    self.goto(&p);
                                    self.recalculate_graphemes();
                                    self.update();
                                    break;
                                }
                            }
                        }
                        Key::Down | Key::Right => {
                            // User wishes to search forwards
                            for p in &search_points {
                                if is_ahead(
                                    &Position {
                                        x: self.doc[self.tab].cursor.x
                                            + self.doc[self.tab].offset.x,
                                        y: self.doc[self.tab].cursor.y
                                            + self.doc[self.tab].offset.y
                                            - OFFSET,
                                    },
                                    &p,
                                ) {
                                    self.goto(&p);
                                    self.recalculate_graphemes();
                                    self.update();
                                    break;
                                }
                            }
                        }
                        Key::Char('\n') | Key::Char('y') | Key::Char(' ') => {
                            let cursor = self.doc[self.tab].cursor;
                            let offset = self.doc[self.tab].offset;
                            // Commit current changes to undo stack
                            self.doc[self.tab].undo_stack.commit();
                            // Calculate the new line after the replacement
                            let line = self.doc[self.tab].rows[self.doc[self.tab].cursor.y
                                + self.doc[self.tab].offset.y
                                - OFFSET]
                                .clone();
                            let before = self.doc[self.tab].rows[self.doc[self.tab].cursor.y
                                + self.doc[self.tab].offset.y
                                - OFFSET]
                                .clone();
                            let after = Row::from(&*re.replace_all(&line.string[..], &arrow[..]));
                            // Check there was actually a change
                            if before.string != after.string {
                                // Push the replace event to the undo stack
                                self.doc[self.tab].undo_stack.push(Event::UpdateLine(
                                    cursor.y + offset.y - OFFSET,
                                    Box::new(before.clone()),
                                    Box::new(after.clone()),
                                ));
                                // TODO: Update relavent lines here
                                self.doc[self.tab].rows[cursor.y + offset.y - OFFSET] = after;
                            }
                            self.update();
                            self.snap_cursor();
                            self.prevent_unicode_hell();
                            self.recalculate_graphemes();
                            // Update search locations
                            search_points = self.doc[self.tab].scan(&target, OFFSET);
                        }
                        Key::Esc => break,
                        _ => (),
                    }
                }
                // Restore cursor position and exit
                self.doc[self.tab].cursor = initial_cursor;
                self.doc[self.tab].offset = initial_offset;
                self.set_command_line("Replace finished".to_string(), Type::Info);
            }
        }
    }
    fn replace_all(&mut self) {
        // Replace all occurances of a substring
        if let Some(target) = self.prompt("Replace", &|_, _, _| {}) {
            if let Some(arrow) = self.prompt("With", &|_, _, _| {}) {
                // Commit undo stack changes
                self.doc[self.tab].undo_stack.commit();
                let re = Regex::new(&target).unwrap();
                let lines = self.doc[self.tab].rows.clone();
                // Replace every occurance
                for (c, line) in lines.iter().enumerate() {
                    let before = self.doc[self.tab].rows[c].clone();
                    let after = Row::from(&*re.replace_all(&line.string[..], &arrow[..]));
                    if before.string != after.string {
                        self.doc[self.tab].undo_stack.push(Event::UpdateLine(
                            c,
                            Box::new(before.clone()),
                            Box::new(after.clone()),
                        ));
                        // TODO: Update relavent lines here
                        self.doc[self.tab].rows[c] = after;
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
        if self.doc[self.tab].dirty {
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
                    // Update the prompt contents
                    if c == '\n' {
                        // Exit on enter key
                        break 'p;
                    } else {
                        result.push(c);
                    }
                    func(self, PromptEvent::CharPress, &result)
                }
                Key::Backspace => {
                    // Handle backspace event
                    result.pop();
                    func(self, PromptEvent::CharPress, &result)
                }
                Key::Esc => {
                    // Handle escape key
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
        // Handle large cursor movements
        match action {
            Key::PageUp => {
                // Move cursor to the top of the screen
                self.doc[self.tab].cursor.y = OFFSET;
                self.snap_cursor();
                self.prevent_unicode_hell();
                self.recalculate_graphemes();
            }
            Key::PageDown => {
                // Move cursor to the bottom of the screen
                self.doc[self.tab].cursor.y = cmp::min(
                    self.doc[self.tab].rows.len().saturating_sub(1),
                    self.term.height.saturating_sub(3) as usize,
                );
                self.snap_cursor();
                self.prevent_unicode_hell();
                self.recalculate_graphemes();
            }
            Key::Home => {
                // Move cursor to the start of the line
                self.doc[self.tab].offset.x = 0;
                self.doc[self.tab].cursor.x = 0;
                self.doc[self.tab].graphemes = 0;
            }
            Key::End => {
                // Move cursor to the end of the line
                let cursor = self.doc[self.tab].cursor;
                let offset = self.doc[self.tab].offset;
                let line = self.doc[self.tab].rows[cursor.y + offset.y - OFFSET].clone();
                if line.length()
                    >= self
                        .term
                        .width
                        .saturating_sub(self.doc[self.tab].line_offset as u16)
                        as usize
                {
                    // Work out the width of the character to traverse
                    let mut jump = 1;
                    if let Some(chr) = line.ext_chars().get(line.length()) {
                        jump = UnicodeWidthStr::width(*chr);
                    }
                    self.doc[self.tab].offset.x = line
                        .length()
                        .saturating_add(jump + self.doc[self.tab].line_offset + 1)
                        .saturating_sub(self.term.width as usize);
                    self.doc[self.tab].cursor.x = self
                        .term
                        .width
                        .saturating_sub((jump + self.doc[self.tab].line_offset + 1) as u16)
                        as usize;
                } else {
                    self.doc[self.tab].cursor.x = line.length();
                }
                self.doc[self.tab].graphemes = line.chars().len();
            }
            _ => (),
        }
    }
    fn move_cursor(&mut self, direction: Key) {
        // Move the cursor around the editor
        match direction {
            Key::Down => {
                // Move the cursor down
                if self.doc[self.tab].cursor.y + self.doc[self.tab].offset.y + 1 - (OFFSET)
                    < self.doc[self.tab].rows.len()
                {
                    // If the proposed move is within the length of the document
                    if self.doc[self.tab].cursor.y == self.term.height.saturating_sub(3) as usize {
                        self.doc[self.tab].offset.y = self.doc[self.tab].offset.y.saturating_add(1);
                    } else {
                        self.doc[self.tab].cursor.y = self.doc[self.tab].cursor.y.saturating_add(1);
                    }
                    self.snap_cursor();
                    self.prevent_unicode_hell();
                    self.recalculate_graphemes();
                }
            }
            Key::Up => {
                // Move the cursor up
                if self.doc[self.tab].cursor.y - OFFSET == 0 {
                    self.doc[self.tab].offset.y = self.doc[self.tab].offset.y.saturating_sub(1);
                } else if self.doc[self.tab].cursor.y != OFFSET {
                    self.doc[self.tab].cursor.y = self.doc[self.tab].cursor.y.saturating_sub(1);
                }
                self.snap_cursor();
                self.prevent_unicode_hell();
                self.recalculate_graphemes();
            }
            Key::Right => {
                // Move the cursor right
                let line = &self.doc[self.tab].rows
                    [self.doc[self.tab].cursor.y + self.doc[self.tab].offset.y - OFFSET];
                // Work out the width of the character to traverse
                let mut jump = 1;
                if let Some(chr) = line
                    .ext_chars()
                    .get(self.doc[self.tab].cursor.x + self.doc[self.tab].offset.x)
                {
                    jump = UnicodeWidthStr::width(*chr);
                }
                // Check the proposed move is within the current line length
                if line.length() > self.doc[self.tab].cursor.x + self.doc[self.tab].offset.x {
                    // Check for normal width character
                    let indicator1 = self.doc[self.tab].cursor.x
                        == self
                            .term
                            .width
                            .saturating_sub((self.doc[self.tab].line_offset + jump + 1) as u16)
                            as usize;
                    // Check for half broken unicode character
                    let indicator2 = self.doc[self.tab].cursor.x
                        == self
                            .term
                            .width
                            .saturating_sub((self.doc[self.tab].line_offset + jump) as u16)
                            as usize;
                    if indicator1 || indicator2 {
                        self.doc[self.tab].offset.x =
                            self.doc[self.tab].offset.x.saturating_add(jump);
                    } else {
                        self.doc[self.tab].cursor.x =
                            self.doc[self.tab].cursor.x.saturating_add(jump);
                    }
                    self.doc[self.tab].graphemes = self.doc[self.tab].graphemes.saturating_add(1);
                }
            }
            Key::Left => {
                // Move the cursor left
                let line = &self.doc[self.tab].rows
                    [self.doc[self.tab].cursor.y + self.doc[self.tab].offset.y - OFFSET];
                // Work out the width of the character to traverse
                let mut jump = 1;
                if let Some(chr) = line.ext_chars().get(
                    (self.doc[self.tab].cursor.x + self.doc[self.tab].offset.x).saturating_sub(1),
                ) {
                    jump = UnicodeWidthStr::width(*chr);
                }
                if self.doc[self.tab].cursor.x == 0 {
                    self.doc[self.tab].offset.x = self.doc[self.tab].offset.x.saturating_sub(jump);
                } else {
                    self.doc[self.tab].cursor.x = self.doc[self.tab].cursor.x.saturating_sub(jump);
                }
                self.doc[self.tab].graphemes = self.doc[self.tab].graphemes.saturating_sub(1);
            }
            _ => (),
        }
    }
    fn snap_cursor(&mut self) {
        // Snap the cursor to the end of the row when outside
        let current = self.doc[self.tab].rows
            [self.doc[self.tab].cursor.y + self.doc[self.tab].offset.y - OFFSET]
            .clone();
        if current.length() <= self.doc[self.tab].cursor.x + self.doc[self.tab].offset.x {
            // If the cursor is out of bounds
            self.leap_cursor(Key::Home);
            self.leap_cursor(Key::End);
        }
    }
    fn prevent_unicode_hell(&mut self) {
        // Make sure that the cursor isn't inbetween a unicode character
        let line = &self.doc[self.tab].rows
            [self.doc[self.tab].cursor.y + self.doc[self.tab].offset.y - OFFSET];
        if line.length() > self.doc[self.tab].cursor.x + self.doc[self.tab].offset.x {
            // As long as the cursor is within range
            let boundaries = line.boundaries();
            let mut index = self.doc[self.tab].cursor.x + self.doc[self.tab].offset.x;
            if !boundaries.contains(&index) && index != 0 {}
            while !boundaries.contains(&index) && index != 0 {
                self.doc[self.tab].cursor.x = self.doc[self.tab].cursor.x.saturating_sub(1);
                self.doc[self.tab].graphemes = self.doc[self.tab].graphemes.saturating_sub(1);
                index = index.saturating_sub(1);
            }
        }
    }
    fn recalculate_graphemes(&mut self) {
        // Recalculate the grapheme cursor after moving up and down
        let current = self.doc[self.tab].rows
            [self.doc[self.tab].cursor.y + self.doc[self.tab].offset.y - OFFSET]
            .clone();
        let jumps = current.get_jumps();
        let mut counter = 0;
        for (mut counter2, i) in jumps.into_iter().enumerate() {
            if counter == self.doc[self.tab].cursor.x + self.doc[self.tab].offset.x {
                break;
            }
            counter2 += 1;
            self.doc[self.tab].graphemes = counter2;
            counter += i;
        }
    }
    fn goto(&mut self, pos: &Position) {
        // Move the cursor to a specific location
        let max_y = self.term.height.saturating_sub(3) as usize;
        let max_x = (self.term.width as usize).saturating_sub(self.doc[self.tab].line_offset);
        let halfway_y = max_y / 2;
        let halfway_x = max_x / 2;
        if self.doc[self.tab].offset.x == 0 && pos.y < max_y && pos.x < max_x {
            // Cursor is on the screen
            self.doc[self.tab].offset = Position { x: 0, y: 0 };
            self.doc[self.tab].cursor = *pos;
        } else {
            // Cursor is off the screen, move to the Y position
            self.doc[self.tab].offset.y = pos.y.saturating_sub(halfway_y);
            self.doc[self.tab].cursor.y = halfway_y;
            if self.doc[self.tab].offset.y + self.doc[self.tab].cursor.y != pos.y {
                // Fix cursor misplacement
                self.doc[self.tab].offset = Position { x: 0, y: 0 };
                self.doc[self.tab].cursor = *pos;
                return;
            }
            // Change the X
            if pos.x >= max_x {
                // Move to the center
                self.doc[self.tab].offset.x = pos.x.saturating_sub(halfway_x);
                self.doc[self.tab].cursor.x = halfway_x;
            } else {
                // No offset
                self.doc[self.tab].offset.x = 0;
                self.doc[self.tab].cursor.x = pos.x;
            }
        }
    }
    fn update(&mut self) {
        // Move the cursor and render the screen
        self.term.hide_cursor();
        self.term.goto(&Position { x: 0, y: 0 });
        self.doc[self.tab].recalculate_offset(&self.config);
        self.render();
        self.term.goto(&Position {
            x: self.doc[self.tab]
                .cursor
                .x
                .saturating_add(self.doc[self.tab].line_offset),
            y: self.doc[self.tab].cursor.y,
        });
        self.term.show_cursor();
        self.term.flush();
    }
    fn welcome_message(&self, text: &str, colour: color::Fg<color::Rgb>) -> String {
        // Render the welcome message
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
            self.doc[self.tab].name,
            if self.doc[self.tab].dirty {
                "[+] \u{fb12} "
            } else {
                " \u{f723} "
            },
            self.doc[self.tab].icon,
        );
        // Create the right part of the status line
        let right = format!(
            " \u{fa70} {} / {} \u{2502} \u{fae6}({}, {}) ",
            self.doc[self.tab].cursor.y + self.doc[self.tab].offset.y + 1 - OFFSET,
            self.doc[self.tab].rows.len(),
            self.doc[self.tab].cursor.x + self.doc[self.tab].offset.x,
            self.doc[self.tab].cursor.y + self.doc[self.tab].offset.y,
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
    fn tab_line(&mut self) -> String {
        // Render the tab line
        let mut result = String::new();
        let mut width = 0;
        // Iterate through documents
        for (num, doc) in self.doc.iter().enumerate() {
            // Calculate value for tab
            let mut name = doc.name.clone();
            let icons: Vec<&str> = doc.icon.graphemes(true).collect();
            if icons.len() > 2 {
                let icons = &icons.get(icons.len() - 2..).unwrap_or_default().join("");
                name = format!("{} {}", icons, doc.name);
            }
            let this;
            if num == self.tab && !self.doc.len() == num {
                // Render inactive tabs
                this = format!("{} {} |", Reader::rgb_bg(self.config.theme.editor_bg), name);
            } else if num == self.tab {
                // Render active tab
                this = format!(
                    "{}{} {} {}{}|",
                    Reader::rgb_bg(self.config.theme.editor_bg),
                    style::Bold,
                    name,
                    style::Reset,
                    Reader::rgb_bg(self.config.theme.status_bg)
                );
            } else if num.saturating_sub(1) == self.tab {
                this = format!("{} {} |", Reader::rgb_bg(self.config.theme.status_bg), name);
            } else {
                this = format!(" {} |", name);
            }
            // Check if tab will fit in window width, otherwise put in a "..."
            width += self.exp.ansi_len(&this);
            if width + 3 > self.term.width as usize {
                result += &"...";
                break;
            } else {
                result += &this;
            }
        }
        format!(
            "{}{}{}{}",
            Reader::rgb_bg(self.config.theme.status_bg),
            result,
            self.term.align_left(&result),
            RESET_BG,
        )
    }
    fn render(&mut self) {
        // Draw the screen to the terminal
        let offset = self.doc[self.tab].offset;
        let mut frame = vec![self.tab_line()];
        let rendered = self.doc[self.tab].render();
        let reg = self.doc[self.tab].regex.clone();
        for row in (OFFSET as u16)..self.term.height {
            let row = row.saturating_sub(OFFSET as u16);
            if let Some(r) = self.doc[self.tab].rows.get_mut(offset.y + row as usize) {
                r.update_syntax(&self.config, &reg, &rendered, offset.y + row as usize);
            }
            if row == self.term.height - 1 - OFFSET as u16 {
                // Render command line
                frame.push(self.command_line());
            } else if row == self.term.height - 2 - OFFSET as u16 {
                // Render status line
                frame.push(self.status_line());
            } else if row == self.term.height / 4 - OFFSET as u16 && self.doc[self.tab].show_welcome
            {
                frame.push(self.welcome_message(
                    &format!("Ox editor  v{}", VERSION),
                    Reader::rgb_fg(self.config.theme.editor_fg),
                ));
            } else if row == (self.term.height / 4).saturating_add(1) - OFFSET as u16
                && self.doc[self.tab].show_welcome
            {
                frame.push(self.welcome_message(
                    "A Rust powered editor by Luke",
                    Reader::rgb_fg(self.config.theme.editor_fg),
                ));
            } else if row == (self.term.height / 4).saturating_add(3) - OFFSET as u16
                && self.doc[self.tab].show_welcome
            {
                frame.push(self.welcome_message(
                    "Ctrl + Q: Exit   ",
                    Reader::rgb_fg(self.config.theme.status_fg),
                ));
            } else if row == (self.term.height / 4).saturating_add(4) - OFFSET as u16
                && self.doc[self.tab].show_welcome
            {
                frame.push(self.welcome_message(
                    "Ctrl + S: Save   ",
                    Reader::rgb_fg(self.config.theme.status_fg),
                ));
            } else if row == (self.term.height / 4).saturating_add(5) - OFFSET as u16
                && self.doc[self.tab].show_welcome
            {
                frame.push(self.welcome_message(
                    "Ctrl + W: Save as",
                    Reader::rgb_fg(self.config.theme.status_fg),
                ));
            } else if let Some(line) = self.doc[self.tab]
                .rows
                .get(self.doc[self.tab].offset.y + row as usize)
            {
                // Render lines of code
                frame.push(self.add_background(&line.render(
                    self.doc[self.tab].offset.x,
                    self.term.width as usize,
                    self.doc[self.tab].offset.y + row as usize,
                    self.doc[self.tab].line_offset,
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
