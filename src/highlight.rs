// Highlight.rs - For syntax highlighting
use crate::util::Exp;
use std::collections::HashMap;
use termion::color;

pub fn highlight(row: &str, regexes: &Exp) -> HashMap<usize, Vec<String>> {
    let mut syntax: HashMap<usize, Vec<String>> = HashMap::new();
    for cap in regexes.digits.captures_iter(row) {
        let cap = cap.get(cap.len().saturating_sub(1)).unwrap();
        if let Some(index) = syntax.get_mut(&cap.start()) {
            index.push(color::Fg(color::Rgb(40, 198, 232)).to_string());
        } else {
            syntax.insert(
                cap.start(),
                vec![color::Fg(color::Rgb(40, 198, 232)).to_string()],
            );
        }
        if let Some(index) = syntax.get_mut(&cap.end()) {
            index.push(color::Fg(color::Reset).to_string());
        } else {
            syntax.insert(cap.end(), vec![color::Fg(color::Reset).to_string()]);
        }
    }
    for cap in regexes.strings.captures_iter(row) {
        let cap = cap.get(cap.len().saturating_sub(1)).unwrap();
        if let Some(index) = syntax.get_mut(&cap.start()) {
            index.push(color::Fg(color::Rgb(39, 222, 145)).to_string());
        } else {
            syntax.insert(
                cap.start(),
                vec![color::Fg(color::Rgb(39, 222, 145)).to_string()],
            );
        }
        if let Some(index) = syntax.get_mut(&cap.end()) {
            index.push(color::Fg(color::Reset).to_string());
        } else {
            syntax.insert(cap.end(), vec![color::Fg(color::Reset).to_string()]);
        }
    }
    syntax
}
