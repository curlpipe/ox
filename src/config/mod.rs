/// For general configuration
use crate::editor::{FileType, FileTypes};
use crate::error::{OxError, Result};
use mlua::prelude::*;
use std::fmt::{Display, Error, Formatter};
use std::sync::{Arc, Mutex};

mod assistant;
mod colors;
mod editor;
mod filetree;
mod highlighting;
mod interface;
mod keys;
mod runner;
mod tasks;

pub use assistant::Assistant;
pub use colors::{Color, Colors};
pub use filetree::FileTree;
pub use highlighting::SyntaxHighlighting;
pub use interface::{GreetingMessage, HelpMessage, LineNumbers, StatusLine, TabLine, Terminal};
pub use keys::{get_listeners, key_to_string, run_key, run_key_before};
pub use tasks::TaskManager;

/// Issue a warning to the user
fn issue_warning(msg: &str) {
    eprintln!("[WARNING] {msg}");
}

/// This contains the default configuration lua file
const DEFAULT_CONFIG: &str = include_str!("../../config/.oxrc");

/// Default plug-in code to use
const PAIRS: &str = include_str!("../../plugins/pairs.lua");
const AUTOINDENT: &str = include_str!("../../plugins/autoindent.lua");
const QUICKCOMMENT: &str = include_str!("../../plugins/quickcomment.lua");

/// This contains the code for setting up plug-in infrastructure
pub const PLUGIN_BOOTSTRAP: &str = include_str!("../plugin/bootstrap.lua");

/// This contains the code for running the plugins
pub const PLUGIN_RUN: &str = include_str!("../plugin/run.lua");

/// This contains the code for plug-ins to use networking
pub const PLUGIN_NETWORKING: &str = include_str!("../plugin/networking.lua");

/// This contains the code for running the plugins
pub const PLUGIN_MANAGER: &str = include_str!("../plugin/plugin_manager.lua");

/// A nice macro to quickly interpret configuration
#[macro_export]
macro_rules! config {
    ($cfg:expr, document) => {
        $cfg.document.borrow::<$crate::config::Document>().unwrap()
    };
    ($cfg:expr, colors) => {
        $cfg.colors.borrow::<$crate::config::Colors>().unwrap()
    };
    ($cfg:expr, syntax) => {
        $cfg.syntax_highlighting
            .borrow::<$crate::config::SyntaxHighlighting>()
            .unwrap()
    };
    ($cfg:expr, line_numbers) => {
        $cfg.line_numbers
            .borrow::<$crate::config::LineNumbers>()
            .unwrap()
    };
    ($cfg:expr, status_line) => {
        $cfg.status_line
            .borrow::<$crate::config::StatusLine>()
            .unwrap()
    };
    ($cfg:expr, tab_line) => {
        $cfg.tab_line.borrow::<$crate::config::TabLine>().unwrap()
    };
    ($cfg:expr, greeting_message) => {
        $cfg.greeting_message
            .borrow::<$crate::config::GreetingMessage>()
            .unwrap()
    };
    ($cfg:expr, help_message) => {
        $cfg.help_message
            .borrow::<$crate::config::HelpMessage>()
            .unwrap()
    };
    ($cfg:expr, file_tree) => {
        $cfg.file_tree.borrow::<$crate::config::FileTree>().unwrap()
    };
    ($cfg:expr, terminal) => {
        $cfg.terminal.borrow::<$crate::config::Terminal>().unwrap()
    };
}

/// The struct that holds all the configuration information
#[derive(Debug)]
pub struct Config {
    pub syntax_highlighting: LuaAnyUserData,
    pub line_numbers: LuaAnyUserData,
    pub colors: LuaAnyUserData,
    pub status_line: LuaAnyUserData,
    pub tab_line: LuaAnyUserData,
    pub greeting_message: LuaAnyUserData,
    pub help_message: LuaAnyUserData,
    pub file_tree: LuaAnyUserData,
    pub terminal: LuaAnyUserData,
    pub document: LuaAnyUserData,
    pub task_manager: Arc<Mutex<TaskManager>>,
}

