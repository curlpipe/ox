// Document.rs - For managing external files
use crate::config::Reader; // Config stuff
use crate::{EventStack, Position, Row}; // The Row and Position struct
use regex::Regex; // For searching and replacing
use std::collections::HashMap; // For hashmaps
use std::fs; // For managing file reading and writing

// Document struct (class) to manage files and text
pub struct Document {
    pub rows: Vec<Row>,         // For holding the contents of the document
    pub path: String,           // For holding the path to the document
    pub name: String,           // For holding the name of the document
    pub line_offset: usize,     // For holding a line number offset
    pub undo_stack: EventStack, // For holding the undo event stack
    pub redo_stack: EventStack, // For holding the redo event stack
    pub reg: Option<HashMap<String, Vec<Regex>>>, // For holding the regex of syntax
}

// Add methods to the document struct
impl Document {
    pub fn new(config: &Reader) -> Self {
        // Create a new, empty document
        Self {
            rows: vec![Row::from("")],
            name: String::from("[No name]"),
            path: String::new(),
            line_offset: config.general.line_number_padding_right
                + config.general.line_number_padding_left,
            undo_stack: EventStack::new(),
            redo_stack: EventStack::new(),
            reg: Reader::get_syntax_regex(&config, ""),
        }
    }
    pub fn open(config: &Reader, path: &str) -> Option<Self> {
        // Create a new document from a path
        if let Ok(file) = fs::read_to_string(path) {
            // File exists
            let mut file = file.split('\n').collect::<Vec<&str>>();
            if let Some(line) = file.iter().last() {
                if line.is_empty() {
                    let _ = file.pop();
                }
            }
            Some(Self {
                rows: file.iter().map(|row| Row::from(*row)).collect(),
                name: path.to_string(),
                path: path.to_string(),
                line_offset: config.general.line_number_padding_right
                    + config.general.line_number_padding_left,
                undo_stack: EventStack::new(),
                redo_stack: EventStack::new(),
                reg: Reader::get_syntax_regex(&config, path.split('.').last().unwrap_or("")),
            })
        } else {
            // File doesn't exist
            None
        }
    }
    pub fn from(config: &Reader, path: &str) -> Self {
        // Create a new document from a path with empty document on error
        if let Some(doc) = Document::open(&config, path) {
            doc
        } else {
            // Create blank document
            Self {
                rows: vec![Row::from("")],
                name: path.to_string(),
                path: path.to_string(),
                line_offset: config.general.line_number_padding_right
                    + config.general.line_number_padding_left,
                undo_stack: EventStack::new(),
                redo_stack: EventStack::new(),
                reg: Reader::get_syntax_regex(&config, path.split('.').last().unwrap_or("")),
            }
        }
    }
    pub fn recalculate_offset(&mut self, config: &Reader) {
        self.line_offset = self.rows.len().to_string().len()
            + config.general.line_number_padding_right
            + config.general.line_number_padding_left;
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
        if let Ok(re) = Regex::new(needle) {
            for (i, row) in self.rows.iter().enumerate() {
                for o in re.find_iter(&row.string) {
                    result.push(Position { x: o.start(), y: i });
                }
            }
        }
        result
    }
    pub fn render(&self) -> String {
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
        match extension {
            Some(ext) => match ext {
                "asm" => "Assembly \u{f471} ",
                "b" => "B \u{e7a3} ",
                "bf" => "Brainfuck \u{e28c} ",
                "bas" => "Basic \u{e7a3} ",
                "bat" => "Batch file \u{e795} ",
                "bash" => "Bash \u{e795} ",
                "c" => "C \u{e61e} ",
                "cr" => "Crystal \u{e7a3} ",
                "cs" => "C# \u{f81a} ",
                "cpp" => "C++ \u{e61d} ",
                "css" => "CSS \u{e749} ",
                "csv" => "CSV \u{f1c0} ",
                "class" | "java" => "Java \u{e738} ",
                "d" => "D \u{e7af} ",
                "db" => "Database \u{f1c0} ",
                "erb" => "ERB \u{e739} ",
                "fish" => "Fish shell \u{f739} ",
                "go" => "Go \u{e724} ",
                "gds" => "Godot Script \u{fba7} ",
                "gitignore" => "Gitignore \u{e702} ",
                "hs" => "Haskell \u{e777} ",
                "html" => "HTML \u{e736} ",
                "js" => "JavaScript \u{e74e} ",
                "json" => "JSON \u{e60b} ",
                "lua" => "LUA \u{e620} ",
                "log" => "Log file \u{f15c} ",
                "md" => "Markdown \u{e73e} ",
                "nim" => "Nim \u{e26e} ",
                "py" | "pyc" => "Python \u{e73c} ",
                "php" => "PHP \u{f81e} ",
                "r" => "R \u{f1c0} ",
                "rs" => "Rust \u{e7a8} ",
                "rb" => "Ruby \u{e739} ",
                "sh" => "Shell \u{e795} ",
                "sql" => "SQL \u{f1c0} ",
                "swift" => "Swift \u{e755} ",
                "sqlite" => "SQLite \u{f1c0} ",
                "txt" => "Plain Text \u{f15c} ",
                "toml" => "Toml \u{f669} ",
                "xml" => "XML \u{f72d} ",
                "vb" => "VB Script \u{4eae}",
                "vim" => "VimScript \u{e7c5} ",
                "yml" | "yaml" => "YAML \u{e7a3} ",
                "zsh" => "Z Shell \u{e795} ",
                _ => "Unknown \u{f128}",
            },
            None => "Unknown \u{f128}",
        }
    }
}
