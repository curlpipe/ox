#![warn(clippy::all, clippy::pedantic)]

mod cli;
mod config;
mod editor;
mod error;
mod ui;

use cli::CommandLineInterface;
use config::{
    key_to_string, run_key, run_key_before, PLUGIN_BOOTSTRAP, PLUGIN_MANAGER, PLUGIN_NETWORKING,
    PLUGIN_RUN,
};
use crossterm::event::Event as CEvent;
use editor::Editor;
use error::Result;
use kaolinite::event::Event;
use kaolinite::searching::Searcher;
use kaolinite::Loc;
use mlua::Error::{RuntimeError, SyntaxError};
use mlua::Lua;
use std::cell::RefCell;
use std::rc::Rc;
use std::result::Result as RResult;
use ui::Feedback;

fn main() {
    // Interact with user to find out what they want to do
    let cli = CommandLineInterface::new();

    // Handle help and version options
    cli.basic_options();

    let result = run(&cli);
    if let Err(err) = result {
        panic!("{err:?}");
    }
}

fn run(cli: &CommandLineInterface) -> Result<()> {
    // Create lua interpreter
    let lua = Lua::new();

    // Create editor
    let editor = match Editor::new(&lua) {
        Ok(editor) => editor,
        Err(error) => panic!("Editor failed to start: {error:?}"),
    };

    // Push editor into lua
    let editor = Rc::new(RefCell::new(editor));
    lua.globals().set("editor", editor.clone())?;

    // Inject the networking library for plug-ins to use
    handle_lua_error(&editor, "", lua.load(PLUGIN_NETWORKING).exec());

    // Load config and initialise
    lua.load(PLUGIN_BOOTSTRAP).exec()?;
    let result = editor.borrow_mut().load_config(&cli.config_path, &lua);
    if let Some(err) = result {
        // Handle error if available
        handle_lua_error(&editor, "configuration", Err(err));
    };

    // Open files user has asked to open
    for (c, file) in cli.to_open.iter().enumerate() {
        // Open the file
        editor.borrow_mut().open_or_new(file.to_string())?;
        // Set read only if applicable
        if cli.flags.read_only {
            editor.borrow_mut().get_doc(c).info.read_only = true;
        }
        // Set highlighter if applicable
        if let Some(ref ext) = cli.file_type {
            let mut highlighter = editor
                .borrow()
                .config
                .syntax_highlighting
                .borrow()
                .get_highlighter(ext);
            highlighter.run(&editor.borrow_mut().get_doc(c).lines);
            *editor.borrow_mut().get_highlighter(c) = highlighter;
        }
    }

    // Handle stdin if applicable
    if cli.flags.stdin {
        let stdin = cli::get_stdin();
        editor.borrow_mut().blank()?;
        let this_doc = editor.borrow_mut().doc_len().saturating_sub(1);
        let mut holder = editor.borrow_mut();
        let doc = holder.get_doc(this_doc);
        doc.exe(Event::Insert(Loc { x: 0, y: 0 }, stdin))?;
        doc.load_to(doc.size.h);
        let lines = doc.lines.clone();
        let hl = holder.get_highlighter(this_doc);
        hl.run(&lines);
        if cli.flags.read_only {
            holder.get_doc(this_doc).info.read_only = true;
        }
    }

    // Create a blank document if none are opened
    editor.borrow_mut().new_if_empty()?;

    // Run plug-ins
    handle_lua_error(&editor, "", lua.load(PLUGIN_RUN).exec());

    // Add in the plugin manager
    handle_lua_error(&editor, "", lua.load(PLUGIN_MANAGER).exec());

    // Run the editor and handle errors if applicable
    editor.borrow_mut().init()?;
    while editor.borrow().active {
        // Render and wait for event
        editor.borrow_mut().render(&lua)?;

        // While waiting for an event to come along, service the task manager
        while let Ok(false) = crossterm::event::poll(std::time::Duration::from_millis(100)) {
            let exec = editor
                .borrow_mut()
                .config
                .task_manager
                .lock()
                .unwrap()
                .execution_list();
            for task in exec {
                if let Ok(target) = lua.globals().get::<_, mlua::Function>(task.clone()) {
                    // Run the code
                    handle_lua_error(&editor, "task", target.call(()));
                } else {
                    editor.borrow_mut().feedback =
                        Feedback::Warning(format!("Function '{task}' was not found"));
                }
            }
        }

        let event = crossterm::event::read()?;
        editor.borrow_mut().feedback = Feedback::None;

        // Handle plug-in before key press mappings
        if let CEvent::Key(key) = event {
            let key_str = key_to_string(key.modifiers, key.code);
            let code = run_key_before(&key_str);
            let result = lua.load(&code).exec();
            handle_lua_error(&editor, &key_str, result);
        }

        // Actually handle editor event (errors included)
        if let Err(err) = editor.borrow_mut().handle_event(event.clone()) {
            editor.borrow_mut().feedback = Feedback::Error(format!("{err:?}"));
        }

        // Handle plug-in after key press mappings (if no errors occured)
        if let CEvent::Key(key) = event {
            let key_str = key_to_string(key.modifiers, key.code);
            let code = run_key(&key_str);
            let result = lua.load(&code).exec();
            handle_lua_error(&editor, &key_str, result);
        }

        editor.borrow_mut().update_highlighter();
        editor.borrow_mut().greet = false;

        // Check for any commands to run
        let command = editor.borrow().command.clone();
        if let Some(command) = command {
            run_editor_command(&editor, &command, &lua);
        }
        editor.borrow_mut().command = None;
    }

    // Exit
    editor.borrow_mut().terminal.end()?;
    Ok(())
}

