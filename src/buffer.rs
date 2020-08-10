// Buffer.rs - For managing buffers
use crate::Row;
use std::fs;

pub struct Buffer {
    pub lines: Vec<Row>,
    pub path: String,
    pub filename: String,
    pub line_number_offset: usize,
}

impl Buffer {
    pub fn new() -> Self {
        Self {
            lines: vec![Row::new("".to_string())],
            path: String::new(),
            filename: String::from("[No name]"),
            line_number_offset: 2,
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
                path: String::from(path),
                filename: String::from(path),
                line_number_offset: 2,
            })
        } else {
            None
        }
    }
    pub fn update_line_offset(&mut self) {
        self.line_number_offset = self.lines.len().to_string().len().saturating_add(1);
    }
    pub fn from(path: &str) -> Self {
        Self {
            lines: vec![Row::new("".to_string())],
            path: String::from(path),
            filename: String::from(path),
            line_number_offset: 2,
        }
    }
    pub fn render(&self) -> String {
        let mut result = String::new();
        for row in &self.lines {
            result.push_str(&row.string);
            result.push('\n');
        }
        result
    }
    pub fn save(&self) -> std::io::Result<()> {
        let string = self.render();
        fs::write(&self.path, string)
    }
    pub fn save_as(&self, path: &str) -> std::io::Result<()> {
        let string = self.render();
        fs::write(path, string)
    }
    pub fn identify(&self) -> &str {
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
