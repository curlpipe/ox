/// Utilities for configuring and rendering parts of the interface
use crate::cli::VERSION;
use crate::editor::{Editor, FileContainer};
#[cfg(not(target_os = "windows"))]
use crate::pty::Shell;
use crate::Feedback;
use kaolinite::searching::Searcher;
use kaolinite::utils::{get_absolute_path, get_file_ext, get_file_name};
use mlua::prelude::*;
use std::result::Result as RResult;

use super::issue_warning;

type LuaRes<T> = RResult<T, LuaError>;

/// For storing general configuration related to the terminal functionality
#[derive(Debug)]
pub struct Terminal {
    pub mouse_enabled: bool,
    pub scroll_amount: usize,
    #[cfg(not(target_os = "windows"))]
    pub shell: Shell,
    #[cfg(target_os = "windows")]
    #[allow(dead_code)]
    pub shell: (),
}

impl Default for Terminal {
    fn default() -> Self {
        Self {
            mouse_enabled: true,
            scroll_amount: 1,
            #[cfg(not(target_os = "windows"))]
            shell: Shell::Bash,
            #[cfg(target_os = "windows")]
            shell: (),
        }
    }
}

impl LuaUserData for Terminal {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("mouse_enabled", |_, this| Ok(this.mouse_enabled));
        fields.add_field_method_set("mouse_enabled", |_, this, value| {
            this.mouse_enabled = value;
            Ok(())
        });
        fields.add_field_method_get("scroll_amount", |_, this| Ok(this.scroll_amount));
        fields.add_field_method_set("scroll_amount", |_, this, value| {
            this.scroll_amount = value;
            Ok(())
        });
        #[cfg(not(target_os = "windows"))]
        fields.add_field_method_get("shell", |_, this| Ok(this.shell));
        #[cfg(not(target_os = "windows"))]
        fields.add_field_method_set("shell", |_, this, value| {
            this.shell = value;
            Ok(())
        });
        #[cfg(target_os = "windows")]
        fields.add_field_method_get("shell", |_, _| Ok("windows not supported"));
        #[cfg(target_os = "windows")]
        fields.add_field_method_set("shell", |_, _, _: String| Ok(()));
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
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
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
    pub fn render(&self, lua: &Lua) -> (String, Vec<usize>) {
        let mut result = self.format.clone();
        // Substitute in simple values
        result = result.replace("{version}", VERSION).to_string();
        result = result.replace('\t', "    ").to_string();
        // Handle highlighted part
        let start = result.find("{highlight_start}");
        let end = result.find("{highlight_end}");
        let highlighted = if let (Some(s), Some(e)) = (start, end) {
            let s = result.chars().take(s).filter(|c| *c == '\n').count();
            let e = result.chars().take(e).filter(|c| *c == '\n').count();
            (s..=e).collect()
        } else {
            vec![]
        };
        result = result.replace("{highlight_start}", "").to_string();
        result = result.replace("{highlight_end}", "").to_string();
        // Find functions to call and substitute in
        let mut searcher = Searcher::new(r"\{[A-Za-z_][A-Za-z0-9_]*\}");
        while let Some(m) = searcher.lfind(&result) {
            let name = m
                .text
                .chars()
                .skip(1)
                .take(m.text.chars().count().saturating_sub(2))
                .collect::<String>();
            if let Ok(func) = lua.globals().get::<LuaFunction>(name) {
                if let Ok(r) = func.call::<LuaString>(()) {
                    result = result.replace(&m.text, r.to_string_lossy().as_str());
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        (result, highlighted)
    }
}

impl LuaUserData for GreetingMessage {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
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
            if let Ok(func) = lua.globals().get::<LuaFunction>(name) {
                if let Ok(r) = func.call::<LuaString>(()) {
                    message = message.replace(&m.text, r.to_string_lossy().as_str());
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
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
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
    pub separators: bool,
    pub format: String,
}

impl Default for TabLine {
    fn default() -> Self {
        Self {
            enabled: true,
            separators: true,
            format: "  {file_name}{modified}  ".to_string(),
        }
    }
}

impl TabLine {
    /// Take the configuration information and render the tab line
    pub fn render(&self, lua: &Lua, fc: &FileContainer, fb: &mut Feedback) -> String {
        let path = fc
            .doc
            .file_name
            .clone()
            .unwrap_or_else(|| "[No Name]".to_string());
        let file_extension = get_file_ext(&path).unwrap_or_else(|| "Unknown".to_string());
        let absolute_path = get_absolute_path(&path).unwrap_or_else(|| "[No Name]".to_string());
        let file_name = get_file_name(&path).unwrap_or_else(|| "[No Name]".to_string());
        let icon = fc.file_type.clone().map_or("󰈙 ".to_string(), |t| t.icon);
        let modified = if fc.doc.event_mgmt.with_disk(&fc.doc.take_snapshot()) {
            ""
        } else {
            "[+]"
        };
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
            if let Ok(func) = lua.globals().get::<LuaFunction>(name) {
                match func.call::<LuaString>(absolute_path.clone()) {
                    Ok(r) => {
                        result = result.replace(&m.text, r.to_string_lossy().as_str());
                    }
                    Err(e) => {
                        *fb = Feedback::Error(format!("Error occured in tab line: {e:?}"));
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
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
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
        fields.add_field_method_get("separators", |_, this| Ok(this.separators));
        fields.add_field_method_set("separators", |_, this, value| {
            this.separators = value;
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
    pub fn render(&self, ptr: &[usize], editor: &Editor, lua: &Lua, w: usize) -> LuaRes<String> {
        let mut result = vec![];
        let fc = editor.files.get(ptr.to_vec()).unwrap();
        let doc = &fc.doc;
        let path = doc
            .file_name
            .clone()
            .unwrap_or_else(|| "[No Name]".to_string());
        let file_extension = get_file_ext(&path).unwrap_or_else(|| "Unknown".to_string());
        let absolute_path = get_absolute_path(&path).unwrap_or_else(|| "[No Name]".to_string());
        let file_name = get_file_name(&path).unwrap_or_else(|| "[No Name]".to_string());
        let file_type = fc
            .file_type
            .clone()
            .map_or("Unknown".to_string(), |ft| ft.name);
        let icon = fc.file_type.clone().map_or("󰈙 ".to_string(), |ft| ft.icon);
        let modified = if doc.event_mgmt.with_disk(&doc.take_snapshot()) {
            ""
        } else {
            "[+]"
        };
        let cursor_y = (doc.loc().y + 1).to_string();
        let cursor_x = doc.char_ptr.to_string();
        let line_count = doc.len_lines().to_string();

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
                if let Ok(func) = lua.globals().get::<LuaFunction>(name) {
                    let r = func.call::<LuaString>(absolute_path.clone())?;
                    part = part.replace(&m.text, r.to_string_lossy().as_str());
                } else {
                    break;
                }
            }
            result.push(part);
        }
        let status: Vec<&str> = result.iter().map(String::as_str).collect();
        Ok(match self.alignment {
            StatusAlign::Between => alinio::align::between(status.as_slice(), w),
            StatusAlign::Around => alinio::align::around(status.as_slice(), w),
        }
        .unwrap_or_else(String::new))
    }
}

impl LuaUserData for StatusLine {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("parts", |lua, this| {
            let parts = lua.create_table()?;
            for (i, part) in this.parts.iter().enumerate() {
                parts.set(i + 1, part.clone())?;
            }
            Ok(parts)
        });
        fields.add_field_method_set("parts", |_, this, value: LuaTable| {
            let mut result = vec![];
            for item in value.pairs::<usize, String>() {
                let (_, part) = item?;
                result.push(part);
            }
            this.parts = result;
            Ok(())
        });
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
