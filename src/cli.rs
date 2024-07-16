use jargon_args::{Key, Jargon};

/// Holds the version number of the crate
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Holds the help dialog
pub const HELP: &str = "\
Ox: A lightweight and flexible text editor

USAGE: ox [options] [files]

OPTIONS:
  --help, -h                 : Show this help message
  --version, -v              : Show the version number
  --config [path], -c [path] : Specify the configuration file
  --readonly, -r             : Prevent opened files from writing
  --filetype [ext], -f [ext] : Set the file type of files opened

EXAMPLES:
  ox
  ox test.txt
  ox test.txt test2.txt
  ox /home/user/docs/test.txt
  ox -c config.lua test.txt
  ox -r -c ~/.config/.oxrc -f lua my_file.lua\
";

/// Struct to help with starting ox
pub struct CommandLineInterface {
    jargon: Jargon,
    pub to_open: Vec<String>,
}

impl CommandLineInterface {
    /// Create a new command line interface helper
    pub fn new() -> Self {
        Self { 
            jargon: Jargon::from_env(), 
            to_open: vec![],
        }
    }

    /// Determine if the user wishes to see the help message
    pub fn help(&mut self) -> bool {
        self.jargon.contains(["-h", "--help"])
    }

    /// Determine if the user wishes to see the version
    pub fn version(&mut self) -> bool {
        self.jargon.contains(["-v", "--version"])
    }

    /// Determine if the user wishes to open files in read only
    pub fn read_only(&mut self) -> bool {
        self.jargon.contains(["-r", "--readonly"])
    }

    /// Get all the files the user has requested
    pub fn get_files(&mut self) {
        self.to_open = self.jargon.clone().finish();
    }

    /// Get file types
    pub fn get_file_type(&mut self) -> Option<String> {
        let filetype_key: Key = ["-f", "--filetype"].into();
        self.jargon.option_arg::<String, Key>(filetype_key.clone())
    }
    /// Configuration file path
    pub fn get_config_path(&mut self) -> String {
        let config_key: Key = ["-c", "--config"].into();
        match self.jargon.option_arg::<String, Key>(config_key.clone()) {
            Some(config) => config,
            None => "~/.oxrc".to_string(),
        }
    }
}
