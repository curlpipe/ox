// Document.rs - For managing external files
use crate::config::{Reader, Status, TokenType};
use crate::editor::OFFSET;
use crate::util::{raw_to_grapheme, tabs_to_spaces, spaces_to_tabs};
use crate::{Event, EventStack, Position, Row, Size};
use regex::Regex;
use std::{cmp, fs};
use termion::event::Key;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

// For holding the info in the command line
pub struct CommandLine {
    pub msg: Type,
    pub text: String,
}

// Enum for the kinds of status messages
pub enum Type {
    Error,
    Warning,
    Info,
}

// Document struct (class) to manage files and text
pub struct Document {
    pub rows: Vec<Row>,         // For holding the contents of the document
    pub path: String,           // For holding the path to the document
    pub name: String,           // For holding the name of the document
    pub dirty: bool,            // True if the current document has been edited
    pub cmd_line: CommandLine,  // For holding the command line
    pub line_offset: usize,     // For holding a line number offset
    pub undo_stack: EventStack, // For holding the undo event stack
    pub redo_stack: EventStack, // For holding the redo event stack
    pub regex: Vec<TokenType>,  // For holding regular expressions
    pub icon: String,           // For holding the icon of the document
    pub kind: String,           // For holding the icon of the document
    pub show_welcome: bool,     // Whether to show welcome in the document
    pub cursor: Position,       // For holding the raw cursor location
    pub offset: Position,       // For holding the offset on the X and Y axes
    pub graphemes: usize,       // For holding the special grapheme cursor
    pub tabs: bool,             // For detecting if tabs are used over spaces
}

