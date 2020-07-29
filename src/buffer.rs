// Buffer.rs - For managing buffers
use std::fs;

pub struct Buffer {
    pub lines: Vec<String>,
    pub offset: u16,
}

impl Buffer {
    pub fn new() -> Self {
        Buffer {
            lines: Vec::new(),
            offset: 0,
        }
    }
    pub fn open(path: &str) -> Self {
        let file = fs::read_to_string(path).unwrap();
        let mut lines = Vec::new();
        for line in file.split('\n') {
            lines.push(line.to_string());
        }
        Self { 
            offset: 0,
            lines,
        }
    }
}
