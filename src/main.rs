mod config;
mod editor;
mod error;
mod cli;
mod ui;

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

    cli.get_files();

    let _ = run(cli);
}

fn run(cli: CommandLineInterface) -> Result<()> {
    // Create editor and open requested files
    let mut editor = Editor::new()?;
    for file in cli.to_open {
        editor.open_or_new(file)?;
    }

    // Run the editor and handle errors if applicable
    while let Err(e) = editor.run() {
        //editor.terminal.end()?;
        //eprintln!("{}", e);
        editor.feedback = Feedback::Error(e.to_string());
    }

    Ok(())
}
