// Editor.rs - Controls the editor and brings everything together
use crate::config::{Reader, Status};
use crate::document::Type;
use crate::util::{is_ahead, is_behind, title, trim_end, Exp};
use crate::{Document, Event, Row, Terminal, VERSION};
use clap::App;
use regex::Regex;
use std::time::{Duration, Instant};
use std::{io::Error, thread};
use termion::event::Key;
use termion::input::{Keys, TermRead};
use termion::{async_stdin, color, style, AsyncReader};
use unicode_segmentation::UnicodeSegmentation;

// Set up color resets
pub const RESET_BG: color::Bg<color::Reset> = color::Bg(color::Reset);
pub const RESET_FG: color::Fg<color::Reset> = color::Fg(color::Reset);

// Set up offset rules
pub const OFFSET: usize = 1;

// Enum for holding prompt events
enum PromptEvent {
    Update,
    CharPress,
    KeyPress(Key),
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
    pub status: Status,             // Holding the status of the config
    quit: bool,                     // Toggle for cleanly quitting the editor
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
            documents.push(Document::new(&config.0, &config.1));
        } else {
            for file in &files {
                documents.push(Document::from(&config.0, &config.1, file));
            }
        }
        // Create the new editor instance
        Ok(Self {
            quit: false,
            // Display information about the config file into text for the status line
            term: Terminal::new()?,
            tab: 0,
            doc: documents,
            last_keypress: None,
            stdin: async_stdin().keys(),
            config: config.0.clone(),
            status: config.1,
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
                    if self.doc[self.tab].cursor.y > self.term.size.height.saturating_sub(3) {
                        // Prevent cursor going off the screen and breaking everything
                        self.doc[self.tab].cursor.y = self.term.size.height.saturating_sub(3);
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
            Key::Char(c) => self.doc[self.tab].character(c, &self.term.size, &self.config),
            Key::Backspace => self.doc[self.tab].backspace(&self.term.size),
            Key::Ctrl('q') => self.quit(),
            Key::Ctrl('s') => self.save(),
            Key::Ctrl('w') => self.save_as(),
            Key::Ctrl('p') => self.save_all(),
            Key::Ctrl('n') => self.new_document(),
            Key::Ctrl('o') => self.open_document(),
            Key::Ctrl('f') => self.search(),
            Key::Ctrl('u') => self.doc[self.tab].undo(&self.config, &self.term.size),
            Key::Ctrl('y') => self.doc[self.tab].redo(&self.config, &self.term.size),
            Key::Ctrl('r') => self.replace(),
            Key::Ctrl('a') => self.replace_all(),
            Key::Ctrl('d') => self.prev_tab(),
            Key::Ctrl('h') => self.next_tab(),
            Key::Left | Key::Right | Key::Up | Key::Down => {
                self.doc[self.tab].move_cursor(key, &self.term.size)
            }
            Key::PageDown | Key::PageUp | Key::Home | Key::End => {
                self.doc[self.tab].leap_cursor(key, &self.term.size)
            }
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
                self.doc[self.tab].set_command_line("Closed tab".to_string(), Type::Info);
            } else {
                // Close current tab and move left
                self.doc.remove(self.tab);
                self.doc[self.tab].set_command_line("Closed tab".to_string(), Type::Info);
            }
        }
    }
    fn new_document(&mut self) {
        // Handle new document event
        self.doc.push(Document::new(&self.config, &self.status));
        self.tab = self.doc.len().saturating_sub(1);
        self.doc[self.tab].dirty = false;
        self.doc[self.tab].show_welcome = true;
        self.doc[self.tab].cursor.y = OFFSET;
        self.doc[self.tab].offset.y = 0;
        self.doc[self.tab].leap_cursor(Key::Home, &self.term.size);
    }
    fn open_document(&mut self) {
        // Handle open document event
        // TODO: Highlight entire file here
        if let Some(result) = self.prompt("Open", &|_, _, _| {}) {
            if let Some(doc) = Document::open(&self.config, &self.status, &result[..]) {
                // Overwrite the current document
                self.doc.push(doc);
                self.tab = self.doc.len().saturating_sub(1);
                self.doc[self.tab].dirty = false;
                self.doc[self.tab].show_welcome = false;
                self.doc[self.tab].cursor.y = OFFSET;
                self.doc[self.tab].offset.y = 0;
                self.doc[self.tab].leap_cursor(Key::Home, &self.term.size);
            } else {
                self.doc[self.tab]
                    .set_command_line("File couldn't be opened".to_string(), Type::Error);
            }
        }
    }
    fn save(&mut self) {
        // Handle save event
        let path = self.doc[self.tab].path.clone();
        if self.doc[self.tab].save().is_ok() {
            // The document saved successfully
            self.doc[self.tab].dirty = false;
            self.doc[self.tab]
                .set_command_line(format!("File saved to {} successfully", path), Type::Info);
        } else {
            // The document couldn't save due to permission errors
            self.doc[self.tab]
                .set_command_line(format!("Failed to save file to {}", path), Type::Error);
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
                self.doc[self.tab]
                    .set_command_line(format!("File saved to {} successfully", result), Type::Info);
                self.doc[self.tab].icon = Document::identify(&result);
                self.doc[self.tab].name = result.clone();
                self.doc[self.tab].path = result;
            } else {
                // The document couldn't save to the file
                self.doc[self.tab]
                    .set_command_line(format!("Failed to save file to {}", result), Type::Error);
            }
        } else {
            // User pressed the escape key
            self.doc[self.tab].set_command_line("Save as cancelled".to_string(), Type::Info);
        }
        // Commit to the undo stack on save as
        self.doc[self.tab].undo_stack.commit();
    }
    fn save_all(&mut self) {
        for i in 0..self.doc.len() {
            let path = self.doc[i].path.clone();
            if self.doc[i].save().is_ok() {
                // The document saved successfully
                self.doc[i].dirty = false;
                self.doc[i]
                    .set_command_line(format!("File saved to {} successfully", path), Type::Info);
            } else {
                // The document couldn't save due to permission errors
                self.doc[i]
                    .set_command_line(format!("Failed to save file to {}", path), Type::Error);
            }
            // Commit to undo stack on document save
            self.doc[i].undo_stack.commit();
        }
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
                                s.doc[s.tab].recalculate_graphemes();
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
                                s.doc[s.tab].recalculate_graphemes();
                                break;
                            }
                        }
                    }
                    Key::Esc => {
                        // Restore cursor and offset position
                        s.doc[s.tab].cursor = initial_cursor;
                        s.doc[s.tab].offset = initial_offset;
                        s.doc[s.tab].recalculate_graphemes();
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
                                s.doc[s.tab].recalculate_graphemes();
                                break;
                            }
                        }
                    }
                }
                PromptEvent::Update => (),
            }
        });
        // User cancelled or found what they were looking for
        self.doc[self.tab].set_command_line("Search exited".to_string(), Type::Info);
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
                        self.doc[self.tab].recalculate_graphemes();
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
                                    self.doc[self.tab].recalculate_graphemes();
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
                                    self.doc[self.tab].recalculate_graphemes();
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
                            self.doc[self.tab].snap_cursor(&self.term.size);
                            self.doc[self.tab].prevent_unicode_hell();
                            self.doc[self.tab].recalculate_graphemes();
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
                self.doc[self.tab].set_command_line("Replace finished".to_string(), Type::Info);
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
        self.doc[self.tab].snap_cursor(&self.term.size);
        self.doc[self.tab].prevent_unicode_hell();
        self.doc[self.tab].recalculate_graphemes();
        self.doc[self.tab].set_command_line("Replaced targets".to_string(), Type::Info);
    }
    fn dirty_prompt(&mut self, key: char, subject: &str) -> bool {
        // For events that require changes to the document
        if self.doc[self.tab].dirty {
            // Handle unsaved changes
            self.doc[self.tab].set_command_line(
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
                        self.doc[self.tab]
                            .set_command_line(format!("{} cancelled", title(subject)), Type::Info);
                    }
                }
                _ => self.doc[self.tab]
                    .set_command_line(format!("{} cancelled", title(subject)), Type::Info),
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
        self.doc[self.tab].set_command_line(format!("{}: ", prompt), Type::Info);
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
            self.doc[self.tab].set_command_line(format!("{}: {}", prompt, result), Type::Info);
            func(self, PromptEvent::Update, &result);
            self.update();
        }
        Some(result)
    }
    fn goto(&mut self, pos: &Position) {
        // Move the cursor to a specific location
        let max_y = self.term.size.height.saturating_sub(3);
        let max_x = (self.term.size.width).saturating_sub(self.doc[self.tab].line_offset);
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
        let pad = " ".repeat((self.term.size.width / 2).saturating_sub(text.len() / 2));
        let pad_right = " ".repeat(
            (self.term.size.width.saturating_sub(1))
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
                self.term.size.width.saturating_sub(1)
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
                self.term.size.width
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
        let line = &self.doc[self.tab].cmd_line.text;
        // Add the correct styling
        match self.doc[self.tab].cmd_line.msg {
            Type::Error => self.add_background(&format!(
                "{}{}{}{}{}",
                style::Bold,
                color::Fg(color::Red),
                self.add_background(&trim_end(&line, self.term.size.width)),
                color::Fg(color::Reset),
                style::Reset
            )),
            Type::Warning => self.add_background(&format!(
                "{}{}{}{}{}",
                style::Bold,
                color::Fg(color::Yellow),
                self.add_background(&trim_end(&line, self.term.size.width)),
                color::Fg(color::Reset),
                style::Reset
            )),
            Type::Info => self.add_background(&trim_end(&line, self.term.size.width)),
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
                this = format!(
                    "{} {}{} |",
                    Reader::rgb_bg(self.config.theme.editor_bg),
                    name,
                    if doc.dirty { "[+]" } else { "" }
                );
            } else if num == self.tab {
                // Render active tab
                this = format!(
                    "{}{} {}{} {}{}|",
                    Reader::rgb_bg(self.config.theme.editor_bg),
                    style::Bold,
                    name,
                    if doc.dirty { "[+]" } else { "" },
                    style::Reset,
                    Reader::rgb_bg(self.config.theme.status_bg)
                );
            } else if num.saturating_sub(1) == self.tab {
                this = format!(
                    "{} {}{} |",
                    Reader::rgb_bg(self.config.theme.status_bg),
                    name,
                    if doc.dirty { "[+]" } else { "" }
                );
            } else {
                this = format!(" {}{} |", name, if doc.dirty { "[+]" } else { "" });
            }
            // Check if tab will fit in window width, otherwise put in a "..."
            width += self.exp.ansi_len(&this);
            if width + 3 > self.term.size.width {
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
        for row in OFFSET..self.term.size.height {
            let row = row.saturating_sub(OFFSET);
            if let Some(r) = self.doc[self.tab].rows.get_mut(offset.y + row) {
                r.update_syntax(&self.config, &reg, &rendered, offset.y + row);
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
                    "Ctrl + Q: Exit   ",
                    Reader::rgb_fg(self.config.theme.status_fg),
                ));
            } else if row == (self.term.size.height / 4).saturating_add(4) - OFFSET
                && self.doc[self.tab].show_welcome
            {
                frame.push(self.welcome_message(
                    "Ctrl + S: Save   ",
                    Reader::rgb_fg(self.config.theme.status_fg),
                ));
            } else if row == (self.term.size.height / 4).saturating_add(5) - OFFSET
                && self.doc[self.tab].show_welcome
            {
                frame.push(self.welcome_message(
                    "Ctrl + W: Save as",
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
