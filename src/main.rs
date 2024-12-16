#![warn(clippy::all, clippy::pedantic)]

mod cli;
mod config;
mod editor;
mod error;
mod events;
#[cfg(not(target_os = "windows"))]
mod pty;
mod ui;

use cli::CommandLineInterface;
use config::{
    get_listeners, key_to_string, run_key, run_key_before, Assistant, Config, PLUGIN_BOOTSTRAP,
    PLUGIN_MANAGER, PLUGIN_NETWORKING, PLUGIN_RUN,
};
use crossterm::event::{Event as CEvent, KeyEvent, KeyEventKind};
use editor::{allowed_by_multi_cursor, handle_multiple_cursors, Editor, FileTypes};
use error::{OxError, Result};
use events::wait_for_event;
use kaolinite::event::{Error as KError, Event};
use kaolinite::searching::Searcher;
use kaolinite::utils::{file_or_dir, get_cwd};
use kaolinite::{Document, Loc};
use mlua::Error::{RuntimeError, SyntaxError};
use mlua::{AnyUserData, FromLua, Lua, Value};
use std::io::ErrorKind;
use std::result::Result as RResult;
use ui::{fatal_error, Feedback};

/// Get editor helper macro
#[macro_export]
macro_rules! ged {
    ($editor:expr) => {
        $editor.borrow::<Editor>().unwrap()
    };
    (mut $editor:expr) => {
        $editor.borrow_mut::<Editor>().unwrap()
    };
}

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
    let editor = lua.create_userdata(editor)?;
    lua.globals().set("editor", editor.clone())?;

    // Inject the networking library for plug-ins to use
    handle_lua_error(
        "",
        lua.load(PLUGIN_NETWORKING).exec(),
        &mut ged!(mut &editor).feedback,
    );

    // Load config and initialise
    lua.load(PLUGIN_BOOTSTRAP).exec()?;
    let result = ged!(mut &editor).load_config(&cli.config_path, &lua);
    if let Some(err) = result {
        // Handle error if available
        handle_lua_error("configuration", Err(err), &mut ged!(mut &editor).feedback);
    };

    // Run plug-ins
    handle_lua_error(
        "",
        lua.load(PLUGIN_RUN).exec(),
        &mut ged!(mut &editor).feedback,
    );

    // Ensure focus is on the initial atom
    let init_atom = ged!(&editor).files.empty_atoms(vec![]);
    if let Some(init_atom) = init_atom {
        ged!(mut &editor).ptr = init_atom;
    }

    // Load in the file types
    let file_types = lua
        .globals()
        .get("file_types")
        .unwrap_or(Value::Table(lua.create_table()?));
    let file_types = FileTypes::from_lua(file_types, &lua).unwrap_or_default();
    ged!(mut &editor)
        .config
        .document
        .borrow_mut::<config::Document>()
        .unwrap()
        .file_types = file_types;
    // Open files user has asked to open
    let cwd = get_cwd().unwrap_or(".".to_string());
    for (c, file) in cli.to_open.iter().enumerate() {
        // Reset cwd
        let _ = std::env::set_current_dir(&cwd);
        // Open the file
        let result = ged!(mut &editor).open_or_new(file.to_string());
        handle_file_opening(&editor, result, file);
        // Set read only if applicable
        if cli.flags.read_only {
            ged!(mut &editor).get_doc(c).info.read_only = true;
        }
        // Set highlighter if applicable
        if let Some(ref file_type) = cli.file_type {
            let tab_width = config!(ged!(&editor).config, document).tab_width;
            let file_type = config!(ged!(mut &editor).config, document)
                .file_types
                .get_name(file_type)
                .unwrap_or_default();
            let mut highlighter = file_type.get_highlighter(&ged!(&editor).config, tab_width);
            highlighter.run(&ged!(mut &editor).get_doc(c).lines);
            let mut editor = ged!(mut &editor);
            let current_ptr = editor.ptr.clone();
            let file = &mut editor.files.get_atom_mut(current_ptr).unwrap().0[c];
            file.highlighter = highlighter;
            file.file_type = Some(file_type);
        }
        // Move the pointer to the file we just created
        ged!(mut &editor).next();
    }
    // Reset the pointer back to the first document
    let current_ptr = ged!(mut &editor).ptr.clone();
    ged!(mut &editor).files.move_to(current_ptr, 0);

    // Handle stdin if applicable
    if cli.flags.stdin {
        let stdin = cli::get_stdin();
        let mut holder = ged!(mut &editor);
        holder.blank()?;
        let this_doc = holder.doc_len().saturating_sub(1);
        let current_ptr = holder.ptr.clone();
        let doc = &mut holder.files.get_atom_mut(current_ptr).unwrap().0[this_doc].doc;
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
    ged!(mut &editor).new_if_empty()?;

    // Add in the plugin manager
    handle_lua_error(
        "",
        lua.load(PLUGIN_MANAGER).exec(),
        &mut ged!(mut &editor).feedback,
    );

    // Run the editor and handle errors if applicable
    ged!(&editor).update_cwd();
    ged!(mut &editor).init()?;
    while ged!(&editor).active {
        // Render (unless a macro is being played, in which case, don't bother)
        if !ged!(&editor).macro_man.playing || ged!(&editor).macro_man.just_completed {
            ged!(mut &editor).render(&lua)?;
        }

        // Wait for an event
        let event = wait_for_event(&editor, &lua)?;

        // Handle the event
        let original_loc = ged!(&editor)
            .try_doc()
            .map(Document::char_loc)
            .unwrap_or_default();
        handle_event(&editor, &event, &lua)?;

        // Handle multi cursors
        if let CEvent::Key(_) = event {
            let has_multicursors = !ged!(&editor)
                .try_doc()
                .map_or(true, |doc| doc.secondary_cursors.is_empty());
            if ged!(&editor).active && allowed_by_multi_cursor(&event) && has_multicursors {
                handle_multiple_cursors(&editor, &event, &lua, &original_loc)?;
            }
        }

        ged!(mut &editor).update_highlighter();

        // Check for any commands to run
        let command = ged!(&editor).command.clone();
        if let Some(command) = command {
            run_editor_command(&editor, &command, &lua);
        }
        ged!(mut &editor).command = None;
    }

    // Run any plugin cleanup operations
    let result = lua.load(run_key("exit")).exec();
    handle_lua_error("exit", result, &mut ged!(mut &editor).feedback);

    ged!(mut &editor).terminal.end()?;
    Ok(())
}

