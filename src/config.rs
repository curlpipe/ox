use mlua::prelude::*;
use std::{cell::RefCell, rc::Rc};
use crate::editor::Editor;
use crate::cli::VERSION;
use crate::error::{OxError, Result};
use kaolinite::utils::filetype;
use std::collections::HashMap;
use crossterm::{
    style::{Color, SetForegroundColor as Fg},
};

// Gracefully exit the program
fn graceful_panic(msg: &str) {
    eprintln!("{}", msg);
    std::process::exit(1);
}

const DEFAULT_CONFIG: &str = include_str!("../config/.oxrc");

#[derive(Debug)]
pub struct Config {
    pub syntax_highlighting: Rc<RefCell<SyntaxHighlighting>>,
    pub line_numbers: Rc<RefCell<LineNumbers>>,
    pub colors: Rc<RefCell<Colors>>,
    pub status_line: Rc<RefCell<StatusLine>>,
    pub greeting_message: Rc<RefCell<GreetingMessage>>,
}

impl Config {
    pub fn read() -> Result<Self> {
        // Load defaults
        let lua = Lua::new();

        // Set up structs to populate (the default values will be thrown away)
        let syntax_highlighting = Rc::new(RefCell::new(SyntaxHighlighting::default()));
        let line_numbers = Rc::new(RefCell::new(LineNumbers::default()));
        let greeting_message = Rc::new(RefCell::new(GreetingMessage::default()));
        let colors = Rc::new(RefCell::new(Colors::default()));
        let status_line = Rc::new(RefCell::new(StatusLine::default()));

        // Push in configuration globals
        lua.globals().set("syntax", syntax_highlighting.clone())?;
        lua.globals().set("line_numbers", line_numbers.clone())?;
        lua.globals().set("greeting_message", greeting_message.clone())?;
        lua.globals().set("status_line", status_line.clone())?;
        lua.globals().set("colors", colors.clone())?;

        // Load the default config to start with
        lua.load(DEFAULT_CONFIG).exec()?;

        // Attempt to read config file from home directory
        if let Ok(path) = shellexpand::full("~/.oxrc") {
            if let Ok(config) = std::fs::read_to_string(path.to_string()) {
                // Update configuration with user-defined values
                lua.load(config).exec()?;
            }
        }

        Ok(Config {
            syntax_highlighting,
            line_numbers,
            greeting_message,
            status_line,
            colors,
        })
    }
}

#[derive(Debug)]
pub struct SyntaxHighlighting {
    pub theme: HashMap<String, ConfigColor>,
}

impl Default for SyntaxHighlighting {
    fn default() -> Self {
        Self {
            theme: HashMap::default(),
        }
    }
}

impl SyntaxHighlighting {
    pub fn get_theme(&self, name: &str) -> Result<Color> {
        if let Some(col) = self.theme.get(name) {
            col.to_color()
        } else {
            Err(OxError::Config(format!("{} has not been given a colour in the theme", name)))
        }
    }
}

impl LuaUserData for SyntaxHighlighting {
    fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method_mut("set", |_, syntax_highlighting, (name, value)| {
            syntax_highlighting.theme.insert(name, ConfigColor::from_lua(value));
            Ok(())
        });
    }
}

#[derive(Debug)]
pub struct LineNumbers {
    pub enabled: bool,
}

impl Default for LineNumbers {
    fn default() -> Self {
        Self {
            enabled: true,
        }
    }
}

impl LuaUserData for LineNumbers {
    fn add_fields<'lua, F: LuaUserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field_method_get("enabled", |_, this| Ok(this.enabled));
        fields.add_field_method_set("enabled", |_, this, value| {
            this.enabled = value;
            Ok(())
        });
    }
}

#[derive(Debug)]
pub struct GreetingMessage {
    pub enabled: bool,
    pub format: String,
}

impl Default for GreetingMessage {
    fn default() -> Self {
        Self {
            enabled: true,
            format: "".to_string(),
        }
    }
}