impl Config {
    /// Take a lua instance, inject all the configuration tables and return a default config struct
    pub fn new(lua: &Lua) -> Result<Self> {
        // Set up structs to populate (the default values will be thrown away)
        let syntax_highlighting = lua.create_userdata(SyntaxHighlighting::default())?;
        let line_numbers = lua.create_userdata(LineNumbers::default())?;
        let greeting_message = lua.create_userdata(GreetingMessage::default())?;
        let help_message = lua.create_userdata(HelpMessage::default())?;
        let colors = lua.create_userdata(Colors::default())?;
        let status_line = lua.create_userdata(StatusLine::default())?;
        let tab_line = lua.create_userdata(TabLine::default())?;
        let file_tree = lua.create_userdata(FileTree::default())?;
        let terminal = lua.create_userdata(Terminal::default())?;
        let document = lua.create_userdata(Document::default())?;

        // Set up the task manager
        let task_manager = Arc::new(Mutex::new(TaskManager::default()));
        let task_manager_clone = Arc::clone(&task_manager);
        std::thread::spawn(move || loop {
            task_manager_clone.lock().unwrap().cycle();
            std::thread::sleep(std::time::Duration::from_secs(1));
        });

        // Push in configuration globals
        lua.globals().set("syntax", syntax_highlighting.clone())?;
        lua.globals().set("line_numbers", line_numbers.clone())?;
        lua.globals()
            .set("greeting_message", greeting_message.clone())?;
        lua.globals().set("help_message", help_message.clone())?;
        lua.globals().set("status_line", status_line.clone())?;
        lua.globals().set("tab_line", tab_line.clone())?;
        lua.globals().set("file_tree", file_tree.clone())?;
        lua.globals().set("colors", colors.clone())?;
        lua.globals().set("terminal", terminal.clone())?;
        lua.globals().set("document", document.clone())?;

        // Define task list
        let task_manager_clone = Arc::clone(&task_manager);
        let get_task_list = lua.create_function(move |_, ()| {
            Ok(format!(
                "{:?}",
                task_manager_clone.lock().unwrap().execution_list()
            ))
        })?;
        lua.globals().set("get_task_list", get_task_list)?;

        // Provide a function "after" to run a function after n seconds
        let task_manager_clone = Arc::clone(&task_manager);
        let after = lua.create_function(move |_, args: (isize, String)| {
            let (delay, target) = args;
            task_manager_clone
                .lock()
                .unwrap()
                .attach(delay, target, false);
            Ok(())
        })?;
        lua.globals().set("after", after)?;

        // Provide a function "every" to run a function every n seconds
        let task_manager_clone = Arc::clone(&task_manager);
        let every = lua.create_function(move |_, args: (isize, String)| {
            let (delay, target) = args;
            task_manager_clone
                .lock()
                .unwrap()
                .attach(delay, target, true);
            Ok(())
        })?;
        lua.globals().set("every", every)?;

        Ok(Config {
            syntax_highlighting,
            line_numbers,
            colors,
            status_line,
            tab_line,
            greeting_message,
            help_message,
            file_tree,
            terminal,
            document,
            task_manager,
        })
    }

    /// Actually take the configuration file, open it and interpret it
    pub fn read(path: &str, lua: &Lua) -> Result<()> {
        // Load the default config to start with
        lua.load(DEFAULT_CONFIG).exec()?;

        // Attempt to read config file from home directory
        let user_provided = Self::get_user_provided_config(path);
        let mut user_provided_config = false;
        if let Some(config) = user_provided {
            // Reset plugin status based on built-in configuration file
            lua.load("plugins = {}").exec()?;
            lua.load("builtins = {}").exec()?;
            // Load in user-defined configuration file
            lua.load(config).exec()?;
            user_provided_config = true;
        }

        // Determine whether or not to load built-in plugins
        let builtins: Vec<(&str, &str)> = vec![
            ("autoindent.lua", AUTOINDENT),
            ("quickcomment.lua", QUICKCOMMENT),
            ("pairs.lua", PAIRS),
        ];
        for (name, code) in &builtins {
            if Self::load_bi(name, user_provided_config, lua) {
                lua.load(*code).exec()?;
            }
        }

        // Return result
        if user_provided_config {
            Ok(())
        } else {
            let msg = "Not Found".to_string();
            Err(OxError::Config { msg })
        }
    }

