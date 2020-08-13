/*
    Buffer.rs - For managing buffers

    This is where external files are managed
    as well as the line numbers.
*/

use crate::Row; // Bring in the Row struct
use std::fs; // For reading and writing external files

// Set up a struct for the buffer
pub struct Buffer {
    pub lines: Vec<Row>,           // For storing the lines
    pub path: String,              // For storing the file path
    pub filename: String,          // For storing the file name
    pub line_number_offset: usize, // For storing the line number offset
}

// Add methods to the buffer struct
impl Buffer {
    pub fn new() -> Self {
        // Create a new empty buffer
        Self {
            lines: vec![Row::new("".to_string())],
            path: String::new(),
            filename: String::from("[No name]"),
            line_number_offset: 2,
        }
    }
    pub fn open(path: &str) -> Option<Self> {
        // Open an external file in a buffer with a result
        if let Ok(file) = fs::read_to_string(path) {
            let mut lines = Vec::new();
            for line in file.split('\n') {
                lines.push(Row::new(line.to_string()));
            }
            Some(Self {
                lines,
                path: String::from(path),
                filename: String::from(path),
                line_number_offset: 2,
            })
        } else {
            None
        }
    }
    pub fn from(path: &str) -> Self {
        // Open an external file regardless of whether it exists
        Self {
            lines: vec![Row::new("".to_string())],
            path: String::from(path),
            filename: String::from(path),
            line_number_offset: 2,
        }
    }
    pub fn update_line_offset(&mut self) {
        // Update the offset caused by the line numbers
        self.line_number_offset = self.lines.len().to_string().len().saturating_add(1);
    }
    pub fn render(&self) -> String {
        // Turn the buffer into a string for writing to a file
        let mut result = String::new();
        for row in &self.lines {
            result.push_str(&row.string);
            result.push('\n');
        }
        result
    }
    pub fn save(&self) -> std::io::Result<()> {
        // Save a file
        let string = self.render();
        fs::write(&self.path, string)
    }
    pub fn save_as(&self, path: &str) -> std::io::Result<()> {
        // Save a file to a specific path
        let string = self.render();
        fs::write(path, string)
    }
    pub fn identify(&self) -> &str {
        // Identify which type of file the current buffer is
        let extension = self.filename.split('.').last();
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
