// Document.rs - For managing external files
use crate::config::{LINE_NUMBER_PADDING, TAB_WIDTH}; // Config stuff
use crate::{Event, EventStack, Position, Row}; // The Row and Position struct
use std::fs;
use unicode_width::UnicodeWidthChar; // For getting the length of unicode chars // For managing file reading and writing

// Document struct (class) to manage files and text
pub struct Document {
    pub rows: Vec<Row>,          // For holding the contents of the document
    pub path: String,            // For holding the path to the document
    pub name: String,            // For holding the name of the document
    pub line_offset: usize,      // For holding a line number offset
    pub event_stack: EventStack, // For holding the event stack
}

// Add methods to the document struct
impl Document {
    pub fn new() -> Self {
        // Create a new, empty document
        Self {
            rows: vec![Row::from("")],
            name: String::from("[No name]"),
            path: String::new(),
            line_offset: 2,
            event_stack: EventStack::new(),
        }
    }
    pub fn open(path: &str) -> Option<Self> {
        // Create a new document from a path
        if let Ok(file) = fs::read_to_string(path) {
            // File exists
            let mut file = file.split('\n').collect::<Vec<&str>>();
            file.pop();
            Some(Self {
                rows: file.iter().map(|row| Row::from(*row)).collect(),
                name: path.to_string(),
                path: path.to_string(),
                line_offset: 2,
                event_stack: EventStack::new(),
            })
        } else {
            // File doesn't exist
            None
        }
    }
    pub fn from(path: &str) -> Self {
        // Create a new document from a path with empty document on error
        if let Some(doc) = Document::open(path) {
            doc
        } else {
            // Create blank document
            Self {
                rows: vec![Row::from("")],
                name: path.to_string(),
                path: path.to_string(),
                line_offset: 2,
                event_stack: EventStack::new(),
            }
        }
    }
    pub fn recalculate_offset(&mut self) {
        self.line_offset = self.rows.len().to_string().len() + LINE_NUMBER_PADDING;
    }
    pub fn save(&self) -> std::io::Result<()> {
        // Save a file
        fs::write(&self.path, self.render())
    }
    pub fn save_as(&self, path: &str) -> std::io::Result<()> {
        // Save a file to a specific path
        fs::write(path, self.render())
    }
    pub fn scan(&self, needle: &str) -> Vec<Position> {
        // Find all the points where "needle" occurs
        let mut result = vec![];
        for (i, row) in self.rows.iter().enumerate() {
            for o in row.string.match_indices(needle).collect::<Vec<_>>() {
                result.push(Position { x: o.0, y: i });
            }
        }
        result
    }
    pub fn register_event(&mut self, event: Event) -> Position {
        self.event_stack.push(event);
        self.do_event(event)
    }
    pub fn do_event(&mut self, event: Event) -> Position {
        match event {
            Event::Insert(pos, graphemes, c, _) => {
                match c {
                    '\n' => {
                        if pos.x == 0 {
                            // Return key pressed at the start of the line
                            self.rows.insert(pos.y, Row::from(""));
                            Position {
                                x: pos.x,
                                y: pos.y.saturating_add(1),
                            }
                        } else if pos.x == self.rows[pos.y].length() {
                            // Return key pressed at the end of the line
                            self.rows.insert(pos.y + 1, Row::from(""));
                            Position {
                                x: 0,
                                y: pos.y.saturating_add(1),
                            }
                        } else {
                            // Return key pressed in the middle of the line
                            let current = self.rows[pos.y].chars();
                            let before = Row::from(&current[..graphemes].join("")[..]);
                            let after = Row::from(&current[graphemes..].join("")[..]);
                            self.rows.insert(pos.y + 1, after);
                            self.rows[pos.y] = before;
                            Position {
                                x: 0,
                                y: pos.y.saturating_add(1),
                            }
                        }
                    }
                    '\t' => {
                        // Tab key
                        for i in 0..TAB_WIDTH {
                            self.do_event(Event::Insert(
                                Position {
                                    x: pos.x + i,
                                    y: pos.y,
                                },
                                graphemes,
                                ' ',
                                pos.x + i,
                            ));
                        }
                        Position {
                            x: pos.x + TAB_WIDTH,
                            y: pos.y,
                        }
                    }
                    _ => {
                        self.rows[pos.y].insert(c, graphemes);
                        Position {
                            x: pos
                                .x
                                .saturating_add(if let Some(i) = UnicodeWidthChar::width(c) {
                                    i
                                } else {
                                    0
                                }),
                            y: pos.y,
                        }
                    }
                }
            }
            Event::Delete(pos, graphemes, c, px) => {
                if pos.x == 0 && pos.y != 0 {
                    // Backspace at the start of a line
                    let current = self.rows[pos.y].string.clone();
                    let prev = self.rows[pos.y - 1].clone();
                    self.rows[pos.y - 1] = Row::from(&(prev.string.clone() + &current)[..]);
                    self.rows.remove(pos.y);
                    Position {
                        x: prev.length(),
                        y: pos.y.saturating_sub(1),
                    }
                } else if pos.y + pos.x != 0 {
                    // Backspace in the middle of a line
                    self.rows[pos.y].delete(graphemes - 1);
                    Position {
                        x: pos
                            .x
                            .saturating_sub(if let Some(i) = UnicodeWidthChar::width(c) {
                                i
                            } else {
                                0
                            }),
                        y: pos.y,
                    }
                } else {
                    pos
                }
            }
        }
    }
    pub fn reverse(&mut self, event: Event) -> Position {
        match event {
            Event::Insert(pos, graphemes, c, _) => {
                match c {
                    '\n' => {
                        if pos.x == 0 {
                            // Return key pressed at the start of the line
                            // CHECK
                            self.rows.remove(pos.y);
                        } else {
                            // Return key pressed at the end of the line
                            // CHECK
                            let current = self.rows[pos.y + 1].string.clone();
                            let before = self.rows[pos.y].string.clone();
                            self.rows[pos.y] = Row::from(&(before + &current)[..]);
                            self.rows.remove(pos.y + 1);
                        }
                    }
                    '\t' => {
                        // Tab key
                        // CHECK
                        for i in 1..=TAB_WIDTH {
                            self.rows[pos.y].delete(graphemes + TAB_WIDTH.saturating_sub(i));
                        }
                    }
                    _ => {
                        // CHECK
                        self.rows[pos.y].delete(graphemes);
                    }
                }
                pos
            }
            Event::Delete(pos, graphemes, c, px) => {
                if pos.x == 0 && pos.y != 0 {
                    // Backspace at the start of a line
                    let current = self.rows[pos.y - 1].string.clone();
                    let before = Row::from(&current[..px]);
                    let after = Row::from(&current[px..]);
                    self.rows.insert(pos.y, after);
                    self.rows[pos.y - 1] = before;
                } else {
                    // Backspace in the middle of a line
                    // CHECK
                    self.rows[pos.y].insert(c, graphemes - 1);
                }
                pos
            }
        }
    }
    fn render(&self) -> String {
        // Render the lines of a document for writing
        self.rows
            .iter()
            .map(|x| x.string.clone())
            .collect::<Vec<String>>()
            .join("\n")
            + "\n"
    }
    pub fn identify(&self) -> &str {
        // Identify which type of file the current buffer is
        let extension = self.name.split('.').last();
        match extension.unwrap() {
            "asm" => "Assembly",
            "b" => "B",
            "bf" => "Brainfuck",
            "bas" => "Basic",
            "bat" => "Batch file",
            "bash" => "Bash",
            "c" => "C",
            "cr" => "Crystal",
            "cs" => "C#",
            "cpp" => "C++",
            "css" => "CSS",
            "csv" => "CSV",
            "class" | "java" => "Java",
            "d" => "D",
            "db" => "Database",
            "erb" => "ERB",
            "fish" => "Fish shell",
            "go" => "Go",
            "gds" => "Godot Script",
            "gitignore" => "Gitignore",
            "hs" => "Haskell",
            "html" => "HTML",
            "js" => "JavaScript",
            "json" => "JSON",
            "lua" => "LUA",
            "log" => "Log file",
            "md" => "Markdown",
            "nim" => "Nim",
            "py" | "pyc" => "Python",
            "php" => "PHP",
            "r" => "R",
            "rs" => "Rust",
            "rb" => "Ruby",
            "sh" => "Shell",
            "sql" => "SQL",
            "swift" => "Swift",
            "sqlite" => "SQLite",
            "txt" => "Plain Text",
            "toml" => "Toml",
            "xml" => "XML",
            "vb" => "VB Script",
            "vim" => "VimScript",
            "yml" | "yaml" => "YAML",
            "zsh" => "Z Shell",
            _ => "Unknown",
        }
    }
}
