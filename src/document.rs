// Document.rs - For managing external files
use crate::config::{Reader, Status, TokenType};
use crate::editor::OFFSET;
use crate::util::{line_offset, spaces_to_tabs, tabs_to_spaces};
use crate::{log, Editor, Event, EventStack, Position, Row, Size, Variable, VERSION};
use crossterm::event::KeyCode as Key;
use regex::Regex;
use std::ffi::OsStr;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::{cmp, fs};
use unicode_width::UnicodeWidthStr;

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

// Enum to determine which tab type
#[derive(Debug, Copy, Clone)]
pub enum TabType {
    Spaces,
    Tabs,
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
    pub tabs: TabType,          // For detecting if tabs are used over spaces
    pub last_save_index: usize, // For holding the last save index
    pub true_path: String,      // For holding the path that was provided as argument
    pub read_only: bool,        // Boolean to determine if the document is read only
}

// Add methods to the document struct
impl Document {
    pub fn new(config: &Reader, status: &Status, read_only: bool) -> Self {
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
            tabs: TabType::Spaces,
            last_save_index: 0,
            true_path: String::new(),
            read_only,
        }
    }
    pub fn open(config: &Reader, status: &Status, path: &str, read_only: bool) -> Option<Self> {
        // Create a new document from a path
        let true_path = path.to_string();
        let path = path.split(':').next().unwrap();
        if let Ok(file) = fs::read_to_string(path) {
            // File exists
            let tabs = file.contains("\n\t");
            let file = tabs_to_spaces(&file, config.general.tab_width);
            let mut file = Document::split_file(&file);
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
                name: Path::new(path)
                    .file_name()
                    .unwrap_or_else(|| OsStr::new(path))
                    .to_str()
                    .unwrap_or(&path)
                    .to_string(),
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
                tabs: if tabs { TabType::Tabs } else { TabType::Spaces },
                last_save_index: 0,
                true_path,
                read_only,
            })
        } else {
            // File doesn't exist
            None
        }
    }
    pub fn from(config: &Reader, status: &Status, path: &str, read_only: bool) -> Self {
        // Create a new document from a path with empty document on error
        let true_path = path.to_string();
        let path = path.split(':').next().unwrap();
        if let Some(doc) = Document::open(&config, &status, &true_path, read_only) {
            log!("Opening file", "File was found");
            doc
        } else {
            // Create blank document
            log!("Opening file", "File not found");
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
                tabs: TabType::Spaces,
                last_save_index: 0,
                true_path,
                read_only,
            }
        }
    }
    pub fn split_file(contents: &str) -> Vec<&str> {
        // Detect DOS line ending
        let splitter = Regex::new("(?ms)(\r\n|\n)").unwrap();
        splitter.split(contents).collect()
    }
    pub fn correct_path(&mut self, term: &Size) {
        if self.true_path.contains(':') {
            let split: Vec<&str> = self.true_path.split(':').collect();
            let mut y = split.get(1).unwrap_or(&"").parse().unwrap_or(0);
            let mut x = split.get(2).unwrap_or(&"").parse().unwrap_or(0);
            if y >= self.rows.len() {
                self.set_command_line(format!("Row {} out of scope", y), Type::Warning);
                y = self.rows.len();
            }
            if x >= self.rows[y.saturating_sub(1)].length() {
                self.set_command_line(format!("Column {} out of scope", x), Type::Warning);
                x = self.rows[y.saturating_sub(1)].length();
            }
            self.goto(
                Position {
                    x,
                    y: y.saturating_sub(1),
                },
                term,
            );
        }
    }
    pub fn set_command_line(&mut self, text: String, msg: Type) {
        // Function to update the command line
        self.cmd_line = CommandLine { text, msg };
    }
    pub fn mass_redraw(&mut self) {
        for i in &mut self.rows {
            i.updated = true;
        }
    }
    pub fn config_to_commandline(status: &Status) -> CommandLine {
        CommandLine {
            text: match status {
                Status::Success => "Welcome to Ox".to_string(),
                Status::File => "Config file not found, using default values".to_string(),
                Status::Parse(error) => format!("Failed to parse: {:?}", error),
                Status::Empty => "Config file is empty, using defaults".to_string(),
            },
            msg: match status {
                Status::Success | Status::Empty => Type::Info,
                Status::File => Type::Warning,
                Status::Parse(_) => Type::Error,
            },
        }
    }
    pub fn format(&self, template: &str) -> String {
        // Form data from a template
        template
            .replace("%f", &self.name)
            .replace("%F", &self.path)
            .replace("%i", &self.icon)
            .replace(
                "%I",
                &if self.icon.is_empty() {
                    String::new()
                } else {
                    format!("{} ", self.icon)
                },
            )
            .replace("%n", &self.kind)
            .replace(
                "%l",
                &format!("{}", self.cursor.y + self.offset.y.saturating_sub(OFFSET)),
            )
            .replace("%L", &format!("{}", self.rows.len()))
            .replace("%x", &format!("{}", self.cursor.x + self.offset.x))
            .replace("%y", &format!("{}", self.cursor.y + self.offset.y))
            .replace("%v", VERSION)
            .replace("%d", if self.dirty { "[+]" } else { "" })
            .replace("%D", if self.dirty { "\u{fb12} " } else { "\u{f723} " })
    }
    pub fn move_cursor(&mut self, direction: Key, term: &Size, wrap: bool) {
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
                let line = &self.rows[self.cursor.y + self.offset.y - OFFSET].clone();
                // Check for line wrapping
                if line.length() == self.cursor.x + self.offset.x
                    && self.cursor.y + self.offset.y - OFFSET != self.rows.len().saturating_sub(1)
                    && wrap
                {
                    self.move_cursor(Key::Down, term, wrap);
                    self.leap_cursor(Key::Home, term);
                    return;
                }
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
                let line = &self.rows[self.cursor.y + self.offset.y - OFFSET].clone();
                if self.cursor.x + self.offset.x == 0
                    && self.cursor.y + self.offset.y - OFFSET != 0
                    && wrap
                {
                    self.move_cursor(Key::Up, term, wrap);
                    self.leap_cursor(Key::End, term);
                    return;
                }
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
    pub fn tab(&mut self, pos: &Position, config: &Reader, term: &Size) {
        // Insert a tab
        for _ in 0..config.general.tab_width {
            self.rows[pos.y].insert(' ', pos.x);
            self.move_cursor(Key::Right, term, config.general.wrap_cursor);
        }
    }
    fn overwrite(&mut self, after: &[Row]) {
        // Override the entire contents of the document
        self.dirty = true;
        self.rows = after.to_vec();
    }
    fn update_line(&mut self, pos: &Position, after: Row, offset: i128) -> usize {
        // Update a line in the document
        self.dirty = true;
        let ind = line_offset(pos.y, offset, self.rows.len());
        self.rows[ind] = after;
        ind
    }
    fn delete_line(&mut self, pos: &Position, offset: i128) {
        // Delete a line in the document
        self.dirty = true;
        let ind = line_offset(pos.y, offset, self.rows.len());
        if self.rows.len() > 1 {
            self.rows.remove(ind);
        }
    }
    fn splice_up(&mut self, pos: &Position, reversed: bool, term: &Size, other: &Position) {
        // Splice the line up to the next
        self.dirty = true;
        let above = self.rows[pos.y.saturating_sub(1)].clone();
        let current = self.rows[pos.y].clone();
        let new = format!("{}{}", above.string, current.string);
        self.rows[pos.y.saturating_sub(1)] = Row::from(&new[..]);
        self.rows.remove(pos.y);
        if reversed {
            self.goto(*other, term);
        } else {
            let other = Position {
                x: above.length(),
                y: pos.y.saturating_sub(1),
            };
            self.goto(
                Position {
                    x: other.x,
                    y: other.y,
                },
                term,
            );
            self.undo_stack.push(Event::SpliceUp(*pos, other));
            self.undo_stack.commit();
        }
    }
    fn split_down(&mut self, pos: &Position, reversed: bool, term: &Size, other: &Position) {
        // Split the line in half
        self.dirty = true;
        let current = self.rows[pos.y].clone();
        let left: String = current.string.chars().take(pos.x).collect();
        let right: String = current.string.chars().skip(pos.x).collect();
        self.rows[pos.y] = Row::from(&left[..]);
        self.rows
            .insert(pos.y.saturating_add(1), Row::from(&right[..]));
        if reversed {
            self.goto(*other, term);
        } else {
            let other = Position {
                x: 0,
                y: pos.y.saturating_add(1),
            };
            self.goto(other, term);
            self.undo_stack.push(Event::SplitDown(*pos, other));
            self.undo_stack.commit();
        }
    }
    pub fn execute(&mut self, event: Event, reversed: bool, term: &Size, config: &Reader) {
        // Document edit event executor
        if self.read_only && Editor::will_edit(&event) {
            return;
        }
        match event {
            Event::Set(variable, value) => match variable {
                Variable::Saved => self.dirty = !value,
            },
            Event::Overwrite(_, ref after) => {
                self.overwrite(after);
                self.goto(Position { x: 0, y: 0 }, term);
                if !reversed {
                    self.undo_stack.push(event);
                }
            }
            Event::UpdateLine(pos, offset, _, ref after) => {
                let ind = self.update_line(&pos, *after.clone(), offset);
                self.goto(Position { x: pos.x, y: ind }, term);
                if !reversed {
                    self.undo_stack.push(event);
                }
            }
            Event::DeleteLine(pos, offset, _) => {
                self.delete_line(&pos, offset);
                self.goto(pos, term);
                if self.cursor.y + self.offset.y - OFFSET >= self.rows.len() {
                    self.move_cursor(Key::Up, term, config.general.wrap_cursor);
                }
                if !reversed {
                    self.undo_stack.push(event);
                }
            }
            Event::Insertion(pos, ch) => {
                self.dirty = true;
                self.rows[pos.y].insert(ch, pos.x);
                self.move_cursor(Key::Right, term, config.general.wrap_cursor);
                self.goto(pos, term);
                self.move_cursor(Key::Right, term, config.general.wrap_cursor);
                if !reversed {
                    self.undo_stack.push(event);
                    if ch == ' ' {
                        self.undo_stack.commit();
                    }
                }
            }
            Event::Deletion(pos, _) => {
                self.dirty = true;
                self.show_welcome = false;
                self.recalculate_graphemes();
                self.goto(pos, term);
                if reversed {
                    self.move_cursor(Key::Left, term, config.general.wrap_cursor);
                } else {
                    self.undo_stack.push(event);
                }
                self.rows[pos.y].delete(self.graphemes.saturating_sub(1));
            }
            Event::InsertLineAbove(pos) => {
                self.dirty = true;
                self.rows.insert(pos.y, Row::from(""));
                self.goto(pos, term);
                self.move_cursor(Key::Down, term, config.general.wrap_cursor);
                if !reversed {
                    self.undo_stack.push(event);
                    self.undo_stack.commit();
                }
            }
            Event::InsertLineBelow(pos) => {
                self.dirty = true;
                self.rows.insert(pos.y.saturating_add(1), Row::from(""));
                self.goto(pos, term);
                if !reversed {
                    self.undo_stack.push(event);
                    self.undo_stack.commit();
                }
            }
            Event::SpliceUp(pos, other) => self.splice_up(&pos, reversed, term, &other),
            Event::SplitDown(pos, other) => self.split_down(&pos, reversed, term, &other),
            Event::InsertTab(pos) => {
                self.dirty = true;
                self.goto(pos, term);
                self.tab(&pos, &config, term);
                if !reversed {
                    self.undo_stack.push(event);
                }
            }
            Event::DeleteTab(pos) => {
                self.dirty = true;
                self.goto(pos, term);
                for _ in 0..config.general.tab_width {
                    self.rows[pos.y].delete(pos.x);
                }
                if !reversed {
                    self.undo_stack.push(event);
                }
            }
            Event::DeleteWord(pos, _) => self.delete_word(&pos, term),
            _ => (),
        }
        self.recalculate_graphemes();
    }
    pub fn word_left(&mut self, term: &Size) {
        self.move_cursor(Key::Left, term, false);
        let row = self.rows[self.cursor.y + self.offset.y - OFFSET].clone();
        while self.cursor.x + self.offset.x != 0
            && row.chars()[self.graphemes.saturating_sub(1)] != " "
        {
            self.move_cursor(Key::Left, term, false);
        }
    }
    pub fn word_right(&mut self, term: &Size) {
        let row = self.rows[self.cursor.y + self.offset.y - OFFSET].clone();
        while self.cursor.x + self.offset.x != row.length() && row.chars()[self.graphemes] != " " {
            self.move_cursor(Key::Right, term, false);
        }
        self.move_cursor(Key::Right, term, false);
    }
    pub fn find_word_boundary_left(&self, pos: &Position) -> Option<Position> {
        self.find_prev(" ", pos)
    }
    pub fn find_word_boundary_right(&self, pos: &Position) -> Option<Position> {
        self.find_next(" ", pos)
    }
    pub fn delete_word(&mut self, pos: &Position, term: &Size) {
        let mut right = if let Some(&" ") = self.rows[pos.y].ext_chars().get(pos.x) {
            *pos
        } else {
            self.find_word_boundary_right(pos)
                .unwrap_or(Position { x: 0, y: pos.y })
        };
        let mut left = self.find_word_boundary_left(pos).unwrap_or(Position {
            x: self.rows[pos.y].length(),
            y: pos.y,
        });
        if right.y != pos.y {
            right = Position {
                x: self.rows[pos.y].length(),
                y: pos.y,
            };
        }
        if left.y != pos.y {
            left = Position { x: 0, y: pos.y };
        }
        self.goto(left, term);
        for _ in left.x..right.x {
            self.rows[pos.y].delete(left.x);
        }
    }
    pub fn goto(&mut self, mut pos: Position, term: &Size) {
        // Move the cursor to a specific location
        let on_y = pos.y >= self.offset.y
            && pos.y <= self.offset.y.saturating_add(term.height.saturating_sub(4));
        let on_x = pos.x >= self.offset.x
            && pos.x
                <= self
                    .offset
                    .x
                    .saturating_add(term.width.saturating_sub(self.line_offset));
        // Verify that the goto is necessary
        if on_y && on_x {
            // No need to adjust offset
            self.cursor.y = pos.y - self.offset.y + OFFSET;
            self.cursor.x = pos.x - self.offset.x;
            return;
        }
        // Move to that position
        let max_y = term.height.saturating_sub(3);
        let max_x = (term.width).saturating_sub(self.line_offset);
        let halfway_y = max_y / 2;
        let halfway_x = max_x / 2;
        pos.y = pos.y.saturating_add(1);
        if self.offset.x == 0 && pos.y < max_y && pos.x < max_x {
            // Cursor is on the screen
            self.offset = Position { x: 0, y: 0 };
            self.cursor = pos;
        } else {
            // Cursor is off the screen, move to the Y position
            self.offset.y = pos.y.saturating_sub(halfway_y);
            self.cursor.y = halfway_y;
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
            if self.offset.y + self.cursor.y != pos.y {
                // Fix cursor misplacement
                self.offset.y = 0;
                self.cursor.y = pos.y;
            }
        }
    }
    pub fn save(&self, path: &str, tab: usize) -> std::io::Result<()> {
        // Save a file
        let contents = self.render(self.tabs, tab);
        log!("Saved file", format!("File tab status is {:?}", self.tabs));
        fs::write(path, contents)
    }
    pub fn find_prev(&self, needle: &str, current: &Position) -> Option<Position> {
        // Find all the points where "needle" occurs before the current position
        let re = Regex::new(needle).ok()?;
        for (c, r) in self
            .rows
            .iter()
            .take(current.y.saturating_add(1))
            .map(|x| x.string.as_str())
            .enumerate()
            .rev()
        {
            let mut xs = vec![];
            for i in re.captures_iter(r) {
                for j in 0..i.len() {
                    let j = i.get(j).unwrap();
                    xs.push(j.start());
                }
            }
            while let Some(i) = xs.pop() {
                if i < current.x || c != current.y {
                    return Some(Position { x: i, y: c });
                }
            }
        }
        None
    }
    pub fn find_next(&self, needle: &str, current: &Position) -> Option<Position> {
        // Find all the points where "needle" occurs after the current position
        let re = Regex::new(needle).ok()?;
        for (c, r) in self
            .rows
            .iter()
            .skip(current.y)
            .map(|x| x.string.as_str())
            .enumerate()
        {
            for i in re.captures_iter(r) {
                for cap in 0..i.len() {
                    let cap = i.get(cap).unwrap();
                    if c != 0 || cap.start() > current.x {
                        return Some(Position {
                            x: cap.start(),
                            y: current.y + c,
                        });
                    }
                }
            }
        }
        None
    }
    pub fn find_all(&self, needle: &str) -> Option<Vec<Position>> {
        // Find all the places where the needle is
        let mut result = vec![];
        let re = Regex::new(needle).ok()?;
        for (c, r) in self.rows.iter().map(|x| x.string.to_string()).enumerate() {
            for i in re.captures_iter(&r) {
                for cap in 0..i.len() {
                    let cap = i.get(cap).unwrap();
                    result.push(Position {
                        x: cap.start(),
                        y: c,
                    });
                }
            }
        }
        Some(result)
    }
    pub fn render(&self, tab_type: TabType, tab_width: usize) -> String {
        // Render the lines of a document for writing
        let render = self
            .rows
            .iter()
            .map(|x| x.string.clone())
            .collect::<Vec<String>>()
            .join("\n")
            + "\n";
        if let TabType::Tabs = tab_type {
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
