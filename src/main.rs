#![warn(clippy::all, clippy::pedantic)]
#![allow(
    clippy::cast_possible_truncation,
    clippy::used_underscore_binding,
    clippy::cast_sign_loss
)]

/*
    Ox editor is a text editor written in the Rust programming language.

    It runs in the terminal and provides keyboard shortcuts to interact.
    This removes the need for a mouse which can slow down editing files.
    I have documented this code where necessary and it has been formatted
    with Rustfmt to ensure clean and consistent style throughout.

    More information here:
    https://rust-lang.org
    https://github.com/rust-lang/rustfmt
    https://github.com/curlpipe/ox
*/

// Bring in the external modules
mod config;
mod document;
mod editor;
mod highlight;
mod oxa;
mod row;
mod terminal;
mod undo;
mod util;

use clap::{App, Arg};
use directories::BaseDirs;
use document::Document;
use editor::{Direction, Editor, Position};
use oxa::Variable;
use row::Row;
use std::fs::OpenOptions;
use std::io::Write;
use std::{env, panic};
use terminal::{Size, Terminal};
use undo::{Event, EventStack};

// Create log macro
#[macro_export]
macro_rules! log {
    ($type:literal, $msg:expr) => {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/ox.log");
        if let Ok(mut log) = file {
            writeln!(log, "{}: {}", $type, $msg).unwrap();
        } else {
            panic!("{:?}", file);
        }
    };
}

// Get the current version of Ox
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    log!("Ox started", "Ox has just been started");
    // Set up panic hook in case of unexpected crash
    panic::set_hook(Box::new(|e| {
        // Reenter canonical mode
        Terminal::exit();
        // Set hook to log crash reason
        log!("Unexpected panic", e);
        // Print panic info
        eprintln!("{}", e);
    }));
    // Attempt to start an editor instance
    let config_dir = load_config().unwrap_or_else(|| " ~/.config/ox/ox.ron".to_string());
    // Gather the command line arguments
    let cli = App::new("Ox")
        .version(VERSION)
        .author("Author: Luke <https://github.com/curlpipe>")
        .about("An independent Rust powered text editor")
        .arg(
            Arg::with_name("files")
                .multiple(true)
                .takes_value(true)
                .help(
                    r#"The files you wish to edit
You can also provide the line number to jump to by doing this:
file.txt:100 (This will go to line 100 in file.txt)"#,
                ),
        )
        .arg(
            Arg::with_name("readonly")
                .long("readonly")
                .short("r")
                .takes_value(false)
                .required(false)
                .help("Enable read only mode"),
        )
        .arg(
            Arg::with_name("config")
                .long("config")
                .short("c")
                .takes_value(true)
                .default_value(&config_dir)
                .help("The directory of the config file"),
        );
    // Fire up the editor, ensuring that no start up problems occured
    if let Ok(mut editor) = Editor::new(cli) {
        editor.run();
    }
}

fn load_config() -> Option<String> {
    // Load the configuration file
    let base_dirs = BaseDirs::new()?;
    Some(format!(
        "{}/ox/ox.ron",
        base_dirs.config_dir().to_str()?.to_string()
    ))
}
