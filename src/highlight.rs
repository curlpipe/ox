// Highlight.rs - For syntax highlighting
use crate::util::Exp;
use std::collections::HashMap;
use termion::color;
use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug, Clone)]
pub struct Token {
    pub span: (usize, usize),
    pub data: String,
    pub kind: String,
}

pub fn cine(token: &Token, hashmap: &mut HashMap<usize, Token>) {
    hashmap.insert((token.clone()).span.0, token.clone());
}

fn bounds(reg: &regex::Match, line: &str) -> (usize, usize) {
    // Work out the width of the capture
    let unicode_width = reg.as_str().graphemes(true).count();
    let pre_length = line[..reg.start()].graphemes(true).count();
    // Calculate the correct boundaries for syntax highlighting
    (pre_length, pre_length + unicode_width)
}

pub fn highlight(row: &str, regexes: &Exp) -> HashMap<usize, Token> {
    let mut syntax: HashMap<usize, Token> = HashMap::new();
    // For digits
    for cap in regexes.digits.captures_iter(row) {
        let cap = cap.get(cap.len().saturating_sub(1)).unwrap();
        let boundaries = bounds(&cap, &row);
        cine(
            &Token {
                span: boundaries,
                data: cap.as_str().to_string(),
                kind: color::Fg(color::Rgb(40, 198, 232)).to_string(),
            },
            &mut syntax,
        );
    }
    // For strings
    for cap in regexes.strings.captures_iter(row) {
        let cap = cap.get(cap.len().saturating_sub(1)).unwrap();
        let boundaries = bounds(&cap, &row);
        cine(
            &Token {
                span: boundaries,
                data: cap.as_str().to_string(),
                kind: color::Fg(color::Rgb(39, 222, 145)).to_string(),
            },
            &mut syntax,
        );
    }
    syntax
}
