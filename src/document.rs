// Document.rs - For managing external files
use crate::config::{Reader, Status, TokenType};
use crate::editor::OFFSET;
use crate::util::{line_offset, spaces_to_tabs, tabs_to_spaces};
use crate::{Event, EventStack, Position, Row, Size, VERSION};
use regex::Regex;
use std::ffi::OsStr;
use std::path::Path;
use std::{cmp, fs};
use termion::event::Key;
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
                name: Path::new(path)
                    .file_name()
                    .unwrap_or(OsStr::new(path))
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
                &format!("{}", self.cursor.y + self.offset.y - OFFSET + 1),
            )
            .replace("%L", &format!("{}", self.rows.len()))
            .replace("%x", &format!("{}", self.cursor.x + self.offset.x))
            .replace("%y", &format!("{}", self.cursor.y + self.offset.y))
            .replace("%v", VERSION)
            .replace("%d", if self.dirty { "[+]" } else { "" })
            .replace("%D", if self.dirty { "\u{fb12} " } else { "\u{f723} " })
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
    pub fn tab(&mut self, pos: &Position, config: &Reader, term: &Size) {
        // Insert a tab
        for _ in 0..config.general.tab_width {
            self.rows[pos.y].insert(' ', pos.x);
            self.move_cursor(Key::Right, term);
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
        match event {
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
                if !reversed {
                    self.undo_stack.push(event);
                }
            }
            Event::Insertion(mut pos, ch) => {
                self.dirty = true;
                self.rows[pos.y].insert(ch, pos.x);
                self.move_cursor(Key::Right, term);
                pos.x = pos.x.saturating_add(1);
                self.goto(pos, term);
                if !reversed {
                    self.undo_stack.push(event);
                    if ch == ' ' {
                        self.undo_stack.commit();
                    }
                }
            }
            Event::Deletion(mut pos, _ch) => {
                self.dirty = true;
                self.show_welcome = false;
                if reversed {
                    pos.x = pos.x.saturating_sub(1);
                } else {
                    self.undo_stack.push(event);
                }
                self.goto(pos, term);
                self.rows[pos.y].delete(pos.x);
            }
            Event::InsertLineAbove(pos) => {
                self.dirty = true;
                self.rows.insert(pos.y, Row::from(""));
                self.goto(pos, term);
                self.move_cursor(Key::Down, term);
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
            _ => (),
        }
    }
    pub fn word_left(&mut self, term: &Size) {
        self.move_cursor(Key::Left, term);
        let row = self.rows[self.cursor.y + self.offset.y - OFFSET].clone();
        while self.cursor.x + self.offset.x != 0
            && row.chars()[self.graphemes.saturating_sub(1)] != " "
        {
            self.move_cursor(Key::Left, term);
        }
    }
    pub fn word_right(&mut self, term: &Size) {
        let row = self.rows[self.cursor.y + self.offset.y - OFFSET].clone();
        while self.cursor.x + self.offset.x != row.length() && row.chars()[self.graphemes] != " " {
            self.move_cursor(Key::Right, term);
        }
        self.move_cursor(Key::Right, term);
    }
    pub fn goto(&mut self, mut pos: Position, term: &Size) {
        // Move the cursor to a specific location
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
        let render = self
            .rows
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
