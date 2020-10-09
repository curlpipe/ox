// Highlight.rs - For syntax highlighting
use crate::util::Exp;
use std::collections::HashMap;
use termion::color;

#[derive(Debug, Clone)]
pub struct Token {
    pub span: (usize, usize),
    pub data: String,
    pub kind: String,
}

pub fn cine(token: &Token, hashmap: &mut HashMap<usize, Token>) {
    hashmap.insert((token.clone()).span.0, token.clone());
}

pub fn highlight(row: &str, regexes: &Exp) -> HashMap<usize, Token> {
    let mut syntax: HashMap<usize, Token> = HashMap::new();
    // For digits
    for cap in regexes.digits.captures_iter(row) {
        let cap = cap.get(cap.len().saturating_sub(1)).unwrap();
        cine(&Token {
            span: (cap.start(), cap.end()),
            data: cap.as_str().to_string(),
            kind: color::Fg(color::Rgb(40, 198, 232)).to_string(),
        }, &mut syntax);
    }
    // For strings
    for cap in regexes.strings.captures_iter(row) {
        let cap = cap.get(cap.len().saturating_sub(1)).unwrap();
        cine(&Token {
            span: (cap.start(), cap.end()),
            data: cap.as_str().to_string(),
            kind: color::Fg(color::Rgb(39, 222, 145)).to_string(),
        }, &mut syntax);
    }
    syntax
}
