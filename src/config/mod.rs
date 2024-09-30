use crate::error::{OxError, Result};
use mlua::prelude::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::{
    cell::RefCell,
    fmt::{Display, Error, Formatter},
    rc::Rc,
};

mod colors;
mod editor;
mod highlighting;
mod interface;
mod keys;
mod tasks;

pub use colors::{Color, Colors};
pub use highlighting::SyntaxHighlighting;
pub use interface::{GreetingMessage, HelpMessage, LineNumbers, StatusLine, TabLine, Terminal};
pub use keys::{key_to_string, run_key, run_key_before};
pub use tasks::TaskManager;

// Issue a warning to the user
fn issue_warning(msg: &str) {
    eprintln!("[WARNING] {msg}");
}

/// This contains the default configuration lua file
const DEFAULT_CONFIG: &str = include_str!("../../config/.oxrc");

/// Default plug-in code to use
const PAIRS: &str = include_str!("../../plugins/pairs.lua");
const AUTOINDENT: &str = include_str!("../../plugins/autoindent.lua");

/// This contains the code for setting up plug-in infrastructure
pub const PLUGIN_BOOTSTRAP: &str = include_str!("../plugin/bootstrap.lua");

/// This contains the code for running the plugins
pub const PLUGIN_RUN: &str = include_str!("../plugin/run.lua");

/// This contains the code for running the plugins
pub const PLUGIN_MANAGER: &str = include_str!("../plugin/plugin_manager.lua");

/// The struct that holds all the configuration information
#[derive(Debug)]
pub struct Config {
    pub syntax_highlighting: Rc<RefCell<SyntaxHighlighting>>,
    pub line_numbers: Rc<RefCell<LineNumbers>>,
    pub colors: Rc<RefCell<Colors>>,
    pub status_line: Rc<RefCell<StatusLine>>,
    pub tab_line: Rc<RefCell<TabLine>>,
    pub greeting_message: Rc<RefCell<GreetingMessage>>,
    pub help_message: Rc<RefCell<HelpMessage>>,
    pub terminal: Rc<RefCell<Terminal>>,
    pub document: Rc<RefCell<Document>>,
    pub task_manager: Arc<Mutex<TaskManager>>,
}

impl Config {
    /// Take a lua instance, inject all the configuration tables and return a default config struct
    pub fn new(lua: &Lua) -> Result<Self> {
        // Set up structs to populate (the default values will be thrown away)
        let syntax_highlighting = Rc::new(RefCell::new(SyntaxHighlighting::default()));
        let line_numbers = Rc::new(RefCell::new(LineNumbers::default()));
        let greeting_message = Rc::new(RefCell::new(GreetingMessage::default()));
        let help_message = Rc::new(RefCell::new(HelpMessage::default()));
        let colors = Rc::new(RefCell::new(Colors::default()));
        let status_line = Rc::new(RefCell::new(StatusLine::default()));
        let tab_line = Rc::new(RefCell::new(TabLine::default()));
        let terminal = Rc::new(RefCell::new(Terminal::default()));
        let document = Rc::new(RefCell::new(Document::default()));

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
            terminal,
            document,
            task_manager,
        })
    }

    /// Actually take the configuration file, open it and interpret it
    pub fn read(&mut self, path: &str, lua: &Lua) -> Result<()> {
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
        for (name, code) in &builtins {
            if Self::load_bi(name, user_provided_config, lua) {
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
    pub fn load_bi(name: &str, user_provided_config: bool, lua: &Lua) -> bool {
        if user_provided_config {
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
        } else {
            // Load when the user hasn't provided a configuration file
            true
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
}

impl Default for Document {
    fn default() -> Self {
        Self {
            tab_width: 4,
            indentation: Indentation::Tabs,
            undo_period: 10,
            wrap_cursor: true,
        }
    }
}

impl LuaUserData for Document {
    fn add_fields<'lua, F: LuaUserDataFields<'lua, Self>>(fields: &mut F) {
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