impl GreetingMessage {
    pub fn render(&self, colors: &Colors) -> Result<String> {
        let highlight = Fg(colors.highlight.to_color()?).to_string();
        let editor_fg = Fg(colors.editor_fg.to_color()?).to_string();
        let mut result = self.format.clone();
        result = result.replace("{version}", &VERSION).to_string();
        result = result.replace("{highlight_start}", &highlight).to_string();
        result = result.replace("{highlight_end}", &editor_fg).to_string();
        Ok(result)
    }
}

impl LuaUserData for GreetingMessage {
    fn add_fields<'lua, F: LuaUserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field_method_get("enabled", |_, this| Ok(this.enabled));
        fields.add_field_method_set("enabled", |_, this, value| {
            this.enabled = value;
            Ok(())
        });
        fields.add_field_method_get("format", |_, this| Ok(this.format.clone()));
        fields.add_field_method_set("format", |_, this, value| {
            this.format = value;
            Ok(())
        });
    }
}

#[derive(Debug)]
pub struct StatusLine {
    pub parts: Vec<String>,
    pub alignment: StatusAlign,
}

impl Default for StatusLine {
    fn default() -> Self {
        Self {
            parts: vec![],
            alignment: StatusAlign::Between,
        }
    }
}

impl StatusLine {
    pub fn render(&self, editor: &Editor, w: usize) -> String {
        let mut result = vec![];
        let ext = editor.doc()
            .file_name
            .as_ref()
            .and_then(|name| Some(name.split('.').last().unwrap().to_string()))
            .unwrap_or_else(|| "".to_string());
        let file_type = filetype(&ext).unwrap_or(ext);
        let file_name = editor.doc()
            .file_name
            .as_ref()
            .and_then(|name| Some(name.split('/').last().unwrap().to_string()))
            .unwrap_or_else(|| "[No Name]".to_string());
        let modified = if editor.doc().modified { "[+]" } else { "" };
        let cursor_y = (editor.doc().loc().y + 1).to_string();
        let cursor_x = editor.doc().char_ptr.to_string();
        let line_count = editor.doc().len_lines().to_string();

        for part in &self.parts {
            let mut part = part.clone();
            part = part.replace("{file_name}", &file_name).to_string();
            part = part.replace("{modified}", &modified).to_string();
            part = part.replace("{file_type}", &file_type).to_string();
            part = part.replace("{cursor_y}", &cursor_y).to_string();
            part = part.replace("{cursor_x}", &cursor_x).to_string();
            part = part.replace("{line_count}", &line_count).to_string();
            result.push(part);
        }
        let status: Vec<&str> = result.iter().map(|s| s.as_str()).collect();
        match self.alignment {
            StatusAlign::Between => alinio::align::between(status.as_slice(), w),
            StatusAlign::Around => alinio::align::around(status.as_slice(), w),
        }.unwrap_or_else(|| "".to_string())
    }
}

impl LuaUserData for StatusLine {
    fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method_mut("add_part", |_, status_line, part| {
            status_line.parts.push(part);
            Ok(())
        });
    }

    fn add_fields<'lua, F: LuaUserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field_method_get("alignment", |_, this| {
            let alignment: String = this.alignment.clone().into();
            Ok(alignment)
        });
        fields.add_field_method_set("alignment", |_, this, value| {
            this.alignment = StatusAlign::from_string(value);
            Ok(())
        });
    }
}

#[derive(Debug, Clone)]
pub enum StatusAlign {
    Around,
    Between,
}

impl StatusAlign {
    pub fn from_string(string: String) -> Self {
        match string.as_str() {
            "around" => Self::Around,
            "between" => Self::Between,
            _ => {
                graceful_panic("\
                    Invalid status line alignment used in configuration file\n\
                    Make sure value is either 'around' or 'between'");
                unreachable!();
            }
        }
    }
}

impl Into<String> for StatusAlign {
    fn into(self) -> String {
        match self {
            Self::Around => "around",
            Self::Between => "between",
        }.to_string()
    }
}

#[derive(Debug)]
pub struct Colors {
    pub editor_bg: ConfigColor,
    pub editor_fg: ConfigColor,

    pub status_bg: ConfigColor,
    pub status_fg: ConfigColor,

