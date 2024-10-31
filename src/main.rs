#![warn(clippy::all, clippy::pedantic)]

mod cli;
mod config;
mod editor;
mod error;
mod ui;

use cli::CommandLineInterface;
use config::{
    get_listeners, key_to_string, run_key, run_key_before, Assistant, Config, PLUGIN_BOOTSTRAP,
    PLUGIN_MANAGER, PLUGIN_NETWORKING, PLUGIN_RUN,
};
use crossterm::event::{Event as CEvent, KeyEvent, KeyEventKind};
use editor::{Editor, FileTypes};
use error::{OxError, Result};
use kaolinite::event::{Error as KError, Event};
use kaolinite::searching::Searcher;
use kaolinite::utils::file_or_dir;
use kaolinite::Loc;
use mlua::Error::{RuntimeError, SyntaxError};
use mlua::{FromLua, Lua, Value};
use std::cell::RefCell;
use std::io::ErrorKind;
use std::rc::Rc;
use std::result::Result as RResult;
use ui::{fatal_error, Feedback};

/// Entry point - grabs command line arguments and runs the editor
fn main() {
    // Interact with user to find out what they want to do
    let cli = CommandLineInterface::new();

    // Handle help and version options
    cli.basic_options();

    // Activate configuration assistant if applicable
    let no_config = Config::get_user_provided_config(&cli.config_path).is_none();
    if no_config || cli.flags.config_assist {
        if let Err(err) = Assistant::run(no_config) {
            panic!("{err:?}");
        }
    }

    // Run the editor
    let result = run(&cli);
    if let Err(err) = result {
        panic!("{err:?}");
    }
}

/// Run the editor
#[allow(clippy::too_many_lines)]
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
    handle_lua_error(
        "",
        lua.load(PLUGIN_NETWORKING).exec(),
        &mut editor.borrow_mut().feedback,
    );

    // Load config and initialise
    lua.load(PLUGIN_BOOTSTRAP).exec()?;
    let result = editor.borrow_mut().load_config(&cli.config_path, &lua);
    if let Some(err) = result {
        // Handle error if available
        handle_lua_error("configuration", Err(err), &mut editor.borrow_mut().feedback);
    };

    // Run plug-ins
    handle_lua_error(
        "",
        lua.load(PLUGIN_RUN).exec(),
        &mut editor.borrow_mut().feedback,
    );

    // Load in the file types
    let file_types = lua
        .globals()
        .get("file_types")
        .unwrap_or(Value::Table(lua.create_table()?));
    let file_types = FileTypes::from_lua(file_types, &lua).unwrap_or_default();
    editor.borrow_mut().config.document.borrow_mut().file_types = file_types;

    // Open files user has asked to open
    let cwd = std::env::current_dir()?;
    for (c, file) in cli.to_open.iter().enumerate() {
        // Reset cwd
        let _ = std::env::set_current_dir(&cwd);
        // Open the file
        let result = editor.borrow_mut().open_or_new(file.to_string());
        handle_file_opening(&editor, result, file);
        // Set read only if applicable
        if cli.flags.read_only {
            editor.borrow_mut().get_doc(c).info.read_only = true;
        }
        // Set highlighter if applicable
        if let Some(ref file_type) = cli.file_type {
            let tab_width = editor.borrow().config.document.borrow().tab_width;
            let file_type = editor
                .borrow_mut()
                .config
                .document
                .borrow()
                .file_types
                .get_name(file_type)
                .unwrap_or_default();
            let mut highlighter = file_type.get_highlighter(&editor.borrow().config, tab_width);
            highlighter.run(&editor.borrow_mut().get_doc(c).lines);
            let mut editor = editor.borrow_mut();
            let file = editor.files.get_mut(c).unwrap();
            file.highlighter = highlighter;
            file.file_type = Some(file_type);
        }
        // Move the pointer to the file we just created
        editor.borrow_mut().next();
    }
    // Reset the pointer back to the first document
    editor.borrow_mut().ptr = 0;

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

    // Add in the plugin manager
    handle_lua_error(
        "",
        lua.load(PLUGIN_MANAGER).exec(),
        &mut editor.borrow_mut().feedback,
    );

    // Run the editor and handle errors if applicable
    editor.borrow().update_cwd();
    editor.borrow_mut().init()?;
    let mut event;
    while editor.borrow().active {
        // Render and wait for event
        editor.borrow_mut().render(&lua)?;
        // Keep requesting events until a valid one is found
        loop {
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
                        handle_lua_error(
                            "task",
                            target.call(()),
                            &mut editor.borrow_mut().feedback,
                        );
                    } else {
                        editor.borrow_mut().feedback =
                            Feedback::Warning(format!("Function '{task}' was not found"));
                    }
                }
            }

            // Read the event
            event = crossterm::event::read()?;

            // Block certain events from passing through
            match event {
                // Key release events cause duplicate and initial key press events which should be ignored
                CEvent::Key(KeyEvent {
                    kind: KeyEventKind::Release,
                    ..
                }) => (),
                _ => break,
            }
        }

        // Clear feedback
        editor.borrow_mut().feedback = Feedback::None;

        // Handle plug-in before key press mappings
        if let CEvent::Key(key) = event {
            let key_str = key_to_string(key.modifiers, key.code);
            let code = run_key_before(&key_str);
            let result = lua.load(&code).exec();
            handle_lua_error(&key_str, result, &mut editor.borrow_mut().feedback);
        }

        // Handle paste event (before event)
        if let CEvent::Paste(ref paste_text) = event {
            let listeners = get_listeners("before:paste", &lua)?;
            for listener in listeners {
                handle_lua_error(
                    "paste",
                    listener.call(paste_text.clone()),
                    &mut editor.borrow_mut().feedback,
                );
            }
        }

        // Actually handle editor event (errors included)
        if let Err(err) = editor.borrow_mut().handle_event(&lua, event.clone()) {
            editor.borrow_mut().feedback = Feedback::Error(format!("{err:?}"));
        }

        // Handle paste event (after event)
        if let CEvent::Paste(ref paste_text) = event {
            let listeners = get_listeners("paste", &lua)?;
            for listener in listeners {
                handle_lua_error(
                    "paste",
                    listener.call(paste_text.clone()),
                    &mut editor.borrow_mut().feedback,
                );
            }
        }

        // Handle plug-in after key press mappings (if no errors occured)
        if let CEvent::Key(key) = event {
            let key_str = key_to_string(key.modifiers, key.code);
            let code = run_key(&key_str);
            let result = lua.load(&code).exec();
            handle_lua_error(&key_str, result, &mut editor.borrow_mut().feedback);
        }

        editor.borrow_mut().update_highlighter();
        if !matches!(event, CEvent::Resize(_, _)) {
            editor.borrow_mut().greet = false;
        }

        // Check for any commands to run
        let command = editor.borrow().command.clone();
        if let Some(command) = command {
            run_editor_command(&editor, &command, &lua);
        }
        editor.borrow_mut().command = None;
    }

    // Run any plugin cleanup operations
    let result = lua.load(run_key("exit")).exec();
    handle_lua_error("exit", result, &mut editor.borrow_mut().feedback);

    editor.borrow_mut().terminal.end()?;
    Ok(())
}

