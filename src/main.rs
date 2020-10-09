#![warn(clippy::all, clippy::pedantic)]
#![allow(clippy::cast_possible_truncation, clippy::used_underscore_binding)]

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
mod row;
mod terminal;
mod undo;
mod util;

use clap::{App, Arg};
use document::Document;
use editor::{Editor, Position};
use row::Row;
use std::time::Duration;
use std::{env, panic, thread};
use terminal::Terminal;
use undo::{Event, EventStack};

// Get the current version of Ox
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    // Attempt to start an editor instance
    let result = panic::catch_unwind(|| {
        let config_path = match env::var("XDG_CONFIG_HOME") {
            Ok(path) => format!("{}/ox/ox.ron", path),
            Err(_) => "~/.config/ox/ox.ron".to_string(),
        };
        // Gather the command line arguments
        let cli = App::new("Ox")
            .version(VERSION)
            .author("Author: Luke <https://github.com/curlpipe>")
            .about("An independent Rust powered text editor")
            .arg(
                Arg::with_name("files")
                    .multiple(true)
                    .takes_value(true)
                    .help("The files you wish to edit"),
            )
            .arg(
                Arg::with_name("config")
                    .long("config")
                    .short("c")
                    .takes_value(true)
                    .default_value(&config_path)
                    .help("The directory of the config file"),
            );
        // Fire up the editor, ensuring that no start up problems occured
        if let Ok(mut editor) = Editor::new(cli) {
            editor.run();
        }
    });
    // Check to see if the editor exited because of a runtime issue
    if result.is_err() {
        // Pause for a few seconds to catch debug information
        thread::sleep(Duration::from_secs(5));
    }
}
