use crate::cli::VERSION;
use crate::editor::{Editor, FileContainer};
use crate::error::Result;
use crate::Feedback;
use crossterm::style::SetForegroundColor as Fg;
use kaolinite::searching::Searcher;
use kaolinite::utils::{get_absolute_path, get_file_ext, get_file_name};
use mlua::prelude::*;

use super::{issue_warning, Colors};

/// For storing general configuration related to the terminal functionality
#[derive(Debug)]
pub struct Terminal {
    pub mouse_enabled: bool,
}

impl Default for Terminal {
    fn default() -> Self {
        Self {
            mouse_enabled: true,
        }
    }
}

impl LuaUserData for Terminal {
    fn add_fields<'lua, F: LuaUserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field_method_get("mouse_enabled", |_, this| Ok(this.mouse_enabled));
        fields.add_field_method_set("mouse_enabled", |_, this, value| {
            this.mouse_enabled = value;
            Ok(())
        });
    }
}

/// For storing configuration information related to line numbers
#[derive(Debug)]
pub struct LineNumbers {
    pub enabled: bool,
    pub padding_left: usize,
    pub padding_right: usize,
}

impl Default for LineNumbers {
    fn default() -> Self {
        Self {
            enabled: true,
            padding_left: 1,
            padding_right: 1,
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
        fields.add_field_method_get("padding_left", |_, this| Ok(this.padding_left));
        fields.add_field_method_set("padding_left", |_, this, value| {
            this.padding_left = value;
            Ok(())
        });
        fields.add_field_method_get("padding_right", |_, this| Ok(this.padding_right));
        fields.add_field_method_set("padding_right", |_, this, value| {
            this.padding_right = value;
            Ok(())
        });
    }
}

/// For storing configuration information related to the greeting message
#[derive(Debug)]
pub struct GreetingMessage {
    pub enabled: bool,
    pub format: String,
}

impl Default for GreetingMessage {
    fn default() -> Self {
        Self {
            enabled: true,
            format: String::new(),
        }
    }
}

impl GreetingMessage {
    /// Take the configuration information and render the greeting message
    pub fn render(&self, lua: &Lua, colors: &Colors) -> Result<String> {
        let highlight = Fg(colors.highlight.to_color()?).to_string();
        let editor_fg = Fg(colors.editor_fg.to_color()?).to_string();
        let mut result = self.format.clone();
        result = result.replace("{version}", VERSION).to_string();
        result = result.replace("{highlight_start}", &highlight).to_string();
        result = result.replace("{highlight_end}", &editor_fg).to_string();
        // Find functions to call and substitute in
        let mut searcher = Searcher::new(r"\{[A-Za-z_][A-Za-z0-9_]*\}");
        while let Some(m) = searcher.lfind(&result) {
            let name = m
                .text
                .chars()
                .skip(1)
                .take(m.text.chars().count().saturating_sub(2))
                .collect::<String>();
            if let Ok(func) = lua.globals().get::<String, LuaFunction>(name) {
                if let Ok(r) = func.call::<(), LuaString>(()) {
                    result = result.replace(&m.text, r.to_str().unwrap_or(""));
                } else {
                    break;
                }
            } else {
                break;
            }
        }
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

/// For storing configuration information related to the help message
#[derive(Debug)]
pub struct HelpMessage {
    pub enabled: bool,
    pub format: String,
}

impl Default for HelpMessage {
    fn default() -> Self {
        Self {
            enabled: true,
            format: String::new(),
        }
    }
}

impl HelpMessage {
    /// Take the configuration information and render the help message
    pub fn render(&self, lua: &Lua) -> Vec<(bool, String)> {
        let mut message = self.format.clone();
        //result = result.replace("{highlight_start}", &highlight).to_string();
        //result = result.replace("{highlight_end}", &editor_fg).to_string();
        message = message.replace("{version}", VERSION).to_string();
        // Find functions to call and substitute in
        let mut searcher = Searcher::new(r"\{[A-Za-z_][A-Za-z0-9_]*\}");
        while let Some(m) = searcher.lfind(&message) {
            let name = m
                .text
                .chars()
                .skip(1)
                .take(m.text.chars().count().saturating_sub(2))
                .collect::<String>();
            if let Ok(func) = lua.globals().get::<String, LuaFunction>(name) {
                if let Ok(r) = func.call::<(), LuaString>(()) {
                    message = message.replace(&m.text, r.to_str().unwrap_or(""));
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        let mut highlighted = false;
        let mut result = vec![];
        for line in message.split('\n') {
            // Process highlighter lines
            if line.trim() == "{highlight_start}" {
                result.push((true, String::new()));
                highlighted = true;
            } else if line.trim() == "{highlight_end}" {
                highlighted = false;
            } else {
                result.push((highlighted, line.to_string()));
            }
        }
        result
    }
}

impl LuaUserData for HelpMessage {
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

/// For storing configuration information related to the status line
#[derive(Debug)]
pub struct TabLine {
    pub enabled: bool,
    pub format: String,
}

impl Default for TabLine {
    fn default() -> Self {
        Self {
            enabled: true,
            format: "  {file_name}{modified}  ".to_string(),
        }
    }
}

impl TabLine {
    /// Take the configuration information and render the tab line
    pub fn render(&self, lua: &Lua, file: &FileContainer, feedback: &mut Feedback) -> String {
        let path = file
            .doc
            .file_name
            .clone()
            .unwrap_or_else(|| "[No Name]".to_string());
        let file_extension = get_file_ext(&path).unwrap_or_else(|| "Unknown".to_string());
        let absolute_path = get_absolute_path(&path).unwrap_or_else(|| "[No Name]".to_string());
        let file_name = get_file_name(&path).unwrap_or_else(|| "[No Name]".to_string());
        let icon = file.file_type.clone().map_or("󰈙 ".to_string(), |t| t.icon);
        let modified = if file.doc.info.modified { "[+]" } else { "" };
        let mut result = self.format.clone();
        result = result
            .replace("{file_extension}", &file_extension)
            .to_string();
        result = result.replace("{file_name}", &file_name).to_string();
        result = result
            .replace("{absolute_path}", &absolute_path)
            .to_string();
        result = result.replace("{path}", &path).to_string();
        result = result.replace("{modified}", modified).to_string();
        result = result.replace("{icon}", &icon).to_string();
        // Find functions to call and substitute in
        let mut searcher = Searcher::new(r"\{[A-Za-z_][A-Za-z0-9_]*\}");
        while let Some(m) = searcher.lfind(&result) {
            let name = m
                .text
                .chars()
                .skip(1)
                .take(m.text.chars().count().saturating_sub(2))
                .collect::<String>();
            if let Ok(func) = lua.globals().get::<String, LuaFunction>(name) {
                match func.call::<String, LuaString>(absolute_path.clone()) {
                    Ok(r) => {
                        result = result.replace(&m.text, r.to_str().unwrap_or(""));
                    }
                    Err(e) => {
                        *feedback = Feedback::Error(format!("Error occured in tab line: {e:?}"));
                        break;
                    }
                }
            } else {
                break;
            }
        }
        result
    }
}

impl LuaUserData for TabLine {
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

/// For storing configuration information related to the status line
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
    /// Take the configuration information and render the status line
    pub fn render(&self, editor: &Editor, lua: &Lua, w: usize) -> String {
        let file = &editor.files[editor.ptr];
        let mut result = vec![];
        let path = editor
            .doc()
            .file_name
            .clone()
            .unwrap_or_else(|| "[No Name]".to_string());
        let file_extension = get_file_ext(&path).unwrap_or_else(|| "Unknown".to_string());
        let absolute_path = get_absolute_path(&path).unwrap_or_else(|| "[No Name]".to_string());
        let file_name = get_file_name(&path).unwrap_or_else(|| "[No Name]".to_string());
        let file_type = file
            .file_type
            .clone()
            .map_or("Unknown".to_string(), |t| t.name);
        let icon = file.file_type.clone().map_or("󰈙 ".to_string(), |t| t.icon);
        let modified = if editor.doc().info.modified {
            "[+]"
        } else {
            ""
        };
        let cursor_y = (editor.doc().loc().y + 1).to_string();
        let cursor_x = editor.doc().char_ptr.to_string();
        let line_count = editor.doc().len_lines().to_string();

        for part in &self.parts {
            let mut part = part.clone();
            part = part.replace("{file_name}", &file_name).to_string();
            part = part
                .replace("{file_extension}", &file_extension)
                .to_string();
            part = part.replace("{icon}", &icon).to_string();
            part = part.replace("{path}", &path).to_string();
            part = part.replace("{absolute_path}", &absolute_path).to_string();
            part = part.replace("{modified}", modified).to_string();
            part = part.replace("{file_type}", &file_type).to_string();
            part = part.replace("{cursor_y}", &cursor_y).to_string();
            part = part.replace("{cursor_x}", &cursor_x).to_string();
            part = part.replace("{line_count}", &line_count).to_string();
            // Find functions to call and substitute in
            let mut searcher = Searcher::new(r"\{[A-Za-z_][A-Za-z0-9_]*\}");
            while let Some(m) = searcher.lfind(&part) {
                let name = m
                    .text
                    .chars()
                    .skip(1)
                    .take(m.text.chars().count().saturating_sub(2))
                    .collect::<String>();
                if let Ok(func) = lua.globals().get::<String, LuaFunction>(name) {
                    if let Ok(r) = func.call::<(), LuaString>(()) {
                        part = part.replace(&m.text, r.to_str().unwrap_or(""));
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
            result.push(part);
        }
        let status: Vec<&str> = result.iter().map(String::as_str).collect();
        match self.alignment {
            StatusAlign::Between => alinio::align::between(status.as_slice(), w),
            StatusAlign::Around => alinio::align::around(status.as_slice(), w),
        }
        .unwrap_or_else(String::new)
    }
}

impl LuaUserData for StatusLine {
    fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method_mut("clear", |_, status_line, ()| {
            status_line.parts.clear();
            Ok(())
        });
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
        fields.add_field_method_set("alignment", |_, this, value: String| {
            this.alignment = StatusAlign::from_string(&value);
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
    /// Converts a status line alignment value from string representation (in lua)
    pub fn from_string(string: &str) -> Self {
        match string {
            "around" => Self::Around,
            "between" => Self::Between,
            // If the user has provided some random value, just default to between
            _ => {
                issue_warning(
                    "\
                    Invalid status line alignment used in configuration file - \
                    make sure value is either 'around' or 'between' (defaulting to 'between')",
                );
                Self::Between
            }
        }
    }
}

impl From<StatusAlign> for String {
    /// Turns a status line object into a string
    fn from(val: StatusAlign) -> Self {
        match val {
            StatusAlign::Around => "around",
            StatusAlign::Between => "between",
        }
        .to_string()
    }
}