    pub highlight: ConfigColor,

    pub line_number_fg: ConfigColor,
    pub line_number_bg: ConfigColor,

    pub tab_active_fg: ConfigColor,
    pub tab_active_bg: ConfigColor,
    pub tab_inactive_fg: ConfigColor,
    pub tab_inactive_bg: ConfigColor,

    pub info_bg: ConfigColor,
    pub info_fg: ConfigColor,
    pub warning_bg: ConfigColor,
    pub warning_fg: ConfigColor,
    pub error_bg: ConfigColor,
    pub error_fg: ConfigColor,
}

impl Default for Colors {
    fn default() -> Self {
        Self {
            editor_bg: ConfigColor::Black,
            editor_fg: ConfigColor::Black,

            status_bg: ConfigColor::Black,
            status_fg: ConfigColor::Black,

            highlight: ConfigColor::Black,

            line_number_fg: ConfigColor::Black,
            line_number_bg: ConfigColor::Black,

            tab_active_fg: ConfigColor::Black,
            tab_active_bg: ConfigColor::Black,
            tab_inactive_fg: ConfigColor::Black,
            tab_inactive_bg: ConfigColor::Black,

            info_bg: ConfigColor::Black,
            info_fg: ConfigColor::Black,
            warning_bg: ConfigColor::Black,
            warning_fg: ConfigColor::Black,
            error_bg: ConfigColor::Black,
            error_fg: ConfigColor::Black,
        }
    }
}

impl LuaUserData for Colors {
    fn add_fields<'lua, F: LuaUserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field_method_get("editor_bg", |env, this| Ok(this.editor_bg.to_lua(env)));
        fields.add_field_method_get("editor_fg", |env, this| Ok(this.editor_fg.to_lua(env)));
        fields.add_field_method_get("status_bg", |env, this| Ok(this.status_bg.to_lua(env)));
        fields.add_field_method_get("status_fg", |env, this| Ok(this.status_fg.to_lua(env)));
        fields.add_field_method_get("highlight", |env, this| Ok(this.highlight.to_lua(env)));
        fields.add_field_method_get("line_number_bg", |env, this| Ok(this.line_number_bg.to_lua(env)));
        fields.add_field_method_get("line_number_fg", |env, this| Ok(this.line_number_fg.to_lua(env)));
        fields.add_field_method_get("tab_active_fg", |env, this| Ok(this.tab_active_fg.to_lua(env)));
        fields.add_field_method_get("tab_active_bg", |env, this| Ok(this.tab_active_bg.to_lua(env)));
        fields.add_field_method_get("tab_inactive_fg", |env, this| Ok(this.tab_inactive_fg.to_lua(env)));
        fields.add_field_method_get("tab_inactive_bg", |env, this| Ok(this.tab_inactive_bg.to_lua(env)));
        fields.add_field_method_get("error_bg", |env, this| Ok(this.error_bg.to_lua(env)));
        fields.add_field_method_get("error_fg", |env, this| Ok(this.error_fg.to_lua(env)));
        fields.add_field_method_get("warning_bg", |env, this| Ok(this.warning_bg.to_lua(env)));
        fields.add_field_method_get("warning_fg", |env, this| Ok(this.warning_fg.to_lua(env)));
        fields.add_field_method_get("info_bg", |env, this| Ok(this.info_bg.to_lua(env)));
        fields.add_field_method_get("info_fg", |env, this| Ok(this.info_fg.to_lua(env)));
        fields.add_field_method_set("editor_bg", |_, this, value| {
            this.editor_bg = ConfigColor::from_lua(value);
            Ok(())
        });
        fields.add_field_method_set("editor_fg", |_, this, value| {
            this.editor_fg = ConfigColor::from_lua(value);
            Ok(())
        });
        fields.add_field_method_set("status_bg", |_, this, value| {
            this.status_bg = ConfigColor::from_lua(value);
            Ok(())
        });
        fields.add_field_method_set("status_fg", |_, this, value| {
            this.status_fg = ConfigColor::from_lua(value);
            Ok(())
        });
        fields.add_field_method_set("highlight", |_, this, value| {
            this.highlight = ConfigColor::from_lua(value);
            Ok(())
        });
        fields.add_field_method_set("line_number_bg", |_, this, value| {
            this.line_number_bg = ConfigColor::from_lua(value);
            Ok(())
        });
        fields.add_field_method_set("line_number_fg", |_, this, value| {
            this.line_number_fg = ConfigColor::from_lua(value);
            Ok(())
        });
        fields.add_field_method_set("tab_active_fg", |_, this, value| {
            this.tab_active_fg = ConfigColor::from_lua(value);
            Ok(())
        });
        fields.add_field_method_set("tab_active_bg", |_, this, value| {
            this.tab_active_bg = ConfigColor::from_lua(value);
            Ok(())
        });
        fields.add_field_method_set("tab_inactive_fg", |_, this, value| {
            this.tab_inactive_fg = ConfigColor::from_lua(value);
            Ok(())
        });
        fields.add_field_method_set("tab_inactive_bg", |_, this, value| {
            this.tab_inactive_bg = ConfigColor::from_lua(value);
            Ok(())
        });
        fields.add_field_method_set("error_bg", |_, this, value| {
            this.error_bg = ConfigColor::from_lua(value);
            Ok(())
        });
        fields.add_field_method_set("error_fg", |_, this, value| {
            this.error_fg = ConfigColor::from_lua(value);
            Ok(())
        });
        fields.add_field_method_set("warning_bg", |_, this, value| {
            this.warning_bg = ConfigColor::from_lua(value);
            Ok(())
        });
        fields.add_field_method_set("warning_fg", |_, this, value| {
            this.warning_fg = ConfigColor::from_lua(value);
            Ok(())
        });
        fields.add_field_method_set("info_bg", |_, this, value| {
            this.info_bg = ConfigColor::from_lua(value);
            Ok(())
        });
        fields.add_field_method_set("info_fg", |_, this, value| {
            this.info_fg = ConfigColor::from_lua(value);
            Ok(())
        });
    }
}

