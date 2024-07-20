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
use std::rc::Rc;
use std::cell::RefCell;
use mlua::Lua;
use mlua::Error::RuntimeError;

fn main() {
    // Interact with user to find out what they want to do
    let cli = CommandLineInterface::new();

    // Handle help and version options
    cli.basic_options();

    let _ = run(cli);
}

fn run(cli: CommandLineInterface) -> Result<()> {
    // Create lua interpreter
    let lua = Lua::new();

    // Create editor
    let mut editor = match Editor::new(&lua) {
        Ok(editor) => editor,
        Err(error) => panic!("Editor failed to start: {:?}", error),
    };

    // Handle stdin if applicable
    if cli.stdin {
        if let Some(stdin) = cli::get_stdin() {
            editor.blank()?;
            let this_doc = editor.doc_len().saturating_sub(1);
            let doc = editor.get_doc(this_doc);
            doc.exe(Event::Insert(Loc { x: 0, y: 0 }, stdin))?;
            doc.load_to(doc.size.h);
            let lines = doc.lines.clone();
            let hl = editor.get_highlighter(this_doc);
            hl.run(&lines);
            if cli.read_only {
                editor.get_doc(this_doc).read_only = true;
            }
        }
    }

    // Open files user has asked to open
    for (c, file) in cli.to_open.iter().enumerate() {
        // Open the file
        editor.open_or_new(file.to_string())?;
        // Set read only if applicable
        if cli.read_only {
            editor.get_doc(c).read_only = true;
        }
        // Set highlighter if applicable
        if let Some(ref ext) = cli.file_type {
            let mut highlighter = from_extension(&ext, 4)
                .unwrap_or_else(|| Highlighter::new(4));
            highlighter.run(&editor.get_doc(c).lines);
            *editor.get_highlighter(c) = highlighter;
        }
    }

    // Push editor into lua
    let editor = Rc::new(RefCell::new(editor));
    lua.globals().set("editor", editor.clone())?;

    // Load config and initialise
    editor.borrow_mut().load_config(cli.config_path, &lua).unwrap();
    editor.borrow_mut().init()?;

    // Run the editor and handle errors if applicable
    while editor.borrow().active {
        let cycle = editor.borrow_mut().cycle();
        match cycle {
            Ok(Some(key)) => {
                // Form the corresponding lua code to run and run it
                let code = format!("(event_mapping[\"{key}\"] or error(\"key not bound\"))()");
                let result = lua.load(&code).exec();
                // Check the result
                match result {
                    // Handle any runtime errors
                    Err(RuntimeError(msg)) => {
                        // Work out if the key has been bound
                        if msg.contains(&"key not bound") {
                            if key.contains(&"_") {
                                editor.borrow_mut().feedback = Feedback::Error(format!("The key binding {key} is not set"));
                            }
                        } else {
                            let mut msg: String = msg.split("\n").next().unwrap().to_string();
                            // Work out the type of error and display an appropriate message
                            if msg.starts_with("[") {
                                msg = msg.split(":").skip(3).collect::<Vec<&str>>().join(":");
                                msg = format!(" on line {msg}");
                            } else {
                                msg = format!(": {msg}");
                            }
                            // Send out the error
                            editor.borrow_mut().feedback = Feedback::Error(format!("Lua error occured{msg}"));
                        }
                    }
                    Err(err) => {
                        editor.borrow_mut().feedback = Feedback::Error(format!("Error occured: {err}"));
                    }
                    _ => (),
                }
            }
            // Display error from editor cycle
            Err(e) => editor.borrow_mut().feedback = Feedback::Error(e.to_string()),
            _ => (),
        }
        editor.borrow_mut().update_highlighter()?;
        editor.borrow_mut().greet = false;
        // Check for any commands to run
        let command = editor.borrow().command.clone();
        if let Some(command) = command {
            run_editor_command(&editor, command, &lua)
        }
        editor.borrow_mut().command = None;
    }

    // Exit
    editor.borrow_mut().terminal.end()?;
    Ok(())
}

fn run_editor_command(editor: &Rc<RefCell<Editor>>, cmd: String, lua: &Lua) {
    let cmd = cmd.replace("'", "\\'").to_string();
    match cmd.split(' ').collect::<Vec<&str>>().as_slice() {
        ["filetype", ext] => {
            // Change the highlighter of the current file
            editor.borrow_mut().highlighter[editor.borrow().ptr] = from_extension(ext, 4)
                .unwrap_or_else(|| Highlighter::new(4));
        }
        ["readonly", "true"] => editor.borrow_mut().doc_mut().read_only = true,
        ["readonly", "false"] => editor.borrow_mut().doc_mut().read_only = false,
        ["help"] => editor.borrow_mut().help = !editor.borrow().help,
        [subcmd, arguments @ ..] => {
            let arguments = arguments.join("', '");
            let code = format!("commands['{subcmd}']({{'{arguments}'}})");
            if let Err(err) = lua.load(code).exec() {
                let line = err.to_string().split("\n").next().unwrap().to_string();
                editor.borrow_mut().feedback = Feedback::Error(line);
            }
        }
        _ => (),
    }
}