fn handle_event(editor: &AnyUserData, event: &CEvent, lua: &Lua) -> Result<()> {
    // Clear screen of temporary items (expect on resize event)
    if !matches!(event, CEvent::Resize(_, _)) {
        ged!(mut &editor).greet = false;
        ged!(mut &editor).feedback = Feedback::None;
    }

    // Handle plug-in before key press mappings
    if let CEvent::Key(key) = event {
        let key_str = key_to_string(key.modifiers, key.code);
        let code = run_key_before(&key_str);
        let result = lua.load(&code).exec();
        handle_lua_error(&key_str, result, &mut ged!(mut &editor).feedback);
    }

    // Handle paste event (before event)
    if let CEvent::Paste(ref paste_text) = event {
        let listeners = get_listeners("before:paste", lua)?;
        for listener in listeners {
            handle_lua_error(
                "paste",
                listener.call(paste_text.clone()),
                &mut ged!(mut &editor).feedback,
            );
        }
    }

    // Actually handle editor event (errors included)
    let event_result = ged!(mut &editor).handle_event(lua, event.clone());
    if let Err(err) = event_result {
        // Nicely display error to user
        match err {
            OxError::Lua(err) => {
                handle_lua_error("event", Err(err), &mut ged!(mut &editor).feedback);
            }
            OxError::AlreadyOpen { file } => {
                ged!(mut &editor).feedback =
                    Feedback::Error(format!("File '{file}' is already open"));
            }
            _ => ged!(mut &editor).feedback = Feedback::Error(format!("{err:?}")),
        }
    }

    // Handle paste event (after event)
    if let CEvent::Paste(ref paste_text) = event {
        let listeners = get_listeners("paste", lua)?;
        for listener in listeners {
            handle_lua_error(
                "paste",
                listener.call(paste_text.clone()),
                &mut ged!(mut &editor).feedback,
            );
        }
    }

    // Handle plug-in after key press mappings (if no errors occured)
    if let CEvent::Key(key) = event {
        let key_str = key_to_string(key.modifiers, key.code);
        let code = run_key(&key_str);
        let result = lua.load(&code).exec();
        handle_lua_error(&key_str, result, &mut ged!(mut &editor).feedback);
    }

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
fn handle_file_opening(editor: &AnyUserData, result: Result<()>, name: &str) {
    // Block any directories from being opened (we'll wait until file tree is implemented)
    if file_or_dir(name) == "directory" {
        fatal_error(&format!("'{name}' is not a file"));
    }
    match result {
        Ok(()) => (),
        Err(OxError::AlreadyOpen { .. }) => {
            let len = ged!(&editor).files.len().saturating_sub(1);
            let current_ptr = ged!(&editor).ptr.clone();
            ged!(mut &editor).files.move_to(current_ptr, len);
        }
        Err(OxError::Kaolinite(kerr)) => match kerr {
            KError::Io(ioerr) => match ioerr.kind() {
                ErrorKind::NotFound => fatal_error(&format!("File '{name}' not found")),
                ErrorKind::PermissionDenied => {
                    fatal_error(&format!("Permission to read file '{name}' denied"));
                }
                ErrorKind::IsADirectory => {
                    fatal_error(&format!("'{name}' is a directory, not a file"));
                }
                ErrorKind::ReadOnlyFilesystem => fatal_error("You are on a read only file system"),
                ErrorKind::ResourceBusy => fatal_error(&format!("The resource '{name}' is busy")),
                ErrorKind::OutOfMemory => fatal_error("You are out of memory"),
                kind => fatal_error(&format!("I/O error occured: {kind:?}")),
            },
            _ => fatal_error(&format!("Backend error opening '{name}': {kerr:?}")),
        },
        result => fatal_error(&format!("Error opening file '{name}': {result:?}")),
    }
}

/// Run a command in the editor
fn run_editor_command(editor: &AnyUserData, cmd: &str, lua: &Lua) {
    let cmd = cmd.replace('\'', "\\'").to_string();
    if let [subcmd, arguments @ ..] = cmd.split(' ').collect::<Vec<&str>>().as_slice() {
        let arguments = arguments.join("', '");
        let code =
            format!("(commands['{subcmd}'] or error('command not found'))({{'{arguments}'}})");
        handle_lua_error(
            subcmd,
            lua.load(code).exec(),
            &mut ged!(mut &editor).feedback,
        );
    }
}
