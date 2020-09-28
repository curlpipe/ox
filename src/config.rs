// Config.rs - In charge of storing configuration information
use ron::de::from_str;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use termion::color;

// Error enum for config reading
#[derive(Debug)]
pub enum Status {
    Parse(String),
    File,
    Success,
}

// Struct for storing and managing configuration
#[derive(Debug, Deserialize)]
pub struct Reader {
    pub general: General,
    pub theme: Theme,
    pub syntax: Syntax,
}

impl Reader {
    pub fn read(config: &str) -> (Self, Status) {
        let rust_keywords: Vec<String> = [
            "as", "break", "const", "continue", "crate", "else", "enum", "extern", "fn", "for",
            "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref",
            "return", "self", "Self", "static", "struct", "super", "trait", "type", "unsafe",
            "use", "where", "while", "async", "await", "dyn", "abstract", "become", "box", "do",
            "final", "macro", "override", "priv", "typeof", "unsized", "virtual", "yield", "try",
            "'static",
        ]
        .iter()
        .map(|x| x.to_string())
        .collect();
        let default = Self {
            general: General {
                line_number_padding_right: 2,
                line_number_padding_left: 1,
                tab_width: 4,
                undo_period: 5,
            },
            theme: Theme {
                editor_bg: (41, 41, 61),
                editor_fg: (255, 255, 255),
                status_bg: (59, 59, 84),
                status_fg: (35, 240, 144),
                line_number_fg: (65, 65, 98),
            },
            syntax: Syntax {
                highlights: [
                    ("comments".to_string(), (113, 113, 169)),
                    ("keywords".to_string(), (134, 76, 232)),
                    ("strings".to_string(), (39, 222, 145)),
                    ("characters".to_string(), (40, 198, 232)),
                    ("digits".to_string(), (134, 76, 232)),
                    ("booleans".to_string(), (86, 217, 178)),
                    ("functions".to_string(), (134, 76, 232)),
                ]
                .iter()
                .cloned()
                .collect(),
                languages: vec![Language {
                    name: "Rust".to_string(),
                    icon: " ".to_string(),
                    extensions: vec!["rs".to_string()],
                    definitions: [
                        ("comments".to_string(), vec!["(//.*$)".to_string()]), // Rust comments
                        ("keywords".to_string(), rust_keywords), // Keywords in the Rust language
                        ("strings".to_string(), vec!["(\".*?\")".to_string()]),
                        ("characters".to_string(), vec!["('.')".to_string()]),
                        ("digits".to_string(), vec!["(\\d+.\\d+|\\d)".to_string()]),
                        (
                            "booleans".to_string(),
                            vec!["true".to_string(), "false".to_string()],
                        ),
                        (
                            "functions".to_string(),
                            vec!["fn (.*)\\s*(".to_string(), "\\s(.*)(".to_string()],
                        ),
                    ]
                    .iter()
                    .cloned()
                    .collect(),
                }],
            },
        };
        let config = if let Ok(config) = shellexpand::full(config) {
            (*config).to_string()
        } else {
            config.to_string()
        };
        if let Ok(file) = fs::read_to_string(config) {
            let result: (Self, Status) = if let Ok(contents) = from_str(&file) {
                (contents, Status::Success)
            } else {
                let result: Result<Self, ron::Error> = from_str(&file);
                (default, Status::Parse(format!("{:?}", result)))
            };
            result
        } else {
            (default, Status::File)
        }
    }
    pub fn rgb_fg(colour: (u8, u8, u8)) -> color::Fg<color::Rgb> {
        color::Fg(color::Rgb(colour.0, colour.1, colour.2))
    }
    pub fn rgb_bg(colour: (u8, u8, u8)) -> color::Bg<color::Rgb> {
        color::Bg(color::Rgb(colour.0, colour.1, colour.2))
    }
}

// Struct for storing the general configuration
#[derive(Debug, Deserialize)]
pub struct General {
    pub line_number_padding_right: usize,
    pub line_number_padding_left: usize,
    pub tab_width: usize,
    pub undo_period: u64,
}

// Struct for storing theme information
#[derive(Debug, Deserialize)]
pub struct Theme {
    pub editor_bg: (u8, u8, u8),
    pub editor_fg: (u8, u8, u8),
    pub status_bg: (u8, u8, u8),
    pub status_fg: (u8, u8, u8),
    pub line_number_fg: (u8, u8, u8),
}

// Struct for storing syntax information
#[derive(Debug, Deserialize)]
pub struct Syntax {
    pub highlights: HashMap<String, (u8, u8, u8)>,
    pub languages: Vec<Language>,
}

// Struct for storing language information
#[derive(Debug, Deserialize)]
pub struct Language {
    pub name: String,
    pub icon: String,
    pub extensions: Vec<String>,
    pub definitions: HashMap<String, Vec<String>>,
}