    /// Read the user-provided config
    pub fn get_user_provided_config(path: &str) -> Option<String> {
        if let Ok(path) = shellexpand::full(&path) {
            if let Ok(config) = std::fs::read_to_string(path.to_string()) {
                return Some(config);
            }
        }
        None
    }

    /// Decide whether to load a built-in plugin
    pub fn load_bi(name: &str, user_provided_config: bool, lua: &Lua) -> bool {
        if user_provided_config {
            // Get list of requested built-in plugins
            let plugins: Vec<String> = lua
                .globals()
                .get::<LuaTable>("builtins")
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
        } else {
            // User hasn't provided configuration file, check for local copy
            !lua.globals()
                .get::<LuaTable>("plugins")
                .unwrap()
                .sequence_values()
                .filter_map(std::result::Result::ok)
                .any(|p: String| p.ends_with(name))
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum Indentation {
    Tabs,
    Spaces,
}

impl Display for Indentation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::result::Result<(), Error> {
        write!(
            f,
            "{}",
            match self {
                Self::Tabs => "tabs",
                Self::Spaces => "spaces",
            }
        )
    }
}

impl From<String> for Indentation {
    /// Interpret an indentation setting from a string format
    fn from(s: String) -> Self {
        match s.as_str() {
            "spaces" => Self::Spaces,
            _ => Self::Tabs,
        }
    }
}

#[derive(Debug)]
pub struct Document {
    pub tab_width: usize,
    pub indentation: Indentation,
    pub undo_period: usize,
    pub wrap_cursor: bool,
    pub file_types: FileTypes,
}

impl Default for Document {
    fn default() -> Self {
        Self {
            tab_width: 4,
            indentation: Indentation::Tabs,
            undo_period: 10,
            wrap_cursor: true,
            file_types: FileTypes::default(),
        }
    }
}

impl LuaUserData for Document {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("tab_width", |_, document| Ok(document.tab_width));
        fields.add_field_method_set("tab_width", |_, this, value| {
            this.tab_width = value;
            Ok(())
        });
        fields.add_field_method_get("indentation", |_, document| {
            Ok(document.indentation.to_string())
        });
        fields.add_field_method_set("indentation", |_, this, value: String| {
            this.indentation = value.into();
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

impl FromLua for FileTypes {
    fn from_lua(value: LuaValue, lua: &Lua) -> std::result::Result<Self, LuaError> {
        let mut result = vec![];
        if let LuaValue::Table(table) = value {
            for i in table.pairs::<String, LuaTable>() {
                let (name, info) = i?;
                let icon = info.get::<String>("icon")?;
                let extensions = info
                    .get::<LuaTable>("extensions")
                    .unwrap_or(lua.create_table()?)
                    .pairs::<usize, String>()
                    .filter_map(|val| if let Ok((_, v)) = val { Some(v) } else { None })
                    .collect::<Vec<String>>();
                let files = info
                    .get::<LuaTable>("files")
                    .unwrap_or(lua.create_table()?)
                    .pairs::<usize, String>()
                    .filter_map(|val| if let Ok((_, v)) = val { Some(v) } else { None })
                    .collect::<Vec<String>>();
                let modelines = info
                    .get::<LuaTable>("modelines")
                    .unwrap_or(lua.create_table()?)
                    .pairs::<usize, String>()
                    .filter_map(|val| if let Ok((_, v)) = val { Some(v) } else { None })
                    .collect::<Vec<String>>();
                let color = info.get::<String>("color")?;
                result.push(FileType {
                    name,
                    icon,
                    files,
                    extensions,
                    modelines,
                    color,
                });
            }
        }
        Ok(Self { types: result })
    }
}
