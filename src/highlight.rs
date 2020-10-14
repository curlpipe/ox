// Highlight.rs - For syntax highlighting
use crate::editor::RESET_FG;
use crate::util::Exp;
use std::collections::HashMap;
use termion::color;
use unicode_width::UnicodeWidthStr;

#[derive(Debug, Clone)]
pub struct Token {
    pub span: (usize, usize),
    pub data: String,
    pub kind: String,
}

impl Token {
    pub fn colorize(&self) -> String {
        format!("{}{}{}", self.kind, self.data, RESET_FG)
    }
}

pub fn cine(token: &Token, hashmap: &mut HashMap<usize, Token>) {
    hashmap.insert((token.clone()).span.0, token.clone());
}

fn bounds(reg: &regex::Match, line: &str) -> (usize, usize) {
    // Work out the width of the capture
    let unicode_width = UnicodeWidthStr::width(reg.as_str());
    let pre_length = UnicodeWidthStr::width(&line[..reg.start()]);
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
    // For characters
    for cap in regexes.characters.captures_iter(row) {
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
    // For function identifiers
    for cap in regexes.functions.captures_iter(row) {
        let cap = cap.get(cap.len().saturating_sub(1)).unwrap();
        let boundaries = bounds(&cap, &row);
        cine(
            &Token {
                span: boundaries,
                data: cap.as_str().to_string(),
                kind: color::Fg(color::Rgb(47, 141, 252)).to_string(),
            },
            &mut syntax,
        );
    }
    // For attributes
    for cap in regexes.attributes.captures_iter(row) {
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
    // For booleans
    for cap in regexes.booleans.captures_iter(row) {
        let cap = cap.get(cap.len().saturating_sub(1)).unwrap();
        let boundaries = bounds(&cap, &row);
        cine(
            &Token {
                span: boundaries,
                data: cap.as_str().to_string(),
                kind: color::Fg(color::Rgb(86, 217, 178)).to_string(),
            },
            &mut syntax,
        );
    }
    // For struct identifiers
    for cap in regexes.structs.captures_iter(row) {
        let cap = cap.get(cap.len().saturating_sub(1)).unwrap();
        let boundaries = bounds(&cap, &row);
        cine(
            &Token {
                span: boundaries,
                data: cap.as_str().to_string(),
                kind: color::Fg(color::Rgb(47, 141, 252)).to_string(),
            },
            &mut syntax,
        );
    }
    // For macros
    for cap in regexes.macros.captures_iter(row) {
        let cap = cap.get(cap.len().saturating_sub(1)).unwrap();
        let boundaries = bounds(&cap, &row);
        cine(
            &Token {
                span: boundaries,
                data: cap.as_str().to_string(),
                kind: color::Fg(color::Rgb(223, 52, 249)).to_string(),
            },
            &mut syntax,
        );
    }
    // For single line comments
    for cap in regexes.single_comments.captures_iter(row) {
        let cap = cap.get(cap.len().saturating_sub(1)).unwrap();
        let boundaries = bounds(&cap, &row);
        cine(
            &Token {
                span: boundaries,
                data: cap.as_str().to_string(),
                kind: color::Fg(color::Rgb(113, 113, 169)).to_string(),
            },
            &mut syntax,
        );
    }
    // For keywords
    for kw in &regexes.keywords {
        for cap in kw.captures_iter(row) {
            let cap = cap.get(cap.len().saturating_sub(1)).unwrap();
            let boundaries = bounds(&cap, &row);
            cine(
                &Token {
                    span: boundaries,
                    data: cap.as_str().to_string(),
                    kind: color::Fg(color::Rgb(134, 76, 232)).to_string(),
                },
                &mut syntax,
            );
        }
    }
    syntax
}

pub fn remove_nested_tokens(tokens: HashMap<usize, Token>, line: &str) -> HashMap<usize, Token> {
    let mut result = HashMap::new();
    let mut c = 0;
    while c < line.len() {
        if let Some(t) = tokens.get(&c) {
            result.insert(t.span.0, t.clone());
            c += t.span.1 - t.span.0;
        } else {
            c += 1;
        }
    }
    result
}
