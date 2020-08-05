use std::str::Chars;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

#[derive(Clone)]
pub struct Row {
    pub string: String,
    pub jumps: Vec<usize>,
}

impl Row {
    pub fn new(row: String) -> Self {
        let mut seg = Vec::new();
        let graphemes = UnicodeSegmentation::graphemes(&row[..], true).collect::<Vec<&str>>();
        for g in graphemes {
            seg.push(UnicodeWidthStr::width(g));
        }
        Self {
            string: row,
            jumps: seg,
        }
    }
    pub fn update_jumps(&mut self) {
        let mut seg = Vec::new();
        let graphemes =
            UnicodeSegmentation::graphemes(&self.string[..], true).collect::<Vec<&str>>();
        for g in graphemes {
            seg.push(UnicodeWidthStr::width(g));
        }
        self.jumps = seg;
    }
    pub fn render(&self) -> String {
        self.string.clone()
    }
    pub fn raw_length(&self) -> usize {
        UnicodeWidthStr::width(&self.string[..])
    }
    pub fn length(&self) -> usize {
        self.jumps.len()
    }
    pub fn chars(&self) -> Chars {
        self.string.chars()
    }
}
