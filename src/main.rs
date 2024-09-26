mod cli;
mod config;
mod editor;
mod error;
mod ui;

use cli::CommandLineInterface;
use config::{run_key, PLUGIN_BOOTSTRAP, PLUGIN_RUN};
use editor::Editor;
use error::Result;
use kaolinite::event::Event;
use kaolinite::Loc;
use mlua::Error::RuntimeError;
use mlua::Lua;
use std::cell::RefCell;
use std::rc::Rc;
use ui::Feedback;

fn main() {
    // Interact with user to find out what they want to do
    let cli = CommandLineInterface::new();

    // Handle help and version options
    cli.basic_options();

    let result = run(cli);
    if let Err(err) = result {
        panic!("{:?}", err);
    }
}

fn run(cli: CommandLineInterface) -> Result<()> {
    // Create lua interpreter
    let lua = Lua::new();

    // Create editor
    let editor = match Editor::new(&lua) {
        Ok(editor) => editor,
        Err(error) => panic!("Editor failed to start: {:?}", error),
    };

    // Push editor into lua
    let editor = Rc::new(RefCell::new(editor));
    lua.globals().set("editor", editor.clone())?;

    // Load config and initialise
    lua.load(PLUGIN_BOOTSTRAP).exec()?;
    if editor
        .borrow_mut()
        .load_config(cli.config_path, &lua)
        .is_err()
    {
        editor.borrow_mut().feedback =
            Feedback::Error("Failed to load configuration file".to_string());
    }

    // Open files user has asked to open
    for (c, file) in cli.to_open.iter().enumerate() {
        // Open the file
        editor.borrow_mut().open_or_new(file.to_string())?;
        // Set read only if applicable
        if cli.read_only {
            editor.borrow_mut().get_doc(c).read_only = true;
        }
        // Set highlighter if applicable
        if let Some(ref ext) = cli.file_type {
            let mut highlighter = editor
                .borrow()
                .config
                .syntax_highlighting
                .borrow()
                .get_highlighter(&ext);
            highlighter.run(&editor.borrow_mut().get_doc(c).lines);
            *editor.borrow_mut().get_highlighter(c) = highlighter;
        }
    }

    // Handle stdin if applicable
    if cli.stdin {
        if let Some(stdin) = cli::get_stdin() {
            editor.borrow_mut().blank()?;
            let this_doc = editor.borrow_mut().doc_len().saturating_sub(1);
            let mut holder = editor.borrow_mut();
            let doc = holder.get_doc(this_doc);
            doc.exe(Event::Insert(Loc { x: 0, y: 0 }, stdin))?;
            doc.load_to(doc.size.h);
            let lines = doc.lines.clone();
            let hl = holder.get_highlighter(this_doc);
            hl.run(&lines);
            if cli.read_only {
                editor.borrow_mut().get_doc(this_doc).read_only = true;
            }
        }
    }

    // Create a blank document if none are opened
    editor.borrow_mut().new_if_empty()?;

    // Run plug-ins
    lua.load(PLUGIN_RUN).exec()?;

    // Run the editor and handle errors if applicable
    editor.borrow_mut().init()?;
    while editor.borrow().active {
        let cycle = editor.borrow_mut().cycle(&lua);
        match cycle {
            Ok(Some(mut key)) => {
                // Form the corresponding lua code to run and run it
                let code = run_key(&key);
                let result = lua.load(&code).exec();
                // Check the result
                match result {
                    // Handle any runtime errors
                    Err(RuntimeError(msg)) => {
                        // Work out if the key has been bound
                        if msg.contains(&"key not bound") {
                            if key.contains(&"_") && key != "_" && !key.starts_with("shift") {
                                if key.ends_with(" ") {
                                    key.pop();
                                    key = format!("{key}space");
                                }
                                editor.borrow_mut().feedback =
                                    Feedback::Error(format!("The key binding {key} is not set"));
                            }
                        } else {
                            let mut msg: String = msg.split("\n").next().unwrap_or("").to_string();
                            // Work out the type of error and display an appropriate message
                            if msg.starts_with("[") {
                                msg = msg.split(":").skip(3).collect::<Vec<&str>>().join(":");
                                msg = format!(" on line {msg}");
                            } else {
                                msg = format!(": {msg}");
                            }
                            // Send out the error
                            editor.borrow_mut().feedback =
                                Feedback::Error(format!("Lua error occured{msg}"));
                        }
                    }
                    Err(err) => {
                        editor.borrow_mut().feedback =
                            Feedback::Error(format!("Error occured: {err}"));
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
        [subcmd, arguments @ ..] => {
            let arguments = arguments.join("', '");
            let code = format!("commands['{subcmd}']({{'{arguments}'}})");
            if let Err(err) = lua.load(code).exec() {
                let line = err.to_string().split("\n").next().unwrap_or("").to_string();
                editor.borrow_mut().feedback = Feedback::Error(line);
            }
        }
        _ => (),
    }
}
