// Buffer.rs - For managing buffers
use crate::Row;
use std::fs;

pub struct Buffer {
    pub lines: Vec<Row>,
    pub path: String,
    pub filename: String,
}

impl Buffer {
    pub fn new() -> Self {
        Buffer {
            lines: vec![Row::new(String::new())],
            path: String::new(),
            filename: String::from("[No name]"),
        }
    }
    pub fn open(path: &str) -> Option<Self> {
        if let Ok(file) = fs::read_to_string(path) {
            let mut lines = Vec::new();
            for line in file.split('\n') {
                lines.push(Row::new(line.to_string()));
            }
            Some(Self {
                lines,
                path: path.to_string(),
                filename: String::from(path),
            })
        } else {
            None
        }
    }
    pub fn save(&self) {
        let mut result = String::new();
        for row in &self.lines {
            result.push_str(&row.string);
            result.push('\n');
        }
        fs::write(&self.path, result).unwrap();
    }
    pub fn identify(&self) -> &str {
        let extension = self.filename.split(".").last();
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
            "class" => "Java",
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
            "java" => "Java",
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
