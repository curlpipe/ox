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
}

impl Reader {
    pub fn read(config: &str) -> (Self, Status) {
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
