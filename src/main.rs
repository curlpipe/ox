mod config;
mod editor;
mod error;
mod cli;
mod ui;

use synoptic::{Highlighter, from_extension};
use error::Result;
use cli::CommandLineInterface;
use editor::Editor;
use ui::Feedback;

fn main() {
    // Interact with user to find out what they want to do
    let mut cli = CommandLineInterface::new();

    if cli.help() {
        println!("{}", cli::HELP);
        return
    } else if cli.version() {
        println!("{}", cli::VERSION);
        return
    }

    let _ = run(cli);
}

fn run(mut cli: CommandLineInterface) -> Result<()> {
    let read_only = cli.read_only();
    let config_path = cli.get_config_path();
    let file_type = cli.get_file_type();
    cli.get_files();
    // Create editor and open requested files
    let mut editor = Editor::new(config_path)?;
    for (c, file) in cli.to_open.iter().enumerate() {
        // Open the file
        editor.open_or_new(file.to_string())?;
        // Set read only if applicable
        if read_only {
            editor.set_readonly(c)?;
        }
        // Set highlighter if applicable
        if let Some(ref ext) = file_type {
            let highlighter = from_extension(&ext, 4)
                .unwrap_or_else(|| Highlighter::new(4));
            editor.set_highlighter(highlighter, c)?;
        }
    }

    // Run the editor and handle errors if applicable
    while let Err(e) = editor.run() {
        //editor.terminal.end()?;
        //eprintln!("{}", e);
        editor.feedback = Feedback::Error(e.to_string());
    }

    Ok(())
}
