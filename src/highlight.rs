// Highlight.rs - For syntax highlighting
use crate::config::Reader;
use regex::Regex;
use std::collections::HashMap;
use unicode_width::UnicodeWidthStr;

// Tokens for storing syntax highlighting info
#[derive(Debug, Clone)]
pub struct Token {
    pub span: (usize, usize),
    pub data: String,
    pub kind: String,
}

pub fn cine(token: &Token, hashmap: &mut HashMap<usize, Token>) {
    // Insert a token into a hashmap
    hashmap.insert((token.clone()).span.0, token.clone());
}

fn bounds(reg: &regex::Match, line: &str) -> (usize, usize) {
    // Work out the width of the capture
    let unicode_width = UnicodeWidthStr::width(reg.as_str());
    let pre_length = UnicodeWidthStr::width(&line[..reg.start()]);
    // Calculate the correct boundaries for syntax highlighting
    (pre_length, pre_length + unicode_width)
}

pub fn highlight(
    row: &str,
    regex: &HashMap<String, Vec<Regex>>,
    highlights: &HashMap<String, (u8, u8, u8)>,
) -> HashMap<usize, Token> {
    // Generate syntax highlighting information
    let mut syntax: HashMap<usize, Token> = HashMap::new();
    if regex.is_empty() {
        // Language not found, return empty hashmap
        return syntax;
    }
    for kw in &regex["keywords"] {
        // Locate keywords
        for cap in kw.captures_iter(row) {
            let cap = cap.get(cap.len().saturating_sub(1)).unwrap();
            let boundaries = bounds(&cap, &row);
            cine(
                &Token {
                    span: boundaries,
                    data: cap.as_str().to_string(),
                    kind: Reader::rgb_fg(highlights["keywords"]).to_string(),
                },
                &mut syntax,
            );
        }
    }
    for (name, exps) in regex {
        // Locate expressions
        for exp in exps.iter() {
            for cap in exp.captures_iter(row) {
                let cap = cap.get(cap.len().saturating_sub(1)).unwrap();
                let boundaries = bounds(&cap, &row);
                cine(
                    &Token {
                        span: boundaries,
                        data: cap.as_str().to_string(),
                        kind: Reader::rgb_fg(highlights[name]).to_string(),
                    },
                    &mut syntax,
                );
            }
        }
    }
    syntax
}

pub fn remove_nested_tokens(tokens: &HashMap<usize, Token>, line: &str) -> HashMap<usize, Token> {
    // Remove tokens within tokens
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
