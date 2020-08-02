// Buffer.rs - For managing buffers
use std::fs;

pub struct Buffer {
    pub lines: Vec<String>,
    pub path: String,
    pub filename: String,
}

impl Buffer {
    pub fn new() -> Self {
        Buffer {
            lines: vec![String::new()],
            path: String::new(),
            filename: String::from("[No name]"),
        }
    }
    pub fn open(path: &str) -> Option<Self> {
        if let Ok(file) = fs::read_to_string(path) {
            let mut lines = Vec::new();
            for line in file.split('\n') {
                lines.push(line.to_string());
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
        fs::write(&self.path, self.lines.join("\n")).unwrap();
    }
}
