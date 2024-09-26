use crate::cli::VERSION;
use crate::editor::Editor;
use crate::error::{OxError, Result};
use crate::ui::Feedback;
use crossterm::{
    event::{KeyCode as KCode, KeyModifiers as KMod, MediaKeyCode, ModifierKeyCode},
    style::{Color, SetForegroundColor as Fg},
};
use kaolinite::searching::Searcher;
use kaolinite::utils::{filetype, get_absolute_path, get_file_ext, get_file_name, icon};
use kaolinite::{Document, Loc};
use mlua::prelude::*;
use std::collections::HashMap;
use std::{cell::RefCell, rc::Rc};
use synoptic::{from_extension, Highlighter};

// Issue a warning to the user
fn issue_warning(msg: &str) {
    eprintln!("[WARNING] {}", msg);
}

/// This contains the default configuration lua file
const DEFAULT_CONFIG: &str = include_str!("../config/.oxrc");

/// Default plug-in code to use
const PAIRS: &str = include_str!("../plugins/pairs.lua");
const AUTOINDENT: &str = include_str!("../plugins/autoindent.lua");

/// This contains the code for setting up plug-in infrastructure
pub const PLUGIN_BOOTSTRAP: &str = include_str!("plugin/bootstrap.lua");

/// This contains the code for running the plugins
pub const PLUGIN_RUN: &str = include_str!("plugin/run.lua");