/// Handle a lua error, showing the user an informative error
fn handle_lua_error(key_str: &str, error: RResult<(), mlua::Error>, feedback: &mut Feedback) {
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
                    *feedback = Feedback::Warning(format!("The key {key_str} is not bound"));
                }
            } else if msg.ends_with("command not found") {
                // Command was not found, issue an error
                *feedback = Feedback::Error(format!("The command '{key_str}' is not defined"));
            } else {
                // Some other runtime error
                *feedback = Feedback::Error(msg.to_string());
            }
        }
        // Handle a syntax error
        Err(SyntaxError { message, .. }) => {
            if key_str == "configuration" {
                let mut message = message.rsplit(':').take(2).collect::<Vec<&str>>();
                message.reverse();
                let message = message.join(":");
                *feedback =
                    Feedback::Error(format!("Syntax Error in config file on line {message}"));
            } else {
                *feedback = Feedback::Error(format!("Syntax Error: {message:?}"));
            }
        }
        // Other miscellaneous error
        Err(err) => {
            *feedback = Feedback::Error(format!("Failed to run Lua code: {err:?}"));
        }
    }
}

/// Handle opening files
fn handle_file_opening(editor: &Rc<RefCell<Editor>>, result: Result<()>, name: &str) {
    // TEMPORARY WORK-AROUND: Delete after Rust 1.83
    if file_or_dir(name) == "directory" {
        fatal_error(&format!("'{name}' is a directory, not a file"));
    }
    match result {
        Ok(()) => (),
        Err(OxError::AlreadyOpen { .. }) => {
            let len = editor.borrow().files.len().saturating_sub(1);
            editor.borrow_mut().ptr = len;
        }
        Err(OxError::Kaolinite(kerr)) => {
            match kerr {
                KError::Io(ioerr) => match ioerr.kind() {
                    ErrorKind::NotFound => fatal_error(&format!("File '{name}' not found")),
                    ErrorKind::PermissionDenied => {
                        fatal_error(&format!("Permission to read file '{name}' denied"));
                    }
                    /*
                    // NOTE: Uncomment when Rust 1.83 becomes stable (io_error_more will be stabilised)
                    ErrorKind::IsADirectory =>
                        fatal_error(&format!("'{name}' is a directory, not a file")),
                    ErrorKind::ReadOnlyFilesystem =>
                        fatal_error(&format!("You are on a read only file system")),
                    ErrorKind::ResourceBusy =>
                        fatal_error(&format!("The resource '{name}' is busy")),
                    */
                    ErrorKind::OutOfMemory => fatal_error("You are out of memory"),
                    kind => fatal_error(&format!("I/O error occured: {kind:?}")),
                },
                _ => fatal_error(&format!("Backend error opening '{name}': {kerr:?}")),
            }
        }
        result => fatal_error(&format!("Error opening file '{name}': {result:?}")),
    }
}

/// Run a command in the editor
fn run_editor_command(editor: &Rc<RefCell<Editor>>, cmd: &str, lua: &Lua) {
    let cmd = cmd.replace('\'', "\\'").to_string();
    if let [subcmd, arguments @ ..] = cmd.split(' ').collect::<Vec<&str>>().as_slice() {
        let arguments = arguments.join("', '");
        let code =
            format!("(commands['{subcmd}'] or error('command not found'))({{'{arguments}'}})");
        handle_lua_error(
            subcmd,
            lua.load(code).exec(),
            &mut editor.borrow_mut().feedback,
        );
    }
}