// Add methods to the document struct
impl Document {
    pub fn new(config: &Reader, status: &Status) -> Self {
        // Create a new, empty document
        Self {
            rows: vec![Row::from("")],
            name: String::from("[No name]"),
            dirty: false,
            cmd_line: Document::config_to_commandline(&status),
            path: String::new(),
            line_offset: config.general.line_number_padding_right
                + config.general.line_number_padding_left,
            undo_stack: EventStack::new(),
            redo_stack: EventStack::new(),
            regex: Reader::get_syntax_regex(&config, ""),
            icon: String::new(),
            kind: String::new(),
            show_welcome: true,
            graphemes: 0,
            cursor: Position { x: 0, y: OFFSET },
            offset: Position { x: 0, y: 0 },
            tabs: false,
        }
    }
    pub fn open(config: &Reader, status: &Status, path: &str) -> Option<Self> {
        // Create a new document from a path
        if let Ok(file) = fs::read_to_string(path) {
            // File exists
            let file = tabs_to_spaces(&file, config.general.tab_width);
            let mut file = file.split('\n').collect::<Vec<&str>>();
            // Handle newline on last line
            if let Some(line) = file.iter().last() {
                if line.is_empty() {
                    let _ = file.pop();
                }
            }
            // Handle empty document by automatically inserting a row
            if file.is_empty() {
                file.push("");
            }
            let ext = path.split('.').last().unwrap_or(&"");
            Some(Self {
                rows: file.iter().map(|row| Row::from(*row)).collect(),
                name: path.to_string(),
                dirty: false,
                cmd_line: Document::config_to_commandline(&status),
                path: path.to_string(),
                line_offset: config.general.line_number_padding_right
                    + config.general.line_number_padding_left,
                undo_stack: EventStack::new(),
                redo_stack: EventStack::new(),
                regex: Reader::get_syntax_regex(&config, ext),
                kind: Self::identify(path).0.to_string(),
                icon: Self::identify(path).1.to_string(),
                show_welcome: false,
                graphemes: 0,
                cursor: Position { x: 0, y: OFFSET },
                offset: Position { x: 0, y: 0 },
                tabs: file.contains(&"\n\t"),
            })
        } else {
            // File doesn't exist
            None
        }
    }
    pub fn from(config: &Reader, status: &Status, path: &str) -> Self {
        // Create a new document from a path with empty document on error
        if let Some(doc) = Document::open(&config, &status, path) {
            doc
        } else {
            // Create blank document
            let ext = path.split('.').last().unwrap_or(&"");
            Self {
                rows: vec![Row::from("")],
                name: path.to_string(),
                path: path.to_string(),
                dirty: false,
                cmd_line: Document::config_to_commandline(&status),
                line_offset: config.general.line_number_padding_right
                    + config.general.line_number_padding_left,
                undo_stack: EventStack::new(),
                redo_stack: EventStack::new(),
                regex: Reader::get_syntax_regex(&config, ext),
                kind: Self::identify(path).0.to_string(),
                icon: Self::identify(path).1.to_string(),
                show_welcome: false,
                graphemes: 0,
                cursor: Position { x: 0, y: OFFSET },
                offset: Position { x: 0, y: 0 },
                tabs: false,
            }
        }
    }
    pub fn set_command_line(&mut self, text: String, msg: Type) {
        // Function to update the command line
        self.cmd_line = CommandLine { text, msg };
    }
    fn config_to_commandline(status: &Status) -> CommandLine {
        CommandLine {
            text: match status {
                Status::Success => "Welcome to Ox".to_string(),
                Status::File => "Config file not found, using default values".to_string(),
                Status::Parse(error) => format!("Failed to parse: {:?}", error),
            },
            msg: match status {
                Status::Success => Type::Info,
                Status::File => Type::Warning,
                Status::Parse(_) => Type::Error,
            },
        }
    }
    pub fn move_cursor(&mut self, direction: Key, term: &Size) {
        // Move the cursor around the editor
        match direction {
            Key::Down => {
                // Move the cursor down
                if self.cursor.y + self.offset.y + 1 - (OFFSET) < self.rows.len() {
                    // If the proposed move is within the length of the document
                    if self.cursor.y == term.height.saturating_sub(3) {
                        self.offset.y = self.offset.y.saturating_add(1);
                    } else {
                        self.cursor.y = self.cursor.y.saturating_add(1);
                    }
                    self.snap_cursor(term);
                    self.prevent_unicode_hell();
                    self.recalculate_graphemes();
                }
            }
            Key::Up => {
                // Move the cursor up
                if self.cursor.y - OFFSET == 0 {
                    self.offset.y = self.offset.y.saturating_sub(1);
                } else if self.cursor.y != OFFSET {
                    self.cursor.y = self.cursor.y.saturating_sub(1);
                }
                self.snap_cursor(term);
                self.prevent_unicode_hell();
                self.recalculate_graphemes();
            }
            Key::Right => {
                // Move the cursor right
                let line = &self.rows[self.cursor.y + self.offset.y - OFFSET];
                // Work out the width of the character to traverse
                let mut jump = 1;
                if let Some(chr) = line.ext_chars().get(self.cursor.x + self.offset.x) {
                    jump = UnicodeWidthStr::width(*chr);
                }
                // Check the proposed move is within the current line length
                if line.length() > self.cursor.x + self.offset.x {
                    // Check for normal width character
                    let indicator1 =
                        self.cursor.x == term.width.saturating_sub(self.line_offset + jump + 1);
                    // Check for half broken unicode character
                    let indicator2 =
                        self.cursor.x == term.width.saturating_sub(self.line_offset + jump);
                    if indicator1 || indicator2 {
                        self.offset.x = self.offset.x.saturating_add(jump);
                    } else {
                        self.cursor.x = self.cursor.x.saturating_add(jump);
                    }
                    self.graphemes = self.graphemes.saturating_add(1);
                }
            }
            Key::Left => {
                // Move the cursor left
                let line = &self.rows[self.cursor.y + self.offset.y - OFFSET];
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
    pub fn leap_cursor(&mut self, action: Key, term: &Size) {
        // Handle large cursor movements
        match action {
            Key::PageUp => {
                // Move cursor to the top of the screen
                self.cursor.y = OFFSET;
                self.snap_cursor(term);
                self.prevent_unicode_hell();
                self.recalculate_graphemes();
            }
            Key::PageDown => {
                // Move cursor to the bottom of the screen
                self.cursor.y = cmp::min(
                    self.rows.len().saturating_sub(1).saturating_add(OFFSET),
                    term.height.saturating_sub(3) as usize,
                );
                self.snap_cursor(term);
                self.prevent_unicode_hell();
                self.recalculate_graphemes();
            }
            Key::Home => {
                // Move cursor to the start of the line
                self.offset.x = 0;
                self.cursor.x = 0;
                self.graphemes = 0;
            }
            Key::End => {
                // Move cursor to the end of the line
                let cursor = self.cursor;
                let offset = self.offset;
                let line = self.rows[cursor.y + offset.y - OFFSET].clone();
                if line.length() >= term.width.saturating_sub(self.line_offset) {
                    // Work out the width of the character to traverse
                    let mut jump = 1;
                    if let Some(chr) = line.ext_chars().get(line.length()) {
                        jump = UnicodeWidthStr::width(*chr);
                    }
                    self.offset.x = line
                        .length()
                        .saturating_add(jump + self.line_offset + 1)
                        .saturating_sub(term.width as usize);
                    self.cursor.x = term.width.saturating_sub(jump + self.line_offset + 1);
                } else {
                    self.cursor.x = line.length();
                }
                self.graphemes = line.chars().len();
            }
            _ => (),
        }
    }
    pub fn snap_cursor(&mut self, term: &Size) {
        // Snap the cursor to the end of the row when outside
        let current = self.rows[self.cursor.y + self.offset.y - OFFSET].clone();
        if current.length() <= self.cursor.x + self.offset.x {
            // If the cursor is out of bounds
            self.leap_cursor(Key::Home, term);
            self.leap_cursor(Key::End, term);
        }
    }
    pub fn prevent_unicode_hell(&mut self) {
        // Make sure that the cursor isn't inbetween a unicode character
        let line = &self.rows[self.cursor.y + self.offset.y - OFFSET];
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
    pub fn recalculate_graphemes(&mut self) {
        // Recalculate the grapheme cursor after moving up and down
        let current = self.rows[self.cursor.y + self.offset.y - OFFSET].clone();
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
    pub fn recalculate_offset(&mut self, config: &Reader) {
        // Calculate the offset for the line numbers
        self.line_offset = self.rows.len().to_string().len()
            + config.general.line_number_padding_right
            + config.general.line_number_padding_left;
    }
    pub fn character(&mut self, c: char, term: &Size, config: &Reader) {
        // The user pressed a character key
        self.dirty = true;
        self.show_welcome = false;
        match c {
            '\n' => self.return_key(term), // The user pressed the return key
            '\t' => {
                // The user pressed the tab key
                self.tab(&config, term);
                self.undo_stack.push(Event::InsertTab(Position {
                    x: self.cursor.x + self.offset.x,
                    y: self.cursor.y + self.offset.y - OFFSET,
                }));
            }
            _ => {
                // Other characters
                // TODO: Update relavent lines here
                self.dirty = true;
                self.show_welcome = false;
                self.rows[self.cursor.y + self.offset.y - OFFSET].insert(c, self.graphemes);
                self.undo_stack.push(Event::InsertMid(
                    Position {
                        x: self.cursor.x + self.offset.x,
                        y: self.cursor.y + self.offset.y - OFFSET,
                    },
                    c,
                ));
                // Commit to the undo stack if space key pressed
                if c == ' ' {
                    self.undo_stack.commit();
                }
                self.move_cursor(Key::Right, term);
            }
        }
        // Wipe the redo stack to avoid conflicts
        self.redo_stack.empty();
    }
    pub fn tab(&mut self, config: &Reader, term: &Size) {
        // Insert a tab
        // TODO: Update relavent lines here
        for _ in 0..config.general.tab_width {
            self.rows[self.cursor.y + self.offset.y - OFFSET].insert(' ', self.graphemes);
            self.move_cursor(Key::Right, term);
        }
    }
    pub fn return_key(&mut self, term: &Size) {
        // Return key
        self.dirty = true;
        self.show_welcome = false;
        // TODO: Update relavent lines here
        if self.cursor.x + self.offset.x == 0 {
            // Return key pressed at the start of the line
            self.rows
                .insert(self.cursor.y + self.offset.y - OFFSET, Row::from(""));
            self.undo_stack.push(Event::ReturnStart(Position {
                x: self.cursor.x + self.offset.x,
                y: self.cursor.y + self.offset.y - OFFSET,
            }));
            self.move_cursor(Key::Down, term);
        } else if self.cursor.x + self.offset.x
            == self.rows[self.cursor.y + self.offset.y - OFFSET].length()
        {
            // Return key pressed at the end of the line
            self.rows
                .insert(self.cursor.y + self.offset.y + 1 - OFFSET, Row::from(""));
            self.undo_stack.push(Event::ReturnEnd(Position {
                x: self.cursor.x + self.offset.x,
                y: self.cursor.y + self.offset.y - OFFSET,
            }));
            self.move_cursor(Key::Down, term);
            self.leap_cursor(Key::Home, term);
            self.recalculate_graphemes();
        } else {
            // Return key pressed in the middle of the line
            let current = self.rows[self.cursor.y + self.offset.y - OFFSET].chars();
            let before = Row::from(&current[..self.graphemes].join("")[..]);
            let after = Row::from(&current[self.graphemes..].join("")[..]);
            self.rows
                .insert(self.cursor.y + self.offset.y + 1 - OFFSET, after);
            self.rows[self.cursor.y + self.offset.y - OFFSET] = before.clone();
            self.undo_stack.push(Event::ReturnMid(
                Position {
                    x: self.cursor.x + self.offset.x,
                    y: self.cursor.y + self.offset.y - OFFSET,
                },
                before.length(),
            ));
            self.move_cursor(Key::Down, term);
            self.leap_cursor(Key::Home, term);
        }
        // Commit to undo stack when return key pressed
        self.undo_stack.commit();
    }
    pub fn backspace(&mut self, term: &Size) {
        // Handling the backspace key
        self.dirty = true;
        self.show_welcome = false;
        // TODO: Update relavent lines here
        if self.cursor.x + self.offset.x == 0 && self.cursor.y + self.offset.y - OFFSET != 0 {
            // Backspace at the start of a line
            let current = self.rows[self.cursor.y + self.offset.y - OFFSET]
                .string
                .clone();
            let prev = self.rows[self.cursor.y + self.offset.y - 1 - OFFSET].clone();
            self.rows[self.cursor.y + self.offset.y - 1 - OFFSET] =
                Row::from(&(prev.string.clone() + &current)[..]);
            self.rows.remove(self.cursor.y + self.offset.y - OFFSET);
            self.move_cursor(Key::Up, term);
            self.cursor.x = prev.length();
            self.recalculate_graphemes();
            self.undo_stack.push(Event::BackspaceStart(Position {
                x: self.cursor.x + self.offset.x,
                y: self.cursor.y + self.offset.y - OFFSET,
            }));
            self.undo_stack.commit();
        } else {
            // Backspace in the middle of a line
            self.move_cursor(Key::Left, term);
            let ch = self.rows[self.cursor.y + self.offset.y - OFFSET].clone();
            self.rows[self.cursor.y + self.offset.y - OFFSET].delete(self.graphemes);
            if let Some(ch) = ch.chars().get(self.graphemes) {
                if let Ok(ch) = ch.parse() {
                    self.undo_stack.push(Event::BackspaceMid(
                        Position {
                            x: self.cursor.x + self.offset.x,
                            y: self.cursor.y + self.offset.y - OFFSET,
                        },
                        ch,
                    ));
                }
            }
        }
    }
    pub fn redo(&mut self, config: &Reader, term: &Size) {
        // Redo an action
        if let Some(events) = self.redo_stack.pop() {
            for event in events.iter().rev() {
                // Reverse the undo action
                match event {
                    // TODO: Update relavent lines here
                    Event::InsertTab(pos) => {
                        self.cursor.y = pos.y - self.offset.y + OFFSET;
                        self.cursor.x =
                            pos.x.saturating_sub(config.general.tab_width) - self.offset.x;
                        self.recalculate_graphemes();
                        self.tab(&config, &term);
                    }
                    Event::InsertMid(pos, c) => {
                        let c_len = UnicodeWidthChar::width(*c).map_or(0, |c| c);
                        self.cursor.y = pos.y - self.offset.y + OFFSET;
                        self.cursor.x = pos.x.saturating_add(c_len) - self.offset.x;
                        self.recalculate_graphemes();
                        self.rows[pos.y].insert(*c, pos.x);
                    }
                    Event::BackspaceMid(pos, _) => {
                        self.cursor.y = pos.y - self.offset.y + OFFSET;
                        self.cursor.x = pos.x - self.offset.x;
                        self.recalculate_graphemes();
                        self.rows[pos.y].delete(pos.x);
                    }
                    Event::ReturnEnd(pos) => {
                        self.cursor.y = pos.y - self.offset.y + OFFSET;
                        self.cursor.x = pos.x - self.offset.x;
                        self.recalculate_graphemes();
                        self.rows.insert(pos.y + 1, Row::from(""));
                        self.move_cursor(Key::Down, &term);
                    }
                    Event::ReturnStart(pos) => {
                        self.cursor.y = pos.y - self.offset.y + OFFSET;
                        self.cursor.x = pos.x - self.offset.x;
                        self.recalculate_graphemes();
                        self.rows.insert(pos.y, Row::from(""));
                        self.move_cursor(Key::Down, &term);
                    }
                    Event::ReturnMid(pos, breakpoint) => {
                        self.cursor.y = pos.y - self.offset.y + OFFSET;
                        self.cursor.x = pos.x - self.offset.x;
                        self.recalculate_graphemes();
                        let current = self.rows[pos.y].string.clone();
                        let before = Row::from(&current[..*breakpoint]);
                        let after = Row::from(&current[*breakpoint..]);
                        self.rows.insert(pos.y + 1, after);
                        self.rows[pos.y] = before;
                        self.move_cursor(Key::Down, &term);
                        self.leap_cursor(Key::Home, &term);
                    }
                    Event::BackspaceStart(pos) => {
                        self.cursor.y = pos.y - self.offset.y + OFFSET;
                        self.recalculate_graphemes();
                        let current = self.rows[pos.y + 1].string.clone();
                        let prev = self.rows[pos.y].clone();
                        self.rows[pos.y + 1] = Row::from(&(prev.string.clone() + &current)[..]);
                        self.rows.remove(pos.y);
                        self.move_cursor(Key::Up, &term);
                        self.cursor.x = prev.length();
                        self.recalculate_graphemes();
                    }
                    Event::UpdateLine(pos, _, after) => {
                        self.rows[*pos] = *after.clone();
                        self.snap_cursor(&term);
                        self.prevent_unicode_hell();
                        self.recalculate_graphemes();
                    }
                    _ => (),
                }
                self.dirty = true;
                self.show_welcome = false;
            }
            self.undo_stack.append(events);
        } else {
            self.set_command_line("Empty Redo Stack".to_string(), Type::Error);
        }
    }
    pub fn undo(&mut self, config: &Reader, term: &Size) {
        // Initiate an undo action
        self.undo_stack.commit();
        if let Some(events) = self.undo_stack.pop() {
            for event in &events {
                // Undo the previous action
                match event {
                    // TODO: Update relavent lines here
                    Event::InsertTab(pos) => {
                        for i in 1..=config.general.tab_width {
                            self.rows[pos.y].delete(pos.x - i);
                            self.move_cursor(Key::Left, &term);
                        }
                    }
                    Event::InsertMid(pos, c) => {
                        let c_len = UnicodeWidthChar::width(*c).map_or(0, |c| c);
                        self.cursor.y = pos.y - self.offset.y + OFFSET;
                        self.cursor.x = pos.x.saturating_add(c_len) - self.offset.x;
                        self.recalculate_graphemes();
                        let string = self.rows[pos.y].string.clone();
                        self.rows[pos.y].delete(raw_to_grapheme(pos.x, &string));
                        for _ in 0..c_len {
                            self.move_cursor(Key::Left, term);
                        }
                    }
                    Event::BackspaceMid(pos, c) => {
                        self.rows[pos.y].insert(*c, pos.x);
                        self.move_cursor(Key::Right, term);
                    }
                    Event::ReturnEnd(pos) => {
                        self.rows.remove(pos.y + 1);
                        self.move_cursor(Key::Up, term);
                        self.leap_cursor(Key::End, term);
                    }
                    Event::ReturnStart(pos) => {
                        self.rows.remove(pos.y);
                        self.move_cursor(Key::Up, term);
                    }
                    Event::ReturnMid(pos, breakpoint) => {
                        let current = self.rows[pos.y].string.clone();
                        let after = self.rows[pos.y + 1].string.clone();
                        self.rows.remove(pos.y);
                        self.rows[pos.y] = Row::from(&(current + &after)[..]);
                        self.move_cursor(Key::Up, term);
                        self.leap_cursor(Key::Home, term);
                        for _ in 0..*breakpoint {
                            self.move_cursor(Key::Right, term);
                        }
                    }
                    Event::BackspaceStart(pos) => {
                        let before = Row::from(&self.rows[pos.y].string[..pos.x]);
                        let after = Row::from(&self.rows[pos.y].string[pos.x..]);
                        self.rows[pos.y] = after;
                        self.rows.insert(pos.y, before);
                        self.move_cursor(Key::Down, term);
                        self.leap_cursor(Key::Home, term);
                    }
                    Event::UpdateLine(pos, before, _) => {
                        self.rows[*pos] = *before.clone();
                        self.snap_cursor(term);
                        self.prevent_unicode_hell();
                        self.recalculate_graphemes();
                    }
                    _ => (),
                }
                self.dirty = true;
                self.show_welcome = false;
            }
            self.redo_stack.append(events);
        } else {
            self.set_command_line("Empty Undo Stack".to_string(), Type::Error);
        }
    }
    pub fn save(&self, tab: usize) -> std::io::Result<()> {
        // Save a file
        let contents = self.render(true, tab);
        fs::write(&self.path, contents)
    }
    pub fn save_as(&self, path: &str, tab: usize) -> std::io::Result<()> {
        // Save a file to a specific path
        let contents = self.render(true, tab);
        fs::write(path, contents)
    }
    pub fn scan(&self, needle: &str, offset: usize) -> Vec<Position> {
        // Find all the points where "needle" occurs
        let mut result = vec![];
        if let Ok(re) = Regex::new(needle) {
            for (i, row) in self.rows.iter().enumerate() {
                for o in re.find_iter(&row.string) {
                    result.push(Position {
                        x: o.start(),
                        y: i + offset,
                    });
                }
            }
        }
        result
    }
    pub fn render(&self, replace_tab: bool, tab_width: usize) -> String {
        // Render the lines of a document for writing
        let render = self.rows
            .iter()
            .map(|x| x.string.clone())
            .collect::<Vec<String>>()
            .join("\n")
            + "\n";
        if replace_tab {
            spaces_to_tabs(&render, tab_width)
        } else {
            render
        }
    }
    pub fn identify(path: &str) -> (&str, &str) {
        // Identify which type of file the current buffer is
        match path.split('.').last() {
            Some(ext) => match ext {
                "asm" => ("Assembly ", "\u{f471} "),
                "b" => ("B", "\u{e7a3} "),
                "bf" => ("Brainfuck", "\u{e28c} "),
                "bas" => ("Basic", "\u{e7a3} "),
                "bat" => ("Batch file", "\u{e795} "),
                "bash" => ("Bash", "\u{e795} "),
                "c" => ("C", "\u{e61e} "),
                "cr" => ("Crystal", "\u{e7a3} "),
                "cs" => ("C#", "\u{f81a} "),
                "cpp" => ("C++", "\u{e61d} "),
                "css" => ("CSS", "\u{e749} "),
                "csv" => ("CSV", "\u{f1c0} "),
                "class" | "java" => ("Java", "\u{e738} "),
                "d" => ("D", "\u{e7af} "),
                "db" => ("Database", "\u{f1c0} "),
                "erb" => ("ERB", "\u{e739} "),
                "fish" => ("Fish shell", "\u{f739} "),
                "go" => ("Go", "\u{e724} "),
                "gds" => ("Godot Script", "\u{fba7} "),
                "gitignore" => ("Gitignore", "\u{e702} "),
                "hs" => ("Haskell", "\u{e777} "),
                "html" => ("HTML", "\u{e736} "),
                "js" => ("JavaScript", "\u{e74e} "),
                "json" => ("JSON", "\u{e60b} "),
                "lua" => ("LUA", "\u{e620} "),
                "log" => ("Log file", "\u{f15c} "),
                "md" => ("Markdown", "\u{e73e} "),
                "nim" => ("Nim", "\u{e26e} "),
                "py" | "pyc" | "pyw" => ("Python", "\u{e73c} "),
                "php" => ("PHP", "\u{f81e} "),
                "r" => ("R", "\u{f1c0} "),
                "rs" => ("Rust", "\u{e7a8} "),
                "rb" => ("Ruby", "\u{e739} "),
                "sh" => ("Shell", "\u{e795} "),
                "sql" => ("SQL", "\u{f1c0} "),
                "swift" => ("Swift", "\u{e755} "),
                "sqlite" => ("SQLite", "\u{f1c0} "),
                "txt" => ("Plain Text", "\u{f15c} "),
                "toml" => ("Toml", "\u{f669} "),
                "xml" => ("XML", "\u{f72d} "),
                "vb" => ("VB Script", "\u{4eae}"),
                "vim" => ("VimScript", "\u{e7c5} "),
                "yml" | "yaml" => ("YAML", "\u{e7a3} "),
                "zsh" => ("Z Shell", "\u{e795} "),
                _ => ("Unknown", "\u{f128}"),
            },
            None => ("Unknown", "\u{f128}"),
        }
    }
}