fn handle_lua_error(editor: &Rc<RefCell<Editor>>, key_str: &str, error: RResult<(), mlua::Error>) {
    match error {
        // All good
        Ok(()) => (),
        // Handle a runtime error
        Err(RuntimeError(msg)) => {
            let msg = msg.split('\n').collect::<Vec<&str>>();
            // Extract description
            let description = msg.first().unwrap_or(&"No Message Text");
            // See if there is any additional error location information
            let mut error_line_finder = Searcher::new(r"^\s*(.+:\d+):.*$");
            let mut location_line = msg
                .iter()
                .skip(1)
                .position(|line| error_line_finder.lfind(line).is_some());
            // Don't attach additional location if description already includes it
            if error_line_finder.lfind(description).is_some() {
                location_line = None;
            }
            // Put together the message (attaching location if not already provided)
            let msg = if let Some(trace) = location_line {
                // There is additional line info, attach it
                let location = msg[trace + 1]
                    .to_string()
                    .trim()
                    .split(':')
                    .take(2)
                    .collect::<Vec<_>>()
                    .join(" on line ");
                format!("{location}: {description}")
            } else {
                (*description).to_string()
            };
            if msg.ends_with("key not bound") {
                // Key was not bound, issue a warning would be helpful
                let key_str = key_str.replace(' ', "space");
                if key_str.contains('_') && key_str != "_" && !key_str.starts_with("shift") {
                    editor.borrow_mut().feedback =
                        Feedback::Warning(format!("The key {key_str} is not bound"));
                }
            } else if msg.ends_with("command not found") {
                // Command was not found, issue an error
                editor.borrow_mut().feedback =
                    Feedback::Error(format!("The command '{key_str}' is not defined"));
            } else {
                // Some other runtime error
                editor.borrow_mut().feedback = Feedback::Error(msg.to_string());
            }
        }
        // Handle a syntax error
        Err(SyntaxError { message, .. }) => {
            if key_str == "configuration" {
                let mut message = message.rsplit(':').take(2).collect::<Vec<&str>>();
                message.reverse();
                let message = message.join(":");
                editor.borrow_mut().feedback =
                    Feedback::Error(format!("Syntax Error in config file on line {message}"));
            } else {
                editor.borrow_mut().feedback =
                    Feedback::Error(format!("Syntax Error: {message:?}"));
            }
        }
        // Other miscellaneous error
        Err(err) => {
            editor.borrow_mut().feedback =
                Feedback::Error(format!("Failed to run Lua code: {err:?}"));
        }
    }
}

// Run a command in the editor
fn run_editor_command(editor: &Rc<RefCell<Editor>>, cmd: &str, lua: &Lua) {
    let cmd = cmd.replace('\'', "\\'").to_string();
    if let [subcmd, arguments @ ..] = cmd.split(' ').collect::<Vec<&str>>().as_slice() {
        let arguments = arguments.join("', '");
        let code =
            format!("(commands['{subcmd}'] or error('command not found'))({{'{arguments}'}})");
        handle_lua_error(editor, subcmd, lua.load(code).exec());
    }
}