#[derive(Debug)]
pub enum ConfigColor {
    Rgb(u8, u8, u8),
    Hex(String),
    Black,
    DarkGrey,
    Red,
    DarkRed,
    Green,
    DarkGreen,
    Yellow,
    DarkYellow,
    Blue,
    DarkBlue,
    Magenta,
    DarkMagenta,
    Cyan,
    DarkCyan,
    White,
    Grey,
    Transparent,
}

impl ConfigColor {
    pub fn from_lua<'a>(value: LuaValue<'a>) -> Self {
        match value {
            LuaValue::String(string) => match string.to_str().unwrap() {
                "black" => Self::Black,
                "darkgrey" => Self::DarkGrey,
                "red" => Self::Red,
                "darkred" => Self::DarkRed,
                "green" => Self::Green,
                "darkgreen" => Self::DarkGreen,
                "yellow" => Self::Yellow,
                "darkyellow" => Self::DarkYellow,
                "blue" => Self::Blue,
                "darkblue" => Self::DarkBlue,
                "magenta" => Self::Magenta,
                "darkmagenta" => Self::DarkMagenta,
                "cyan" => Self::Cyan,
                "darkcyan" => Self::DarkCyan,
                "white" => Self::White,
                "grey" => Self::Grey,
                "transparent" => Self::Transparent,
                hex => Self::Hex(hex.to_string()),
            },
            LuaValue::Table(table) => {
                if table.len().unwrap() != 3 {
                    graceful_panic("Invalid RGB sequence used in configuration file (must be a list of 3 numbers)");
                }
                let b: u8 = table.pop().expect("Invalid rgb sequence");
                let g: u8 = table.pop().expect("Invalid rgb sequence");
                let r: u8 = table.pop().expect("Invalid rgb sequence");
                Self::Rgb(r, g, b)
            }
            _ => {
                graceful_panic("Invalid data type used for colour in configuration file");
                unreachable!()
            }
        }
    }

    pub fn to_lua<'a>(&self, env: &'a Lua) -> LuaValue<'a> {
        match self {
            ConfigColor::Hex(hex) => {
                let string = env.create_string(hex).unwrap();
                LuaValue::String(string)
            }
            ConfigColor::Rgb(r, g, b) => {
                // Create lua table
                let table = env.create_table().unwrap();
                table.push(*r as isize).unwrap();
                table.push(*g as isize).unwrap();
                table.push(*b as isize).unwrap();
                LuaValue::Table(table)
            }
            ConfigColor::Black => LuaValue::String(env.create_string("black").unwrap()),
            ConfigColor::DarkGrey => LuaValue::String(env.create_string("darkgrey").unwrap()),
            ConfigColor::Red => LuaValue::String(env.create_string("red").unwrap()),
            ConfigColor::DarkRed => LuaValue::String(env.create_string("darkred").unwrap()),
            ConfigColor::Green => LuaValue::String(env.create_string("green").unwrap()),
            ConfigColor::DarkGreen => LuaValue::String(env.create_string("darkgreen").unwrap()),
            ConfigColor::Yellow => LuaValue::String(env.create_string("yellow").unwrap()),
            ConfigColor::DarkYellow => LuaValue::String(env.create_string("darkyellow").unwrap()),
            ConfigColor::Blue => LuaValue::String(env.create_string("blue").unwrap()),
            ConfigColor::DarkBlue => LuaValue::String(env.create_string("darkblue").unwrap()),
            ConfigColor::Magenta => LuaValue::String(env.create_string("magenta").unwrap()),
            ConfigColor::DarkMagenta => LuaValue::String(env.create_string("darkmagenta").unwrap()),
            ConfigColor::Cyan => LuaValue::String(env.create_string("cyan").unwrap()),
            ConfigColor::DarkCyan => LuaValue::String(env.create_string("darkcyan").unwrap()),
            ConfigColor::White => LuaValue::String(env.create_string("white").unwrap()),
            ConfigColor::Grey => LuaValue::String(env.create_string("grey").unwrap()),
            ConfigColor::Transparent => LuaValue::String(env.create_string("transparent").unwrap()),
        }
    }

    pub fn to_color(&self) -> Result<Color> {
        Ok(match self {
            ConfigColor::Hex(hex) => {
                let (r, g, b) = self.hex_to_rgb(hex)?;
                Color::Rgb { r, g, b }
            }
            ConfigColor::Rgb(r, g, b) => Color::Rgb { r: *r, g: *g, b: *b },
            ConfigColor::Black => Color::Black,
            ConfigColor::DarkGrey => Color::DarkGrey,
            ConfigColor::Red => Color::Red,
            ConfigColor::DarkRed => Color::DarkRed,
            ConfigColor::Green => Color::Green,
            ConfigColor::DarkGreen => Color::DarkGreen,
            ConfigColor::Yellow => Color::Yellow,
            ConfigColor::DarkYellow => Color::DarkYellow,
            ConfigColor::Blue => Color::Blue,
            ConfigColor::DarkBlue => Color::DarkBlue,
            ConfigColor::Magenta => Color::Magenta,
            ConfigColor::DarkMagenta => Color::DarkMagenta,
            ConfigColor::Cyan => Color::Cyan,
            ConfigColor::DarkCyan => Color::DarkCyan,
            ConfigColor::White => Color::White,
            ConfigColor::Grey => Color::Grey,
            ConfigColor::Transparent => Color::Reset,
        })
    }

    fn hex_to_rgb(&self, hex: &str) -> Result<(u8, u8, u8)> {
        // Remove the leading '#' if present
        let hex = hex.trim_start_matches('#');

        // Ensure the hex code is exactly 6 characters long
        if hex.len() != 6 {
            graceful_panic("Invalid hex code used in configuration file");
        }

        // Parse the hex string into the RGB components
        let r = u8::from_str_radix(&hex[0..2], 16).expect("invalid R component in hex code");
        let g = u8::from_str_radix(&hex[2..4], 16).expect("invalid G component in hex code");
        let b = u8::from_str_radix(&hex[4..6], 16).expect("invalid B component in hex code");

        Ok((r, g, b))
    }
}