/// This contains the code for handling a key binding
pub fn run_key(key: &str) -> String {
    format!(
        "
        globalevent = (global_event_mapping[\"*\"] or {{}})
        for _, f in ipairs(globalevent) do
            f()
        end
        key = (global_event_mapping[\"{key}\"] or error(\"key not bound\"))
        for _, f in ipairs(key) do
            f()
        end
        "
    )
}

/// The struct that holds all the configuration information
#[derive(Debug)]
pub struct Config {
    pub syntax_highlighting: Rc<RefCell<SyntaxHighlighting>>,
    pub line_numbers: Rc<RefCell<LineNumbers>>,
    pub colors: Rc<RefCell<Colors>>,
    pub status_line: Rc<RefCell<StatusLine>>,
    pub tab_line: Rc<RefCell<TabLine>>,
    pub greeting_message: Rc<RefCell<GreetingMessage>>,
    pub terminal: Rc<RefCell<TerminalConfig>>,
    pub document: Rc<RefCell<DocumentConfig>>,
}

impl Config {
    /// Take a lua instance, inject all the configuration tables and return a default config struct
    pub fn new(lua: &Lua) -> Result<Self> {
        // Set up structs to populate (the default values will be thrown away)
        let syntax_highlighting = Rc::new(RefCell::new(SyntaxHighlighting::default()));
        let line_numbers = Rc::new(RefCell::new(LineNumbers::default()));
        let greeting_message = Rc::new(RefCell::new(GreetingMessage::default()));
        let colors = Rc::new(RefCell::new(Colors::default()));
        let status_line = Rc::new(RefCell::new(StatusLine::default()));
        let tab_line = Rc::new(RefCell::new(TabLine::default()));
        let terminal = Rc::new(RefCell::new(TerminalConfig::default()));
        let document = Rc::new(RefCell::new(DocumentConfig::default()));

        // Push in configuration globals
        lua.globals().set("syntax", syntax_highlighting.clone())?;
        lua.globals().set("line_numbers", line_numbers.clone())?;
        lua.globals()
            .set("greeting_message", greeting_message.clone())?;
        lua.globals().set("status_line", status_line.clone())?;
        lua.globals().set("tab_line", tab_line.clone())?;
        lua.globals().set("colors", colors.clone())?;
        lua.globals().set("terminal", terminal.clone())?;
        lua.globals().set("document", document.clone())?;

        Ok(Config {
            syntax_highlighting,
            line_numbers,
            greeting_message,
            tab_line,
            status_line,
            colors,
            terminal,
            document,
        })
    }

    /// Actually take the configuration file, open it and interpret it
    pub fn read(&mut self, path: String, lua: &Lua) -> Result<()> {
        // Load the default config to start with
        lua.load(DEFAULT_CONFIG).exec()?;
        // Reset plugin status based on built-in configuration file
        lua.load("plugins = {}").exec()?;
        lua.load("builtins = {}").exec()?;

        // Judge pre-user config state
        let status_parts = self.status_line.borrow().parts.len();

        // Attempt to read config file from home directory
        let mut user_provided_config = false;
        if let Ok(path) = shellexpand::full(&path) {
            if let Ok(config) = std::fs::read_to_string(path.to_string()) {
                // Update configuration with user-defined values
                lua.load(config).exec()?;
                user_provided_config = true;
            }
        }

        // Remove any default values if necessary
        if self.status_line.borrow().parts.len() > status_parts {
            self.status_line.borrow_mut().parts.drain(0..status_parts);
        }

        // Determine whether or not to load built-in plugins
        let mut builtins: HashMap<&str, &str> = HashMap::default();
        builtins.insert("pairs.lua", PAIRS);
        builtins.insert("autoindent.lua", AUTOINDENT);
        for (name, code) in builtins.iter() {
            if self.load_bi(name, user_provided_config, &lua) {
                lua.load(*code).exec()?;
            }
        }

        if user_provided_config {
            Ok(())
        } else {
            Err(OxError::Config("Not Found".to_string()))
        }
    }

    /// Decide whether to load a built-in plugin
    pub fn load_bi(&self, name: &str, user_provided_config: bool, lua: &Lua) -> bool {
        if !user_provided_config {
            // Load when the user hasn't provided a configuration file
            true
        } else {
            // Get list of user-loaded plug-ins
            let plugins: Vec<String> = lua
                .globals()
                .get::<_, LuaTable>("builtins")
                .unwrap()
                .sequence_values()
                .filter_map(std::result::Result::ok)
                .collect();
            // If the user wants to load the plug-in but it isn't available
            if let Some(idx) = plugins.iter().position(|p| p.ends_with(name)) {
                // User wants the plug-in
                let path = &plugins[idx];
                // true if plug-in isn't avilable
                !std::path::Path::new(path).exists()
            } else {
                // User doesn't want the plug-in
                false
            }
        }
    }
}

/// For storing general configuration related to the terminal functionality
#[derive(Debug)]
pub struct TerminalConfig {
    pub mouse_enabled: bool,
}

impl Default for TerminalConfig {
    fn default() -> Self {
        Self {
            mouse_enabled: true,
        }
    }
}

impl LuaUserData for TerminalConfig {
    fn add_fields<'lua, F: LuaUserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field_method_get("mouse_enabled", |_, this| Ok(this.mouse_enabled));
        fields.add_field_method_set("mouse_enabled", |_, this, value| {
            this.mouse_enabled = value;
            Ok(())
        });
    }
}

/// For storing configuration information related to syntax highlighting
#[derive(Debug)]
pub struct SyntaxHighlighting {
    pub theme: HashMap<String, ConfigColor>,
    pub user_rules: HashMap<String, Highlighter>,
}

impl Default for SyntaxHighlighting {
    fn default() -> Self {
        Self {
            theme: HashMap::default(),
            user_rules: HashMap::default(),
        }
    }
}

impl SyntaxHighlighting {
    /// Get a colour from the theme
    pub fn get_theme(&self, name: &str) -> Result<Color> {
        if let Some(col) = self.theme.get(name) {
            col.to_color()
        } else {
            Err(OxError::Config(format!(
                "{} has not been given a colour in the theme",
                name
            )))
        }
    }

    /// Get a highlighter given a file extension
    pub fn get_highlighter(&self, ext: &str) -> Highlighter {
        self.user_rules
            .get(ext)
            .and_then(|h| Some(h.clone()))
            .unwrap_or_else(|| from_extension(ext, 4).unwrap_or_else(|| Highlighter::new(4)))
    }
}

impl LuaUserData for SyntaxHighlighting {
    fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method_mut(
            "keywords",
            |lua, _, (name, pattern): (String, Vec<String>)| {
                let table = lua.create_table()?;
                table.set("kind", "keyword")?;
                table.set("name", name)?;
                table.set("pattern", format!("({})", pattern.join("|")))?;
                Ok(table)
            },
        );
        methods.add_method_mut("keyword", |lua, _, (name, pattern): (String, String)| {
            let table = lua.create_table()?;
            table.set("kind", "keyword")?;
            table.set("name", name)?;
            table.set("pattern", pattern)?;
            Ok(table)
        });
        methods.add_method_mut(
            "bounded",
            |lua, _, (name, start, end, escape): (String, String, String, bool)| {
                let table = lua.create_table()?;
                table.set("kind", "bounded")?;
                table.set("name", name)?;
                table.set("start", start)?;
                table.set("end", end)?;
                table.set("escape", escape.to_string())?;
                Ok(table)
            },
        );
        type BoundedInterpArgs = (String, String, String, String, String, bool);
        methods.add_method_mut(
            "bounded_interpolation",
            |lua, _, (name, start, end, i_start, i_end, escape): BoundedInterpArgs| {
                let table = lua.create_table()?;
                table.set("kind", "bounded_interpolation")?;
                table.set("name", name)?;
                table.set("start", start)?;
                table.set("end", end)?;
                table.set("i_start", i_start)?;
                table.set("i_end", i_end)?;
                table.set("escape", escape.to_string())?;
                Ok(table)
            },
        );
        methods.add_method_mut(
            "new",
            |_, syntax_highlighting, (extensions, rules): (LuaTable, LuaTable)| {
                // Make note of the highlighter
                for ext_idx in 1..(extensions.len()? + 1) {
                    // Create highlighter
                    let mut highlighter = Highlighter::new(4);
                    // Add rules one by one
                    for rule_idx in 1..(rules.len()? + 1) {
                        // Get rule
                        let rule = rules.get::<i64, HashMap<String, String>>(rule_idx)?;
                        // Find type of rule and attatch it to the highlighter
                        match rule["kind"].as_str() {
                            "keyword" => {
                                highlighter.keyword(rule["name"].clone(), &rule["pattern"])
                            }
                            "bounded" => highlighter.bounded(
                                rule["name"].clone(),
                                rule["start"].clone(),
                                rule["end"].clone(),
                                rule["escape"] == "true",
                            ),
                            "bounded_interpolation" => highlighter.bounded_interp(
                                rule["name"].clone(),
                                rule["start"].clone(),
                                rule["end"].clone(),
                                rule["i_start"].clone(),
                                rule["i_end"].clone(),
                                rule["escape"] == "true",
                            ),
                            _ => unreachable!(),
                        }
                    }
                    let ext = extensions.get::<i64, String>(ext_idx)?;
                    syntax_highlighting.user_rules.insert(ext, highlighter);
                }
                Ok(())
            },
        );
        methods.add_method_mut("set", |_, syntax_highlighting, (name, value)| {
            syntax_highlighting
                .theme
                .insert(name, ConfigColor::from_lua(value));
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
            format: "".to_string(),
        }
    }
}

impl GreetingMessage {
    /// Take the configuration information and render the greeting message
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
    pub fn render(&self, document: &Document) -> String {
        let path = document
            .file_name
            .clone()
            .unwrap_or_else(|| "[No Name]".to_string());
        let file_extension = get_file_ext(&path).unwrap_or_else(|| "Unknown".to_string());
        let absolute_path = get_absolute_path(&path).unwrap_or_else(|| "[No Name]".to_string());
        let file_name = get_file_name(&path).unwrap_or_else(|| "[No Name]".to_string());
        let icon = icon(&filetype(&file_extension).unwrap_or_else(|| "".to_string()));
        let modified = if document.modified { "[+]" } else { "" };
        let mut result = self.format.clone();
        result = result
            .replace("{file_extension}", &file_extension)
            .to_string();
        result = result.replace("{file_name}", &file_name).to_string();
        result = result
            .replace("{absolute_path}", &absolute_path)
            .to_string();
        result = result.replace("{path}", &path).to_string();
        result = result.replace("{modified}", &modified).to_string();
        result = result.replace("{icon}", &icon).to_string();
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
    pub fn render(&self, editor: &Editor, lua: &Lua, w: usize) -> String {
        let mut result = vec![];
        let path = editor
            .doc()
            .file_name
            .to_owned()
            .unwrap_or_else(|| "[No Name]".to_string());
        let file_extension = get_file_ext(&path).unwrap_or_else(|| "".to_string());
        let absolute_path = get_absolute_path(&path).unwrap_or_else(|| "[No Name]".to_string());
        let file_name = get_file_name(&path).unwrap_or_else(|| "[No Name]".to_string());
        let file_type = filetype(&file_extension).unwrap_or_else(|| file_extension.to_string());
        let icon = icon(&filetype(&file_extension).unwrap_or_else(|| "".to_string()));
        let modified = if editor.doc().modified { "[+]" } else { "" };
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
            part = part.replace("{modified}", &modified).to_string();
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
        let status: Vec<&str> = result.iter().map(|s| s.as_str()).collect();
        match self.alignment {
            StatusAlign::Between => alinio::align::between(status.as_slice(), w),
            StatusAlign::Around => alinio::align::around(status.as_slice(), w),
        }
        .unwrap_or_else(|| "".to_string())
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

impl Into<String> for StatusAlign {
    fn into(self) -> String {
        match self {
            Self::Around => "around",
            Self::Between => "between",
        }
        .to_string()
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

pub fn key_to_string(modifiers: KMod, key: KCode) -> String {
    let mut result = "".to_string();
    // Deal with modifiers
    if modifiers.contains(KMod::CONTROL) {
        result += "ctrl_";
    }
    if modifiers.contains(KMod::ALT) {
        result += "alt_";
    }
    if modifiers.contains(KMod::SHIFT) {
        result += "shift_";
    }
    result += &match key {
        KCode::Char('\\') => "\\\\".to_string(),
        KCode::Char('"') => "\\\"".to_string(),
        KCode::Backspace => "backspace".to_string(),
        KCode::Enter => "enter".to_string(),
        KCode::Left => "left".to_string(),
        KCode::Right => "right".to_string(),
        KCode::Up => "up".to_string(),
        KCode::Down => "down".to_string(),
        KCode::Home => "home".to_string(),
        KCode::End => "end".to_string(),
        KCode::PageUp => "pageup".to_string(),
        KCode::PageDown => "pagedown".to_string(),
        KCode::Tab => "tab".to_string(),
        KCode::BackTab => "backtab".to_string(),
        KCode::Delete => "delete".to_string(),
        KCode::Insert => "insert".to_string(),
        KCode::F(num) => format!("f{num}"),
        KCode::Char(ch) => format!("{}", ch.to_lowercase()),
        KCode::Null => "null".to_string(),
        KCode::Esc => "esc".to_string(),
        KCode::CapsLock => "capslock".to_string(),
        KCode::ScrollLock => "scrolllock".to_string(),
        KCode::NumLock => "numlock".to_string(),
        KCode::PrintScreen => "printscreen".to_string(),
        KCode::Pause => "pause".to_string(),
        KCode::Menu => "menu".to_string(),
        KCode::KeypadBegin => "keypadbegin".to_string(),
        KCode::Media(key) => match key {
            MediaKeyCode::Play => "play",
            MediaKeyCode::Pause => "pause",
            MediaKeyCode::PlayPause => "playpause",
            MediaKeyCode::Reverse => "reverse",
            MediaKeyCode::Stop => "stop",
            MediaKeyCode::FastForward => "fastforward",
            MediaKeyCode::TrackNext => "next",
            MediaKeyCode::TrackPrevious => "previous",
            MediaKeyCode::Record => "record",
            MediaKeyCode::Rewind => "rewind",
            MediaKeyCode::LowerVolume => "lowervolume",
            MediaKeyCode::RaiseVolume => "raisevolume",
            MediaKeyCode::MuteVolume => "mutevolume",
        }
        .to_string(),
        KCode::Modifier(key) => match key {
            ModifierKeyCode::LeftShift => "lshift",
            ModifierKeyCode::LeftControl => "lctrl",
            ModifierKeyCode::LeftAlt => "lalt",
            ModifierKeyCode::LeftSuper => "lsuper",
            ModifierKeyCode::LeftHyper => "lhyper",
            ModifierKeyCode::LeftMeta => "lmeta",
            ModifierKeyCode::RightControl => "rctrl",
            ModifierKeyCode::RightAlt => "ralt",
            ModifierKeyCode::RightSuper => "rsuper",
            ModifierKeyCode::RightHyper => "rhyper",
            ModifierKeyCode::RightMeta => "rmeta",
            ModifierKeyCode::RightShift => "rshift",
            ModifierKeyCode::IsoLevel3Shift => "iso3shift",
            ModifierKeyCode::IsoLevel5Shift => "iso5shift",
        }
        .to_string(),
    };
    return result;
}

fn update_highlighter(editor: &mut Editor) {
    if let Err(err) = editor.update_highlighter() {
        editor.feedback = Feedback::Error(err.to_string());
    }
}

#[derive(Debug)]
pub struct DocumentConfig {
    pub tab_width: usize,
    pub undo_period: usize,
    pub wrap_cursor: bool,
}

impl Default for DocumentConfig {
    fn default() -> Self {
        Self {
            tab_width: 4,
            undo_period: 10,
            wrap_cursor: true,
        }
    }
}

impl LuaUserData for DocumentConfig {
    fn add_fields<'lua, F: LuaUserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field_method_get("tab_width", |_, document| Ok(document.tab_width));
        fields.add_field_method_set("tab_width", |_, this, value| {
            this.tab_width = value;
            Ok(())
        });
        fields.add_field_method_get("undo_period", |_, document| Ok(document.undo_period));
        fields.add_field_method_set("undo_period", |_, this, value| {
            this.undo_period = value;
            Ok(())
        });
        fields.add_field_method_get("wrap_cursor", |_, document| Ok(document.wrap_cursor));
        fields.add_field_method_set("wrap_cursor", |_, this, value| {
            this.wrap_cursor = value;
            Ok(())
        });
    }
}

impl LuaUserData for Editor {
    fn add_fields<'lua, F: LuaUserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field_method_get("pasting", |_, editor| Ok(editor.paste_flag));
        fields.add_field_method_get("cursor", |_, editor| {
            let loc = editor.doc().char_loc();
            Ok(LuaLoc {
                x: loc.x,
                y: loc.y + 1,
            })
        });
        fields.add_field_method_get("document_name", |_, editor| {
            let name = editor.doc().file_name.clone();
            Ok(name)
        });
        fields.add_field_method_get("document_length", |_, editor| {
            let len = editor.doc().len_lines();
            Ok(len)
        });
        fields.add_field_method_get("version", |_, _| Ok(VERSION));
        fields.add_field_method_get("current_document_id", |_, editor| Ok(editor.ptr));
        fields.add_field_method_get("document_count", |_, editor| Ok(editor.doc.len()));
        fields.add_field_method_get("help_visible", |_, editor| Ok(editor.help));
        fields.add_field_method_get("document_type", |_, editor| {
            let ext = editor
                .doc()
                .file_name
                .as_ref()
                .and_then(|name| Some(name.split('.').last().unwrap_or("")))
                .unwrap_or("");
            let file_type = kaolinite::utils::filetype(ext);
            Ok(file_type)
        });
    }

    fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        // Reload the configuration file
        methods.add_method_mut("reload_config", |lua, editor, ()| {
            if editor
                .load_config(editor.config_path.clone(), &lua)
                .is_err()
            {
                editor.feedback = Feedback::Error("Failed to reload config".to_string());
            }
            Ok(())
        });
        // Display messages
        methods.add_method_mut("display_error", |_, editor, message: String| {
            editor.feedback = Feedback::Error(message);
            Ok(())
        });
        methods.add_method_mut("display_warning", |_, editor, message: String| {
            editor.feedback = Feedback::Warning(message);
            Ok(())
        });
        methods.add_method_mut("display_info", |_, editor, message: String| {
            editor.feedback = Feedback::Info(message);
            Ok(())
        });
        // Prompt the user
        methods.add_method_mut("prompt", |_, editor, question: String| {
            Ok(editor
                .prompt(question)
                .unwrap_or_else(|_| "error".to_string()))
        });
        // Edit commands (relative)
        methods.add_method_mut("insert", |_, editor, text: String| {
            for ch in text.chars() {
                if let Err(err) = editor.character(ch) {
                    editor.feedback = Feedback::Error(err.to_string());
                }
            }
            update_highlighter(editor);
            Ok(())
        });
        methods.add_method_mut("remove", |_, editor, ()| {
            if let Err(err) = editor.backspace() {
                editor.feedback = Feedback::Error(err.to_string());
            }
            update_highlighter(editor);
            Ok(())
        });
        methods.add_method_mut("insert_line", |_, editor, ()| {
            if let Err(err) = editor.enter() {
                editor.feedback = Feedback::Error(err.to_string());
            }
            update_highlighter(editor);
            Ok(())
        });
        methods.add_method_mut("remove_line", |_, editor, ()| {
            if let Err(err) = editor.delete_line() {
                editor.feedback = Feedback::Error(err.to_string());
            }
            update_highlighter(editor);
            Ok(())
        });
        // Cursor moving
        methods.add_method_mut("move_to", |_, editor, (x, y): (usize, usize)| {
            let y = y.saturating_sub(1);
            editor.doc_mut().move_to(&Loc { x, y });
            update_highlighter(editor);
            Ok(())
        });
        methods.add_method_mut("move_up", |_, editor, ()| {
            editor.up();
            update_highlighter(editor);
            Ok(())
        });
        methods.add_method_mut("move_down", |_, editor, ()| {
            editor.down();
            update_highlighter(editor);
            Ok(())
        });
        methods.add_method_mut("move_left", |_, editor, ()| {
            editor.left();
            update_highlighter(editor);
            Ok(())
        });
        methods.add_method_mut("move_right", |_, editor, ()| {
            editor.right();
            update_highlighter(editor);
            Ok(())
        });
        methods.add_method_mut("select_up", |_, editor, ()| {
            editor.select_up();
            update_highlighter(editor);
            Ok(())
        });
        methods.add_method_mut("select_down", |_, editor, ()| {
            editor.select_down();
            update_highlighter(editor);
            Ok(())
        });
        methods.add_method_mut("select_left", |_, editor, ()| {
            editor.select_left();
            update_highlighter(editor);
            Ok(())
        });
        methods.add_method_mut("select_right", |_, editor, ()| {
            editor.select_right();
            update_highlighter(editor);
            Ok(())
        });
        methods.add_method_mut("select_all", |_, editor, ()| {
            editor.select_all();
            update_highlighter(editor);
            Ok(())
        });
        methods.add_method_mut("cut", |_, editor, ()| {
            if let Err(err) = editor.cut() {
                editor.feedback = Feedback::Error(err.to_string());
            } else {
                editor.feedback = Feedback::Info("Text cut to clipboard".to_owned());
            }
            Ok(())
        });
        methods.add_method_mut("copy", |_, editor, ()| {
            if let Err(err) = editor.copy() {
                editor.feedback = Feedback::Error(err.to_string());
            } else {
                editor.feedback = Feedback::Info("Text copied to clipboard".to_owned());
            }
            Ok(())
        });
        methods.add_method_mut("move_home", |_, editor, ()| {
            editor.doc_mut().move_home();
            update_highlighter(editor);
            Ok(())
        });
        methods.add_method_mut("move_end", |_, editor, ()| {
            editor.doc_mut().move_end();
            update_highlighter(editor);
            Ok(())
        });
        methods.add_method_mut("move_page_up", |_, editor, ()| {
            editor.doc_mut().move_page_up();
            update_highlighter(editor);
            Ok(())
        });
        methods.add_method_mut("move_page_down", |_, editor, ()| {
            editor.doc_mut().move_page_down();
            update_highlighter(editor);
            Ok(())
        });
        methods.add_method_mut("move_top", |_, editor, ()| {
            editor.doc_mut().move_top();
            update_highlighter(editor);
            Ok(())
        });
        methods.add_method_mut("move_bottom", |_, editor, ()| {
            editor.doc_mut().move_bottom();
            update_highlighter(editor);
            Ok(())
        });
        methods.add_method_mut("move_previous_word", |_, editor, ()| {
            editor.prev_word();
            update_highlighter(editor);
            Ok(())
        });
        methods.add_method_mut("move_next_word", |_, editor, ()| {
            editor.next_word();
            update_highlighter(editor);
            Ok(())
        });
        methods.add_method_mut(
            "insert_at",
            |_, editor, (text, x, y): (String, usize, usize)| {
                let y = y.saturating_sub(1);
                let location = editor.doc_mut().char_loc();
                editor.doc_mut().move_to(&Loc { x, y });
                for ch in text.chars() {
                    if let Err(err) = editor.character(ch) {
                        editor.feedback = Feedback::Error(err.to_string());
                    }
                }
                editor.doc_mut().move_to(&location);
                update_highlighter(editor);
                Ok(())
            },
        );
        methods.add_method_mut("remove_at", |_, editor, (x, y): (usize, usize)| {
            let y = y.saturating_sub(1);
            let location = editor.doc_mut().char_loc();
            editor.doc_mut().move_to(&Loc { x, y });
            if let Err(err) = editor.delete() {
                editor.feedback = Feedback::Error(err.to_string());
            }
            editor.doc_mut().move_to(&location);
            update_highlighter(editor);
            Ok(())
        });
        methods.add_method_mut("insert_line_at", |_, editor, (text, y): (String, usize)| {
            let y = y.saturating_sub(1);
            let location = editor.doc_mut().char_loc();
            if y < editor.doc().len_lines() {
                editor.doc_mut().move_to_y(y);
                editor.doc_mut().move_home();
                if let Err(err) = editor.enter() {
                    editor.feedback = Feedback::Error(err.to_string());
                }
                editor.up();
            } else {
                editor.doc_mut().move_bottom();
                if let Err(err) = editor.enter() {
                    editor.feedback = Feedback::Error(err.to_string());
                }
            }
            for ch in text.chars() {
                if let Err(err) = editor.character(ch) {
                    editor.feedback = Feedback::Error(err.to_string());
                }
            }
            editor.doc_mut().move_to(&location);
            update_highlighter(editor);
            Ok(())
        });
        methods.add_method_mut("remove_line_at", |_, editor, y: usize| {
            let y = y.saturating_sub(1);
            let location = editor.doc_mut().char_loc();
            editor.doc_mut().move_to_y(y);
            if let Err(err) = editor.delete_line() {
                editor.feedback = Feedback::Error(err.to_string());
            }
            editor.doc_mut().move_to(&location);
            update_highlighter(editor);
            Ok(())
        });
        methods.add_method_mut("open_command_line", |_, editor, ()| {
            match editor.prompt("Command") {
                Ok(command) => {
                    editor.command = Some(command);
                }
                Err(err) => {
                    editor.feedback = Feedback::Error(err.to_string());
                }
            }
            Ok(())
        });
        methods.add_method_mut("previous_tab", |_, editor, ()| {
            editor.prev();
            Ok(())
        });
        methods.add_method_mut("next_tab", |_, editor, ()| {
            editor.next();
            Ok(())
        });
        methods.add_method_mut("new", |_, editor, ()| {
            if let Err(err) = editor.new_document() {
                editor.feedback = Feedback::Error(err.to_string());
            }
            Ok(())
        });
        methods.add_method_mut("open", |_, editor, ()| {
            if let Err(err) = editor.open_document() {
                editor.feedback = Feedback::Error(err.to_string());
            }
            Ok(())
        });
        methods.add_method_mut("save", |_, editor, ()| {
            if let Err(err) = editor.save() {
                editor.feedback = Feedback::Error(err.to_string());
            }
            Ok(())
        });
        methods.add_method_mut("save_as", |_, editor, ()| {
            if let Err(err) = editor.save_as() {
                editor.feedback = Feedback::Error(err.to_string());
            }
            Ok(())
        });
        methods.add_method_mut("save_all", |_, editor, ()| {
            if let Err(err) = editor.save_all() {
                editor.feedback = Feedback::Error(err.to_string());
            }
            Ok(())
        });
        methods.add_method_mut("quit", |_, editor, ()| {
            if let Err(err) = editor.quit() {
                editor.feedback = Feedback::Error(err.to_string());
            }
            Ok(())
        });
        methods.add_method_mut("undo", |_, editor, ()| {
            if let Err(err) = editor.undo() {
                editor.feedback = Feedback::Error(err.to_string());
            }
            update_highlighter(editor);
            Ok(())
        });
        methods.add_method_mut("redo", |_, editor, ()| {
            if let Err(err) = editor.redo() {
                editor.feedback = Feedback::Error(err.to_string());
            }
            update_highlighter(editor);
            Ok(())
        });
        methods.add_method_mut("search", |_, editor, ()| {
            if let Err(err) = editor.search() {
                editor.feedback = Feedback::Error(err.to_string());
            }
            update_highlighter(editor);
            Ok(())
        });
        methods.add_method_mut("replace", |_, editor, ()| {
            if let Err(err) = editor.replace() {
                editor.feedback = Feedback::Error(err.to_string());
            }
            update_highlighter(editor);
            Ok(())
        });
        methods.add_method("get_character", |_, editor, ()| {
            let loc = editor.doc().char_loc();
            let ch = editor
                .doc()
                .line(loc.y)
                .unwrap_or_else(|| "".to_string())
                .chars()
                .nth(loc.x)
                .and_then(|ch| Some(ch.to_string()))
                .unwrap_or_else(|| "".to_string());
            Ok(ch)
        });
        methods.add_method_mut("get_character_at", |_, editor, (x, y): (usize, usize)| {
            editor.doc_mut().load_to(y);
            let y = y.saturating_sub(1);
            let ch = editor
                .doc()
                .line(y)
                .unwrap_or_else(|| "".to_string())
                .chars()
                .nth(x)
                .and_then(|ch| Some(ch.to_string()))
                .unwrap_or_else(|| "".to_string());
            update_highlighter(editor);
            Ok(ch)
        });
        methods.add_method("get_line", |_, editor, ()| {
            let loc = editor.doc().char_loc();
            let line = editor.doc().line(loc.y).unwrap_or_else(|| "".to_string());
            Ok(line)
        });
        methods.add_method_mut("get_line_at", |_, editor, y: usize| {
            editor.doc_mut().load_to(y);
            let y = y.saturating_sub(1);
            let line = editor.doc().line(y).unwrap_or_else(|| "".to_string());
            update_highlighter(editor);
            Ok(line)
        });
        methods.add_method_mut("move_to_document", |_, editor, id: usize| {
            editor.ptr = id;
            Ok(())
        });
        methods.add_method_mut("move_previous_match", |_, editor, query: String| {
            editor.prev_match(&query);
            update_highlighter(editor);
            Ok(())
        });
        methods.add_method_mut("hide_help_message", |_, editor, ()| {
            editor.help = false;
            Ok(())
        });
        methods.add_method_mut("show_help_message", |_, editor, ()| {
            editor.help = true;
            Ok(())
        });
        methods.add_method_mut("set_read_only", |_, editor, status: bool| {
            editor.doc_mut().read_only = status;
            Ok(())
        });
        methods.add_method_mut("set_file_type", |_, editor, ext: String| {
            let mut highlighter = editor
                .config
                .syntax_highlighting
                .borrow()
                .get_highlighter(&ext);
            highlighter.run(&editor.doc().lines);
            editor.highlighter[editor.ptr] = highlighter;
            Ok(())
        });
    }
}

pub struct LuaLoc {
    x: usize,
    y: usize,
}

impl IntoLua<'_> for LuaLoc {
    fn into_lua(self, lua: &Lua) -> std::result::Result<LuaValue<'_>, LuaError> {
        let table = lua.create_table()?;
        table.set("x", self.x)?;
        table.set("y", self.y)?;
        Ok(LuaValue::Table(table))
    }
}
