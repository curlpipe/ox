// Document.rs - For managing external files
use crate::config::LINE_NUMBER_PADDING; // Config stuff
use crate::{Position, Row}; // The Row and Position struct
use std::fs; // For managing file reading and writing

// Document struct (class) to manage files and text
pub struct Document {
    pub rows: Vec<Row>,     // For holding the contents of the document
    pub path: String,       // For holding the path to the document
    pub name: String,       // For holding the name of the document
    pub line_offset: usize, // For holding a line number offset
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
