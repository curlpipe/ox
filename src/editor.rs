// Editor.rs - Controls the editor and brings everything together
use crate::config::{KeyBinding, RawKey, Reader, Status, Theme};
use crate::document::{TabType, Type};
use crate::highlight::Token;
use crate::oxa::interpret_line;
use crate::undo::{reverse, BankType};
use crate::util::{title, trim_end, Exp};
use crate::{log, Document, Event, Row, Size, Terminal, VERSION};
use clap::App;
use crossterm::event::{Event as InputEvent, KeyCode, KeyEvent, KeyModifiers};
use crossterm::style::{Attribute, Color, SetBackgroundColor, SetForegroundColor};
use crossterm::ErrorKind;
use regex::Regex;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Error, ErrorKind as Iek, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use unicode_width::UnicodeWidthStr;

// Set up color resets
pub const RESET_BG: SetBackgroundColor = SetBackgroundColor(Color::Reset);
pub const RESET_FG: SetForegroundColor = SetForegroundColor(Color::Reset);

// Set up offset rules
pub const OFFSET: usize = 1;

// Macro for running shell commands within the editor
macro_rules! shell {
    ($command:expr, $confirm:expr, $root:expr) => {
        // Execute a shell command
        let command = if $root {
            Command::new("sudo")
                .arg("bash")
                .arg("-c")
                .arg($command)
                .stdout(Stdio::piped())
                .spawn()
        } else {
            Command::new("bash")
                .arg("-c")
                .arg($command)
                .stdout(Stdio::piped())
                .spawn()
        };
        if let Ok(s) = command {
            log!("Shell", "Command requested");
            if let Ok(s) = s
                .stdout
                .ok_or_else(|| Error::new(Iek::Other, "Could not capture standard output."))
            {
                // Go back into canonical mode to restore normal operation
                Terminal::exit();
                log!("Shell", "Ready to go");
                // Stream the input and output of the command to the current stdout
                BufReader::new(s)
                    .lines()
                    .filter_map(std::result::Result::ok)
                    .for_each(|line| println!("{}", line));
                // Wait for user to press enter, then reenter raw mode
                log!("Shell", "Exited");
                if $confirm {
                    println!("Shell command exited. Press [Return] to continue");
                    let mut output = String::new();
                    let _ = std::io::stdin().read_line(&mut output);
                }
                Terminal::enter();
            } else {
                log!("Failure to open standard output", "");
            }
        } else {
            log!(
                "Failure to run command",
                format!(
                    "{} {:?}",
                    $command,
                    Command::new($command).stdout(Stdio::piped()).spawn()
                )
            );
        }
    };
}

// Enum for holding prompt events
enum PromptEvent {
    Update,
    CharPress(bool),
    KeyPress(KeyCode),
}

// For representing positions
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Position {
    pub x: usize,
    pub y: usize,
}

// Enum for direction
#[derive(Clone, Copy, Debug)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

// The main editor struct
pub struct Editor {
    pub config: Reader,                      // Storage for configuration
    pub status: Status,                      // Holding the status of the config
    config_path: String,                     // Holds the file path of the config file
    quit: bool,                              // Toggle for cleanly quitting the editor
    term: Terminal,                          // For the handling of the terminal
    doc: Vec<Document>,                      // For holding our document
    tab: usize,                              // Holds the number of the current tab
    last_keypress: Option<Instant>,          // For holding the time of the last input event
    keypress: KeyBinding,                    // For holding the last keypress event
    exp: Exp,                                // For holding expressions
    position_bank: HashMap<usize, Position>, // Bank for cursor positions
    row_bank: HashMap<usize, Row>,           // Bank for lines
    theme: String,                           // Currently used theme
}

