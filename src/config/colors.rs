use crate::error::Result;
use crossterm::style::Color;
use mlua::prelude::*;

use super::issue_warning;

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

    pub selection_fg: ConfigColor,
    pub selection_bg: ConfigColor,
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

            selection_fg: ConfigColor::White,
            selection_bg: ConfigColor::Blue,
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
        fields.add_field_method_get("line_number_bg", |env, this| {
            Ok(this.line_number_bg.to_lua(env))
        });
        fields.add_field_method_get("line_number_fg", |env, this| {
            Ok(this.line_number_fg.to_lua(env))
        });
        fields.add_field_method_get("tab_active_fg", |env, this| {
            Ok(this.tab_active_fg.to_lua(env))
        });
        fields.add_field_method_get("tab_active_bg", |env, this| {
            Ok(this.tab_active_bg.to_lua(env))
        });
        fields.add_field_method_get("tab_inactive_fg", |env, this| {
            Ok(this.tab_inactive_fg.to_lua(env))
        });
        fields.add_field_method_get("tab_inactive_bg", |env, this| {
            Ok(this.tab_inactive_bg.to_lua(env))
        });
        fields.add_field_method_get("error_bg", |env, this| Ok(this.error_bg.to_lua(env)));
        fields.add_field_method_get("error_fg", |env, this| Ok(this.error_fg.to_lua(env)));
        fields.add_field_method_get("warning_bg", |env, this| Ok(this.warning_bg.to_lua(env)));
        fields.add_field_method_get("warning_fg", |env, this| Ok(this.warning_fg.to_lua(env)));
        fields.add_field_method_get("info_bg", |env, this| Ok(this.info_bg.to_lua(env)));
        fields.add_field_method_get("info_fg", |env, this| Ok(this.info_fg.to_lua(env)));
        fields.add_field_method_get("selection_fg", |env, this| {
            Ok(this.selection_fg.to_lua(env))
        });
        fields.add_field_method_get("selection_bg", |env, this| {
            Ok(this.selection_bg.to_lua(env))
        });
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
        fields.add_field_method_set("selection_fg", |_, this, value| {
            this.selection_fg = ConfigColor::from_lua(value);
            Ok(())
        });
        fields.add_field_method_set("selection_bg", |_, this, value| {
            this.selection_bg = ConfigColor::from_lua(value);
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
            LuaValue::String(string) => match string.to_str().unwrap_or("transparent") {
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
                if table.len().unwrap_or(3) != 3 {
                    issue_warning("Invalid RGB sequence used in configuration file (must be a list of 3 numbers)");
                    return Self::Transparent;
                }
                let mut tri: Vec<u8> = vec![];
                for _ in 0..3 {
                    if let Ok(val) = table.pop() {
                        tri.insert(0, val)
                    } else {
                        issue_warning("Invalid RGB sequence provided - please check your numerical values are between 0 and 255");
                        tri.insert(0, 255);
                    }
                }
                Self::Rgb(tri[0], tri[1], tri[2])
            }
            _ => {
                issue_warning("Invalid data type used for colour in configuration file");
                Self::Transparent
            }
        }
    }

    pub fn to_lua<'a>(&self, env: &'a Lua) -> LuaValue<'a> {
        let msg = "Failed to create lua string";
        match self {
            ConfigColor::Hex(hex) => {
                let string = env.create_string(hex).expect(msg);
                LuaValue::String(string)
            }
            ConfigColor::Rgb(r, g, b) => {
                // Create lua table
                let table = env.create_table().expect("Failed to create lua table");
                let _ = table.push(*r as isize);
                let _ = table.push(*g as isize);
                let _ = table.push(*b as isize);
                LuaValue::Table(table)
            }
            ConfigColor::Black => LuaValue::String(env.create_string("black").expect(msg)),
            ConfigColor::DarkGrey => LuaValue::String(env.create_string("darkgrey").expect(msg)),
            ConfigColor::Red => LuaValue::String(env.create_string("red").expect(msg)),
            ConfigColor::DarkRed => LuaValue::String(env.create_string("darkred").expect(msg)),
            ConfigColor::Green => LuaValue::String(env.create_string("green").expect(msg)),
            ConfigColor::DarkGreen => LuaValue::String(env.create_string("darkgreen").expect(msg)),
            ConfigColor::Yellow => LuaValue::String(env.create_string("yellow").expect(msg)),
            ConfigColor::DarkYellow => {
                LuaValue::String(env.create_string("darkyellow").expect(msg))
            }
            ConfigColor::Blue => LuaValue::String(env.create_string("blue").expect(msg)),
            ConfigColor::DarkBlue => LuaValue::String(env.create_string("darkblue").expect(msg)),
            ConfigColor::Magenta => LuaValue::String(env.create_string("magenta").expect(msg)),
            ConfigColor::DarkMagenta => {
                LuaValue::String(env.create_string("darkmagenta").expect(msg))
            }
            ConfigColor::Cyan => LuaValue::String(env.create_string("cyan").expect(msg)),
            ConfigColor::DarkCyan => LuaValue::String(env.create_string("darkcyan").expect(msg)),
            ConfigColor::White => LuaValue::String(env.create_string("white").expect(msg)),
            ConfigColor::Grey => LuaValue::String(env.create_string("grey").expect(msg)),
            ConfigColor::Transparent => {
                LuaValue::String(env.create_string("transparent").expect(msg))
            }
        }
    }

    pub fn to_color(&self) -> Result<Color> {
        Ok(match self {
            ConfigColor::Hex(hex) => {
                let (r, g, b) = self.hex_to_rgb(hex)?;
                Color::Rgb { r, g, b }
            }
            ConfigColor::Rgb(r, g, b) => Color::Rgb {
                r: *r,
                g: *g,
                b: *b,
            },
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
            panic!("Invalid hex code used in configuration file - ensure they are of length 6");
        }

        // Parse the hex string into the RGB components
        let mut tri: Vec<u8> = vec![];
        for i in 0..3 {
            let section = &hex[(i * 2)..(i * 2 + 2)];
            if let Ok(val) = u8::from_str_radix(section, 16) {
                tri.insert(0, val)
            } else {
                panic!("Invalid hex code used in configuration file - ensure all digits are between 0 and F");
            }
        }
        Ok((tri[0], tri[1], tri[2]))
    }
}
