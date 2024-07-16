mod config;
mod editor;
mod error;
mod cli;
mod ui;

use kaolinite::event::Event;
use kaolinite::Loc;
use synoptic::{Highlighter, from_extension};
use error::Result;
use cli::CommandLineInterface;
use editor::Editor;
use ui::Feedback;

fn main() {
    let stdin = cli::get_stdin();
    // Interact with user to find out what they want to do
    let mut cli = CommandLineInterface::new();

    if cli.help() {
        println!("{}", cli::HELP);
        return
    } else if cli.version() {
        println!("{}", cli::VERSION);
        return
    }

    let _ = run(cli, stdin);
}

fn run(mut cli: CommandLineInterface, stdin: Option<String>) -> Result<()> {
    let read_only = cli.read_only();
    let config_path = cli.get_config_path();
    let file_type = cli.get_file_type();
    // Create editor and open requested files
    let mut editor = Editor::new(config_path)?;

    // Handle stdin if applicable
    if cli.stdin() {
        if let Some(stdin) = stdin {
            editor.blank()?;
            let this_doc = editor.doc_len().saturating_sub(1);
            let doc = editor.get_doc(this_doc);
            doc.exe(Event::Insert(Loc { x: 0, y: 0 }, stdin))?;
            doc.load_to(doc.size.h);
            let lines = doc.lines.clone();
            let hl = editor.get_highlighter(this_doc);
            hl.run(&lines);
            if read_only {
                editor.get_doc(this_doc).read_only = true;
            }
        }
    }

    // Get files user has asked to open
    cli.get_files();
    for (c, file) in cli.to_open.iter().enumerate() {
        // Open the file
        editor.open_or_new(file.to_string())?;
        // Set read only if applicable
        if read_only {
            editor.get_doc(c).read_only = true;
        }
        // Set highlighter if applicable
        if let Some(ref ext) = file_type {
            let mut highlighter = from_extension(&ext, 4)
                .unwrap_or_else(|| Highlighter::new(4));
            highlighter.run(&editor.get_doc(c).lines);
            *editor.get_highlighter(c) = highlighter;
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