// Implementing methods for our editor struct / class
impl Editor {
    pub fn new(args: App) -> Result<Self, ErrorKind> {
        // Create a new editor instance
        let args = args.get_matches();
        // Set up terminal
        let term = Terminal::new()?;
        // Set up the arguments
        let files: Vec<&str> = args.values_of("files").unwrap_or_default().collect();
        let config_path = args.value_of("config").unwrap_or_default();
        let mut config = Reader::read(config_path);
        // Check for fallback colours
        if config.0.theme.fallback {
            let max = Terminal::availablility();
            log!("Available Colours", max);
            if max != 24 {
                // Fallback to 16 bit colours
                config.0.highlights.insert(
                    "16fallback".to_string(),
                    [
                        ("comments".to_string(), (128, 128, 128)),
                        ("keywords".to_string(), (0, 0, 255)),
                        ("namespaces".to_string(), (0, 0, 255)),
                        ("references".to_string(), (0, 0, 128)),
                        ("strings".to_string(), (0, 128, 0)),
                        ("characters".to_string(), (0, 128, 128)),
                        ("digits".to_string(), (0, 128, 128)),
                        ("booleans".to_string(), (0, 255, 0)),
                        ("functions".to_string(), (0, 128, 128)),
                        ("structs".to_string(), (0, 128, 128)),
                        ("macros".to_string(), (128, 0, 128)),
                        ("attributes".to_string(), (0, 128, 128)),
                        ("headers".to_string(), (0, 128, 128)),
                        ("symbols".to_string(), (128, 128, 0)),
                        ("global".to_string(), (0, 255, 0)),
                        ("operators".to_string(), (0, 128, 128)),
                        ("regex".to_string(), (0, 255, 0)),
                        ("search_inactive".to_string(), (128, 128, 128)),
                        ("search_active".to_string(), (0, 128, 128)),
                    ]
                    .iter()
                    .cloned()
                    .collect(),
                );
                config.0.theme = Theme {
                    transparent_editor: false,
                    editor_bg: (0, 0, 0),
                    editor_fg: (255, 255, 255),
                    status_bg: (128, 128, 128),
                    status_fg: (255, 255, 255),
                    line_number_fg: (255, 255, 255),
                    line_number_bg: (0, 0, 0),
                    active_tab_fg: (255, 255, 255),
                    inactive_tab_fg: (255, 255, 255),
                    active_tab_bg: (128, 128, 128),
                    inactive_tab_bg: (0, 0, 0),
                    warning_fg: (255, 255, 0),
                    error_fg: (255, 0, 0),
                    info_fg: (255, 255, 255),
                    default_theme: "16fallback".to_string(),
                    fallback: true,
                };
            }
        }
        // Read in documents
        let mut documents = vec![];
        if files.is_empty() {
            documents.push(Document::new(
                &config.0,
                &config.1,
                args.is_present("readonly"),
            ));
        } else {
            for file in &files {
                documents.push(Document::from(
                    &config.0,
                    &config.1,
                    file,
                    args.is_present("readonly"),
                ));
            }
        }
        // Calculate neater paths
        for d in &mut documents {
            d.correct_path(&term.size);
        }
        // Create the new editor instance
        Ok(Self {
            quit: false,
            // Display information about the config file into text for the status line
            term,
            tab: 0,
            doc: documents,
            last_keypress: None,
            keypress: KeyBinding::Unsupported,
            config: config.0.clone(),
            config_path: config_path.to_string(),
            status: config.1,
            exp: Exp::new(),
            position_bank: HashMap::new(),
            row_bank: HashMap::new(),
            theme: config.0.theme.default_theme,
        })
    }
    pub fn run(&mut self) {
        // Run the editor instance
        log!("Ox opened", "Ox was opened successfully");
        while !self.quit {
            self.update();
            self.process_input();
        }
        // Leave alternative screen and disable raw mode
        Terminal::exit();
    }
    fn read_event(&mut self) -> InputEvent {
        // Wait until a key, mouse or terminal resize event
        loop {
            if let Ok(true) = crossterm::event::poll(Duration::from_millis(16)) {
                if let Ok(key) = crossterm::event::read() {
                    // When a keypress was detected
                    self.last_keypress = Some(Instant::now());
                    return key;
                }
            } else {
                // Check for a period of inactivity
                if let Some(time) = self.last_keypress {
                    // Check to see if it's over the config undo period
                    if time.elapsed().as_secs() >= self.config.general.undo_period {
                        // Commit the undo changes to the stack
                        self.doc[self.tab].undo_stack.commit();
                        self.last_keypress = None;
                    }
                }
            }
        }
    }
    fn key_event_to_ox_key(key: KeyCode, modifiers: KeyModifiers) -> KeyBinding {
        // Convert crossterm's complicated key structure into Ox's simpler one
        let inner = match key {
            KeyCode::Char(c) => RawKey::Char(c),
            KeyCode::BackTab => RawKey::BackTab,
            KeyCode::Insert => RawKey::Insert,
            KeyCode::Esc => RawKey::Esc,
            KeyCode::Backspace => RawKey::Backspace,
            KeyCode::Tab => RawKey::Tab,
            KeyCode::Enter => RawKey::Enter,
            KeyCode::Delete => RawKey::Delete,
            KeyCode::Null => RawKey::Null,
            KeyCode::PageUp => RawKey::PageUp,
            KeyCode::PageDown => RawKey::PageDown,
            KeyCode::Home => RawKey::Home,
            KeyCode::End => RawKey::End,
            KeyCode::Up => RawKey::Up,
            KeyCode::Down => RawKey::Down,
            KeyCode::Left => RawKey::Left,
            KeyCode::Right => RawKey::Right,
            KeyCode::F(i) => return KeyBinding::F(i),
        };
        match modifiers {
            KeyModifiers::CONTROL => KeyBinding::Ctrl(inner),
            KeyModifiers::ALT => KeyBinding::Alt(inner),
            KeyModifiers::SHIFT => KeyBinding::Shift(inner),
            KeyModifiers::NONE => KeyBinding::Raw(inner),
            _ => KeyBinding::Unsupported,
        }
    }
    fn process_key(&mut self, key: KeyEvent) {
        self.doc[self.tab].show_welcome = false;
        let cursor = self.doc[self.tab].cursor;
        let offset = self.doc[self.tab].offset;
        let current = Position {
            x: cursor.x + offset.x,
            y: cursor.y + offset.y - OFFSET,
        };
        let ox_key = Editor::key_event_to_ox_key(key.code, key.modifiers);
        self.keypress = ox_key;
        match ox_key {
            KeyBinding::Raw(RawKey::Enter) => {
                self.doc[self.tab].redo_stack.empty();
                if current.x == 0 {
                    // Return key pressed at the start of the line
                    self.execute(Event::InsertLineAbove(current), false);
                } else if current.x == self.doc[self.tab].rows[current.y].length() {
                    // Return key pressed at the end of the line
                    self.execute(Event::InsertLineBelow(current), false);
                    self.execute(Event::MoveCursor(1, Direction::Down), false);
                } else {
                    // Return key pressed in the middle of the line
                    self.execute(Event::SplitDown(current, current), false);
                }
            }
            KeyBinding::Raw(RawKey::Tab) => {
                self.doc[self.tab].redo_stack.empty();
                self.execute(Event::InsertTab(current), false);
            }
            KeyBinding::Raw(RawKey::Backspace) => {
                self.doc[self.tab].redo_stack.empty();
                self.execute(
                    if current.x == 0 && current.y != 0 {
                        // Backspace at the start of a line
                        Event::SpliceUp(current, current)
                    } else if current.x == 0 {
                        return;
                    } else {
                        // Backspace in the middle of a line
                        let row = self.doc[self.tab].rows[current.y].clone();
                        let chr = row
                            .ext_chars()
                            .get(current.x.saturating_add(1))
                            .map_or(" ", |chr| *chr);
                        let current = Position {
                            x: current.x.saturating_sub(UnicodeWidthStr::width(chr)),
                            y: current.y,
                        };
                        Event::Deletion(current, chr.parse().unwrap_or(' '))
                    },
                    false,
                );
            }
            // Detect control and alt and function key bindings
            KeyBinding::Ctrl(_) | KeyBinding::Alt(_) | KeyBinding::F(_) => {
                if let Some(commands) = self.config.keys.get(&ox_key) {
                    for i in commands.clone() {
                        self.text_to_event(&i);
                    }
                }
            }
            KeyBinding::Raw(RawKey::Char(c)) | KeyBinding::Shift(RawKey::Char(c)) => {
                self.doc[self.tab].redo_stack.empty();
                self.execute(Event::Insertion(current, c), false);
            }
            KeyBinding::Raw(RawKey::Up) => self.execute(Event::MoveCursor(1, Direction::Up), false),
            KeyBinding::Raw(RawKey::Down) => {
                self.execute(Event::MoveCursor(1, Direction::Down), false)
            }
            KeyBinding::Raw(RawKey::Left) => {
                self.execute(Event::MoveCursor(1, Direction::Left), false)
            }
            KeyBinding::Raw(RawKey::Right) => {
                self.execute(Event::MoveCursor(1, Direction::Right), false)
            }
            KeyBinding::Raw(RawKey::PageDown) => self.execute(Event::PageDown, false),
            KeyBinding::Raw(RawKey::PageUp) => self.execute(Event::PageUp, false),
            KeyBinding::Raw(RawKey::Home) => self.execute(Event::Home, false),
            KeyBinding::Raw(RawKey::End) => self.execute(Event::End, false),
            _ => (),
        }
    }
    fn process_input(&mut self) {
        // Read a key and act on it
        match self.read_event() {
            InputEvent::Key(key) => self.process_key(key),
            InputEvent::Resize(width, height) => {
                // Terminal resize event
                self.term.size = Size {
                    width: width as usize,
                    height: height as usize,
                };
                // Move cursor if needed
                let size = self.term.size.height.saturating_sub(3);
                if self.doc[self.tab].cursor.y > size && size != 0 {
                    // Prevent cursor going off the screen and breaking everything
                    self.doc[self.tab].cursor.y = size;
                }
                // Re-render everything to the new size
                self.update();
            }
            InputEvent::Mouse(_) => (),
        }
    }
    fn new_document(&mut self) {
        // Create a new document
        self.doc
            .push(Document::new(&self.config, &self.status, false));
        self.tab = self.doc.len().saturating_sub(1);
    }
    fn open_document(&mut self, file: Option<String>) {
        // Open a document
        let to_open = if let Some(path) = file {
            // File was specified
            path
        } else if let Some(path) = self.prompt("Open", ": ", &|_, _, _| {}) {
            // Ask for a file and open it
            path
        } else {
            // User cancelled
            return;
        };
        if let Some(doc) = Document::open(&self.config, &self.status, &to_open, false) {
            // Overwrite the current document
            self.doc.push(doc);
            self.tab = self.doc.len().saturating_sub(1);
        } else {
            self.doc[self.tab].set_command_line("File couldn't be opened".to_string(), Type::Error);
        }
    }
    fn save_document(&mut self, file: Option<String>, prompt: bool) {
        // Save the document
        let save = if let Some(file) = file {
            // File was specified
            file
        } else {
            // File not specified
            if prompt {
                // Save as
                if let Some(path) = self.prompt("Save as", ": ", &|_, _, _| {}) {
                    path
                } else {
                    // User cancelled
                    return;
                }
            } else {
                // Use current document
                self.doc[self.tab].path.clone()
            }
        };
        if self.doc[self.tab].path != save && Path::new(&save).exists() {
            // File already exists, possible loss of data
            self.doc[self.tab]
                .set_command_line(format!("File {} already exists", save), Type::Error);
            return;
        }
        // Attempt document save
        let tab_width = self.config.general.tab_width;
        if self.doc[self.tab].save(&save, tab_width).is_ok() {
            // The document saved successfully
            let ext = save.split('.').last().unwrap_or(&"");
            self.doc[self.tab].dirty = false;
            self.doc[self.tab].set_command_line(
                format!("File saved to \"{}\" successfully", save),
                Type::Info,
            );
            // Update the current documents details in case of filetype change
            self.doc[self.tab].last_save_index = self.doc[self.tab].undo_stack.len();
            self.doc[self.tab].kind = Document::identify(&save).0.to_string();
            self.doc[self.tab].icon = Document::identify(&save).1.to_string();
            self.doc[self.tab].name = Path::new(&save)
                .file_name()
                .unwrap_or_else(|| OsStr::new(&save))
                .to_str()
                .unwrap_or(&save)
                .to_string();
            self.doc[self.tab].path = save.clone();
            self.doc[self.tab].regex = Reader::get_syntax_regex(&self.config, ext);
        } else if save.is_empty() {
            // The document couldn't save due to an empty name
            self.doc[self.tab].set_command_line(
                "Filename is blank, please specify file name".to_string(),
                Type::Error,
            );
        } else {
            // The document couldn't save due to permission errors / invalid name
            self.doc[self.tab]
                .set_command_line(format!("Failed to save file to \"{}\"", save), Type::Error);
        }
        // Commit to undo stack on document save
        self.execute(Event::Commit, false);
    }
    fn save_every_document(&mut self) {
        // Save every document in the editor
        let tab_width = self.config.general.tab_width;
        let mut successes = 0;
        let mut failiures = 0;
        for i in 0..self.doc.len() {
            let path = self.doc[i].path.clone();
            if self.doc[i].save(&path, tab_width).is_ok() {
                // The document saved successfully
                self.doc[i].dirty = false;
                successes += 1;
            } else {
                // The document couldn't save due to permission errors
                failiures += 1;
            }
            self.doc[i].set_command_line(
                format!("Saved {} documents, {} failed", successes, failiures),
                Type::Info,
            );
            // Commit to undo stack on document save
            self.execute(Event::Commit, false);
        }
    }
    fn quit_document(&mut self, force: bool) {
        // For handling a quit event
        if let KeyBinding::Ctrl(_) | KeyBinding::Alt(_) = self.keypress {
            if force || self.dirty_prompt(self.keypress, "quit") {
                if self.doc.len() <= 1 {
                    // Quit Ox
                    self.quit = true;
                    return;
                } else if self.tab == self.doc.len().saturating_sub(1) {
                    // Close current tab and move right
                    self.doc.remove(self.tab);
                    self.tab -= 1;
                } else {
                    // Close current tab and move left
                    self.doc.remove(self.tab);
                }
                self.doc[self.tab].set_command_line("Closed tab".to_string(), Type::Info);
            }
        }
    }
    fn quit_all(&mut self, force: bool) {
        // Quit all the documents in the editor
        self.tab = 0;
        while !self.quit {
            self.execute(Event::Quit(force), false);
        }
    }
    fn next_tab(&mut self) {
        // Move to the next tab
        if self.tab.saturating_add(1) < self.doc.len() {
            self.tab = self.tab.saturating_add(1);
        }
    }
    fn prev_tab(&mut self) {
        // Move to the previous tab
        self.tab = self.tab.saturating_sub(1);
    }
    pub fn shell(&mut self, mut command: String, substitution: bool, root: bool, confirm: bool) {
        if substitution {
            let file =
                self.doc[self.tab].render(self.doc[self.tab].tabs, self.config.general.tab_width);
            command = command.replacen("%F", &self.doc[self.tab].path, 1);
            command = command.replacen("%C", &file, 1);
        }
        shell!(&command, confirm, root);
    }
    pub fn execute(&mut self, event: Event, reversed: bool) {
        // Event executor
        if self.doc[self.tab].read_only && Editor::will_edit(&event) {
            return;
        }
        match event {
            Event::New => self.new_document(),
            Event::Open(file) => self.open_document(file),
            Event::Save(file, prompt) => self.save_document(file, prompt),
            Event::SaveAll => self.save_every_document(),
            Event::Quit(force) => self.quit_document(force),
            Event::QuitAll(force) => self.quit_all(force),
            Event::NextTab => self.next_tab(),
            Event::PrevTab => self.prev_tab(),
            Event::Search => self.search(),
            Event::Replace => self.replace(),
            Event::ReplaceAll => self.replace_all(),
            Event::Cmd => self.cmd(),
            Event::Shell(command, confirm, substitution, root) => {
                self.shell(command, confirm, substitution, root)
            }
            Event::ReloadConfig => {
                let config = Reader::read(&self.config_path);
                self.config = config.0;
                self.doc[self.tab].cmd_line = Document::config_to_commandline(&config.1);
            }
            Event::Theme(name) => {
                self.theme = name;
                self.doc[self.tab].mass_redraw();
                self.update();
            }
            Event::MoveWord(direction) => match direction {
                Direction::Left => self.doc[self.tab].word_left(&self.term.size),
                Direction::Right => self.doc[self.tab].word_right(&self.term.size),
                _ => {},
            },
            Event::GotoCursor(pos) => {
                let rows = &self.doc[self.tab].rows;
                if rows.len() > pos.y && rows[pos.y].length() >= pos.x {
                    self.doc[self.tab].goto(pos, &self.term.size);
                }
            }
            Event::MoveCursor(magnitude, direction) => {
                for _ in 0..magnitude {
                    self.doc[self.tab].move_cursor(
                        match direction {
                            Direction::Up => KeyCode::Up,
                            Direction::Down => KeyCode::Down,
                            Direction::Left => KeyCode::Left,
                            Direction::Right => KeyCode::Right,
                        },
                        &self.term.size,
                        self.config.general.wrap_cursor,
                    );
                }
            }
            Event::Commit => self.doc[self.tab].undo_stack.commit(),
            Event::Store(kind, bank) => {
                let cursor = self.doc[self.tab].cursor;
                let offset = self.doc[self.tab].offset;
                let current = Position {
                    x: cursor.x + offset.x,
                    y: cursor.y + offset.y - OFFSET,
                };
                match kind {
                    BankType::Cursor => {
                        self.position_bank.insert(bank, current);
                    }
                    BankType::Line => {
                        self.row_bank
                            .insert(bank, self.doc[self.tab].rows[current.y].clone());
                    }
                }
            }
            Event::Load(kind, bank) => {
                let cursor = self.doc[self.tab].cursor;
                let offset = self.doc[self.tab].offset;
                let current = Position {
                    x: cursor.x + offset.x,
                    y: cursor.y + offset.y - OFFSET,
                };
                match kind {
                    BankType::Cursor => {
                        let cursor = *self.position_bank.get(&bank).unwrap_or(&current);
                        self.doc[self.tab].goto(cursor, &self.term.size);
                    }
                    BankType::Line => {
                        if let Some(row) = self.row_bank.get(&bank) {
                            self.doc[self.tab].rows[current.y] = row.clone();
                        }
                    }
                }
            }
            Event::Home => self.doc[self.tab].leap_cursor(KeyCode::Home, &self.term.size),
            Event::End => self.doc[self.tab].leap_cursor(KeyCode::End, &self.term.size),
            Event::PageUp => self.doc[self.tab].leap_cursor(KeyCode::PageUp, &self.term.size),
            Event::PageDown => self.doc[self.tab].leap_cursor(KeyCode::PageDown, &self.term.size),
            Event::Undo => self.undo(),
            Event::Redo => self.redo(),
            // Event is a document event, send to current document
            _ => self.doc[self.tab].execute(event, reversed, &self.term.size, &self.config),
        }
        self.doc[self.tab].recalculate_graphemes();
    }
    fn cmd(&mut self) {
        // Recieve macro command
        if let Some(command) = self.prompt(":", "", &|s, e, _| {
            if let PromptEvent::KeyPress(KeyCode::Esc) = e {
                s.doc[s.tab].set_command_line("".to_string(), Type::Info);
            }
        }) {
            // Parse and Lex instruction
            for command in command.split('|') {
                self.text_to_event(command);
            }
        }
    }
    fn execute_macro(&mut self, command: &str) {
        // Work out number of times to execute it
        let mut command = command.to_string();
        let times = if let Ok(times) = command.split(' ').next().unwrap_or("").parse::<usize>() {
            command = command.split(' ').skip(1).collect::<Vec<_>>().join(" ");
            times
        } else {
            1
        };
        // Build and execute the macro
        for _ in 0..times {
            for i in self.config.macros[&command].clone() {
                self.text_to_event(&i);
            }
        }
    }
    fn text_to_event(&mut self, command: &str) {
        let command = command.trim_start_matches(' ');
        let mut cmd = command.split(' ');
        let actual_command;
        let times = if let Ok(repeat) = cmd.next().unwrap_or_default().parse::<usize>() {
            actual_command = cmd.collect::<Vec<_>>().join(" ");
            repeat
        } else {
            actual_command = command.to_string();
            1
        };
        for _ in 0..times {
            if self.config.macros.contains_key(&actual_command) {
                self.execute_macro(&actual_command);
            } else {
                let cursor = self.doc[self.tab].cursor;
                let offset = self.doc[self.tab].offset;
                let instruction = interpret_line(
                    &actual_command,
                    &Position {
                        x: cursor.x + offset.x,
                        y: cursor.y + offset.y - OFFSET,
                    },
                    self.doc[self.tab].graphemes,
                    &self.doc[self.tab].rows,
                );
                // Execute the instruction
                if let Some(instruct) = instruction {
                    for i in instruct {
                        if Editor::will_edit(&i) {
                            self.doc[self.tab].redo_stack.empty();
                        }
                        self.execute(i, false);
                    }
                    self.doc[self.tab].undo_stack.commit();
                }
            }
        }
    }
    pub fn will_edit(event: &Event) -> bool {
        matches!(event, Event::SpliceUp(_, _)
            | Event::SplitDown(_, _)
            | Event::InsertLineAbove(_)
            | Event::InsertLineBelow(_)
            | Event::Deletion(_, _)
            | Event::Insertion(_, _)
            | Event::InsertTab(_)
            | Event::DeleteTab(_)
            | Event::DeleteLine(_, _, _)
            | Event::UpdateLine(_, _, _, _)
            | Event::ReplaceAll
            | Event::Replace
            | Event::Overwrite(_, _))
    }
    pub fn undo(&mut self) {
        self.doc[self.tab].undo_stack.commit();
        if let Some(events) = self.doc[self.tab].undo_stack.pop() {
            for event in events.clone() {
                if let Some(reversed) = reverse(event, self.doc[self.tab].rows.len()) {
                    for i in reversed {
                        self.execute(i, true);
                    }
                    self.update();
                }
            }
            self.doc[self.tab]
                .redo_stack
                .append(events.into_iter().rev().collect());
        } else {
            self.doc[self.tab].set_command_line("Empty Undo Stack".to_string(), Type::Error);
        }
        if self.doc[self.tab].undo_stack.len() == self.doc[self.tab].last_save_index {
            self.doc[self.tab].dirty = false;
        }
    }
    pub fn redo(&mut self) {
        if let Some(events) = self.doc[self.tab].redo_stack.pop() {
            for event in events {
                self.execute(event, false);
                self.update();
            }
        } else {
            self.doc[self.tab].set_command_line("Empty Redo Stack".to_string(), Type::Error);
        }
    }
    fn refresh_view(&mut self) {
        let offset = self.doc[self.tab].offset.y;
        for o in self.doc[self.tab]
            .rows
            .iter_mut()
            .skip(offset)
            .take(self.term.size.width)
        {
            o.updated = true;
        }
    }
    fn highlight_bg_tokens(&mut self, t: &str, current: Position) -> Option<()> {
        let occurances = self.doc[self.tab].find_all(t)?;
        for i in &mut self.doc[self.tab].rows {
            i.bg_syntax.clear();
        }
        if !t.is_empty() {
            for o in occurances {
                self.doc[self.tab].rows[o.y].bg_syntax.insert(
                    o.x,
                    Token {
                        span: (o.x, o.x + UnicodeWidthStr::width(t)),
                        data: t.to_string(),
                        kind: Reader::rgb_bg(
                            self.config.highlights[&self.theme][if o == current {
                                "search_active"
                            } else {
                                "search_inactive"
                            }],
                        )
                        .to_string(),
                        priority: 10,
                    },
                );
            }
        }
        None
    }
    fn search(&mut self) {
        // For searching the file
        let initial_cursor = self.doc[self.tab].cursor;
        let initial_offset = self.doc[self.tab].offset;
        let initial = Position {
            x: initial_cursor.x + initial_offset.x,
            y: initial_cursor.y + initial_offset.y - OFFSET,
        };
        // Ask for a search term after saving the current cursor position
        self.prompt("Search", ": ", &|s, e, t| {
            // Find all occurances in the document
            let current = Position {
                x: s.doc[s.tab].cursor.x + s.doc[s.tab].offset.x,
                y: s.doc[s.tab].cursor.y + s.doc[s.tab].offset.y - OFFSET,
            };
            match e {
                PromptEvent::KeyPress(c) => match c {
                    KeyCode::Up | KeyCode::Left => {
                        if let Some(p) = s.doc[s.tab].find_prev(t, &current) {
                            s.doc[s.tab].goto(p, &s.term.size);
                            s.refresh_view();
                            s.highlight_bg_tokens(&t, p);
                        }
                    }
                    KeyCode::Down | KeyCode::Right => {
                        if let Some(p) = s.doc[s.tab].find_next(t, &current) {
                            s.doc[s.tab].goto(p, &s.term.size);
                            s.refresh_view();
                            s.highlight_bg_tokens(&t, p);
                        }
                    }
                    KeyCode::Esc => {
                        s.doc[s.tab].cursor = initial_cursor;
                        s.doc[s.tab].offset = initial_offset;
                    }
                    _ => (),
                },
                PromptEvent::CharPress(backspace) => {
                    // Highlight the tokens
                    if backspace {
                        s.highlight_bg_tokens(&t, initial);
                    }
                    if let Some(p) = s.doc[s.tab].find_next(t, &initial) {
                        s.doc[s.tab].goto(p, &s.term.size);
                        s.refresh_view();
                        s.highlight_bg_tokens(&t, p);
                    } else {
                        s.doc[s.tab].goto(initial, &s.term.size);
                        s.highlight_bg_tokens(&t, initial);
                    }
                }
                PromptEvent::Update => (),
            }
        });
        // User cancelled or found what they were looking for
        for i in &mut self.doc[self.tab].rows {
            i.bg_syntax.clear();
        }
        self.doc[self.tab].set_command_line("Search exited".to_string(), Type::Info);
    }
    fn replace(&mut self) {
        // Replace text within the document
        let initial_cursor = self.doc[self.tab].cursor;
        let initial_offset = self.doc[self.tab].offset;
        let current = Position {
            x: self.doc[self.tab].cursor.x + self.doc[self.tab].offset.x,
            y: self.doc[self.tab].cursor.y + self.doc[self.tab].offset.y - OFFSET,
        };
        // After saving the cursor position, ask the user for the information
        if let Some(target) = self.prompt("Replace", ": ", &|_, _, _| {}) {
            let re = if let Ok(re) = Regex::new(&target) {
                re
            } else {
                self.doc[self.tab].set_command_line("Invalid Regex".to_string(), Type::Error);
                return;
            };
            self.highlight_bg_tokens(&target, current);
            if let Some(arrow) = self.prompt("With", ": ", &|_, _, _| {}) {
                if let Some(p) = self.doc[self.tab].find_next(&target, &current) {
                    self.doc[self.tab].goto(p, &self.term.size);
                    self.highlight_bg_tokens(&target, p);
                    self.update();
                }
                loop {
                    // Read an event
                    let key = if let InputEvent::Key(key) = self.read_event() {
                        key
                    } else {
                        continue;
                    };
                    let current = Position {
                        x: self.doc[self.tab].cursor.x + self.doc[self.tab].offset.x,
                        y: self.doc[self.tab].cursor.y + self.doc[self.tab].offset.y - OFFSET,
                    };
                    match key.code {
                        KeyCode::Up | KeyCode::Left => {
                            if let Some(p) = self.doc[self.tab].find_prev(&target, &current) {
                                self.doc[self.tab].goto(p, &self.term.size);
                                self.highlight_bg_tokens(&target, p);
                            }
                        }
                        KeyCode::Down | KeyCode::Right => {
                            if let Some(p) = self.doc[self.tab].find_next(&target, &current) {
                                self.doc[self.tab].goto(p, &self.term.size);
                                self.highlight_bg_tokens(&target, p);
                            }
                        }
                        KeyCode::Char('y') | KeyCode::Enter | KeyCode::Char(' ') => {
                            self.doc[self.tab].undo_stack.commit();
                            let before = self.doc[self.tab].rows[current.y].clone();
                            let after = Row::from(&*re.replace_all(&before.string, &arrow[..]));
                            self.doc[self.tab].rows[current.y] = after.clone();
                            self.highlight_bg_tokens(&target, current);
                            if before.string != after.string {
                                self.doc[self.tab].undo_stack.push(Event::UpdateLine(
                                    current,
                                    0,
                                    Box::new(before),
                                    Box::new(after),
                                ));
                            }
                        }
                        KeyCode::Esc => {
                            self.doc[self.tab].cursor = initial_cursor;
                            self.doc[self.tab].offset = initial_offset;
                            self.doc[self.tab]
                                .set_command_line("Replace finished".to_string(), Type::Info);
                            break;
                        }
                        _ => (),
                    }
                    self.update();
                }
            }
            for i in &mut self.doc[self.tab].rows {
                i.bg_syntax.clear();
            }
        }
    }
    fn replace_all(&mut self) {
        // Replace all occurances of a substring
        if let Some(target) = self.prompt("Replace all", ": ", &|_, _, _| {}) {
            let re = if let Ok(re) = Regex::new(&target) {
                re
            } else {
                return;
            };
            if let Some(arrow) = self.prompt("With", ": ", &|_, _, _| {}) {
                // Find all occurances
                let search_points = if let Some(t) = self.doc[self.tab].find_all(&target) {
                    t
                } else {
                    vec![]
                };
                for p in search_points {
                    let before = self.doc[self.tab].rows[p.y].clone();
                    let after = Row::from(&*re.replace_all(&before.string, &arrow[..]));
                    self.doc[self.tab].rows[p.y] = after.clone();
                    if before.string != after.string {
                        self.doc[self.tab].undo_stack.push(Event::UpdateLine(
                            Position { x: 0, y: p.y },
                            0,
                            Box::new(before),
                            Box::new(after),
                        ));
                    }
                }
            }
        }
        // Exit message
        self.doc[self.tab].set_command_line("Replace finished".to_string(), Type::Info);
    }
    fn dirty_prompt(&mut self, key: KeyBinding, subject: &str) -> bool {
        // For events that require changes to the document
        if self.doc[self.tab].dirty {
            // Handle unsaved changes
            self.doc[self.tab].set_command_line(
                format!("Unsaved Changes! {:?} to force {}", key, subject),
                Type::Warning,
            );
            self.update();
            if let InputEvent::Key(KeyEvent {
                code: c,
                modifiers: m,
            }) = self.read_event()
            {
                let ox_key = Editor::key_event_to_ox_key(c, m);
                match ox_key {
                    KeyBinding::Raw(RawKey::Enter) => return true,
                    KeyBinding::Ctrl(_) | KeyBinding::Alt(_) => {
                        if ox_key == key {
                            return true;
                        } else {
                            self.doc[self.tab].set_command_line(
                                format!("{} cancelled", title(subject)),
                                Type::Info,
                            );
                        }
                    }
                    _ => self.doc[self.tab]
                        .set_command_line(format!("{} cancelled", title(subject)), Type::Info),
                }
            }
        } else {
            return true;
        }
        false
    }
    fn prompt(
        &mut self,
        prompt: &str,
        ending: &str,
        func: &dyn Fn(&mut Self, PromptEvent, &str),
    ) -> Option<String> {
        // Create a new prompt
        self.doc[self.tab].set_command_line(format!("{}{}", prompt, ending), Type::Info);
        self.update();
        let mut result = String::new();
        'p: loop {
            if let InputEvent::Key(KeyEvent {
                code: c,
                modifiers: m,
            }) = self.read_event()
            {
                match Editor::key_event_to_ox_key(c, m) {
                    KeyBinding::Raw(RawKey::Enter) => {
                        // Exit on enter key
                        break 'p;
                    }
                    KeyBinding::Raw(RawKey::Char(c)) | KeyBinding::Shift(RawKey::Char(c)) => {
                        // Update the prompt contents
                        result.push(c);
                        func(self, PromptEvent::CharPress(false), &result)
                    }
                    KeyBinding::Raw(RawKey::Backspace) => {
                        // Handle backspace event
                        result.pop();
                        func(self, PromptEvent::CharPress(true), &result)
                    }
                    KeyBinding::Raw(RawKey::Esc) => {
                        // Handle escape key
                        func(self, PromptEvent::KeyPress(c), &result);
                        return None;
                    }
                    _ => func(self, PromptEvent::KeyPress(c), &result),
                }
            }
            self.doc[self.tab]
                .set_command_line(format!("{}{}{}", prompt, ending, result), Type::Info);
            func(self, PromptEvent::Update, &result);
            self.update();
        }
        Some(result)
    }
    fn update(&mut self) {
        // Move the cursor and render the screen
        Terminal::hide_cursor();
        Terminal::goto(&Position { x: 0, y: 0 });
        self.doc[self.tab].recalculate_offset(&self.config);
        self.render();
        Terminal::goto(&Position {
            x: self.doc[self.tab]
                .cursor
                .x
                .saturating_add(self.doc[self.tab].line_offset),
            y: self.doc[self.tab].cursor.y,
        });
        Terminal::show_cursor();
        Terminal::flush();
    }
    fn welcome_message(&self, text: &str, colour: SetForegroundColor) -> String {
        // Render the welcome message
        let pad = " ".repeat(
            (self.term.size.width / 2)
                .saturating_sub(text.len() / 2)
                .saturating_sub(self.config.general.line_number_padding_right)
                .saturating_sub(self.config.general.line_number_padding_left)
                .saturating_sub(1),
        );
        let pad_right = " ".repeat(
            (self.term.size.width.saturating_sub(1))
                .saturating_sub(text.len() + pad.len())
                .saturating_sub(self.config.general.line_number_padding_left)
                .saturating_sub(self.config.general.line_number_padding_right),
        );
        format!(
            "{}{}{}~{}{}{}{}{}{}{}{}",
            if self.config.theme.transparent_editor {
                RESET_BG
            } else {
                Reader::rgb_bg(self.config.theme.line_number_bg)
            },
            Reader::rgb_fg(self.config.theme.line_number_fg),
            " ".repeat(self.config.general.line_number_padding_left),
            RESET_FG,
            colour,
            " ".repeat(self.config.general.line_number_padding_right),
            if self.config.theme.transparent_editor {
                RESET_BG
            } else {
                Reader::rgb_bg(self.config.theme.editor_bg)
            },
            trim_end(
                &format!("{}{}", pad, text),
                self.term.size.width.saturating_sub(4)
            ),
            pad_right,
            RESET_FG,
            RESET_BG,
        )
    }
    fn status_line(&mut self) -> String {
        // Produce the status line
        // Create the left part of the status line
        let left = self.doc[self.tab].format(&self.config.general.status_left);
        // Create the right part of the status line
        let right = self.doc[self.tab].format(&self.config.general.status_right);
        // Get the padding value
        let padding = self.term.align_break(&left, &right);
        // Generate it
        format!(
            "{}{}{}{}{}{}{}",
            Attribute::Bold,
            Reader::rgb_fg(self.config.theme.status_fg),
            Reader::rgb_bg(self.config.theme.status_bg),
            trim_end(
                &format!("{}{}{}", left, padding, right),
                self.term.size.width
            ),
            RESET_BG,
            RESET_FG,
            Attribute::Reset,
        )
    }
    fn add_background(&self, text: &str) -> String {
        // Add a background colour to a line
        if self.config.theme.transparent_editor {
            text.to_string()
        } else {
            format!(
                "{}{}{}{}",
                Reader::rgb_bg(self.config.theme.editor_bg),
                text,
                self.term.align_left(&text),
                RESET_BG
            )
        }
    }
    fn command_line(&self) -> String {
        // Render the command line
        let line = &self.doc[self.tab].cmd_line.text;
        // Add the correct styling
        match self.doc[self.tab].cmd_line.msg {
            Type::Error => self.add_background(&format!(
                "{}{}{}{}{}",
                Attribute::Bold,
                Reader::rgb_fg(self.config.theme.error_fg),
                self.add_background(&trim_end(&line, self.term.size.width)),
                RESET_FG,
                Attribute::Reset
            )),
            Type::Warning => self.add_background(&format!(
                "{}{}{}{}{}",
                Attribute::Bold,
                Reader::rgb_fg(self.config.theme.warning_fg),
                self.add_background(&trim_end(&line, self.term.size.width)),
                RESET_FG,
                Attribute::Reset
            )),
            Type::Info => self.add_background(&format!(
                "{}{}{}",
                Reader::rgb_fg(self.config.theme.info_fg),
                self.add_background(&trim_end(&line, self.term.size.width)),
                RESET_FG,
            )),
        }
    }
    fn tab_line(&mut self) -> String {
        // Render the tab line
        let mut result = vec![];
        let mut widths = vec![];
        let active_background = Reader::rgb_bg(self.config.theme.active_tab_bg);
        let inactive_background = Reader::rgb_bg(self.config.theme.inactive_tab_bg);
        let active_foreground = Reader::rgb_fg(self.config.theme.active_tab_fg);
        let inactive_foreground = Reader::rgb_fg(self.config.theme.inactive_tab_fg);
        // Iterate through documents and create their tab text
        for num in 0..self.doc.len() {
            let this = format!(
                "{} {} {}{}{}\u{2502}",
                if num == self.tab {
                    format!(
                        "{}{}{}",
                        Attribute::Bold,
                        active_background,
                        active_foreground,
                    )
                } else {
                    format!("{}{}", inactive_background, inactive_foreground,)
                },
                self.doc[num].format(&self.config.general.tab),
                Attribute::Reset,
                inactive_background,
                inactive_foreground,
            );
            widths.push(self.exp.ansi_len(this.as_str()));
            result.push(this);
        }
        // Determine if the tab can be rendered on screen
        let mut more_right = true;
        while widths.iter().sum::<usize>() > self.term.size.width {
            if self.tab == 0 || self.tab == 1 {
                result.pop();
                widths.pop();
                more_right = false;
            } else {
                result.remove(0);
                widths.remove(0);
            }
        }
        if widths.iter().sum::<usize>() < self.term.size.width.saturating_sub(3) && !more_right {
            result.push("...".to_string());
        }
        let result = result.join("");
        format!(
            "{}{}{}{}{}{}",
            inactive_background,
            inactive_foreground,
            result,
            self.term.align_left(&result),
            RESET_FG,
            RESET_BG,
        )
    }
    fn render(&mut self) {
        // Draw the screen to the terminal
        let offset = self.doc[self.tab].offset;
        let mut frame = vec![self.tab_line()];
        let rendered = self.doc[self.tab].render(TabType::Spaces, 0);
        let reg = self.doc[self.tab].regex.clone();
        if self.config.theme.transparent_editor {
            // Prevent garbage characters spamming the screen
            Terminal::clear();
        }
        for row in OFFSET..self.term.size.height {
            // Clear the current line
            let row = row.saturating_sub(OFFSET);
            if let Some(r) = self.doc[self.tab].rows.get_mut(offset.y + row) {
                if r.updated {
                    r.update_syntax(&self.config, &reg, &rendered, offset.y + row, &self.theme);
                    r.updated = false;
                }
            }
            if row == self.term.size.height - 1 - OFFSET {
                // Render command line
                frame.push(self.command_line());
            } else if row == self.term.size.height - 2 - OFFSET {
                // Render status line
                frame.push(self.status_line());
            } else if row == self.term.size.height / 4 - OFFSET && self.doc[self.tab].show_welcome {
                frame.push(self.welcome_message(
                    &format!("Ox editor  v{}", VERSION),
                    Reader::rgb_fg(self.config.theme.editor_fg),
                ));
            } else if row == (self.term.size.height / 4).saturating_add(1) - OFFSET
                && self.doc[self.tab].show_welcome
            {
                frame.push(self.welcome_message(
                    "A Rust powered editor by Luke",
                    Reader::rgb_fg(self.config.theme.editor_fg),
                ));
            } else if row == (self.term.size.height / 4).saturating_add(3) - OFFSET
                && self.doc[self.tab].show_welcome
            {
                frame.push(self.welcome_message(
                    "To access the wiki: Press F1",
                    Reader::rgb_fg(self.config.theme.status_fg),
                ));
            } else if row == (self.term.size.height / 4).saturating_add(5) - OFFSET
                && self.doc[self.tab].show_welcome
            {
                frame.push(self.welcome_message(
                    "Start typing to begin",
                    Reader::rgb_fg(self.config.theme.status_fg),
                ));
            } else if let Some(line) = self.doc[self.tab]
                .rows
                .get(self.doc[self.tab].offset.y + row)
            {
                // Render lines of code
                frame.push(self.add_background(&line.render(
                    self.doc[self.tab].offset.x,
                    self.term.size.width,
                    self.doc[self.tab].offset.y + row,
                    self.doc[self.tab].line_offset,
                    &self.config,
                )));
            } else {
                // Render empty lines
                let o = self.doc[self.tab].line_offset.saturating_sub(
                    1 + self.config.general.line_number_padding_right
                        + self.config.general.line_number_padding_left,
                );
                frame.push(format!(
                    "{}{}{}",
                    Reader::rgb_fg(self.config.theme.line_number_fg),
                    self.add_background(&format!(
                        "{}{}~{}{}{}",
                        if self.config.theme.transparent_editor {
                            RESET_BG
                        } else {
                            Reader::rgb_bg(self.config.theme.line_number_bg)
                        },
                        " ".repeat(self.config.general.line_number_padding_left),
                        " ".repeat(self.config.general.line_number_padding_right),
                        " ".repeat(o),
                        Reader::rgb_bg(self.config.theme.editor_bg),
                    )),
                    RESET_FG
                ));
            }
        }
        print!("{}", frame.join("\r\n"));
    }
}
