use crate::{ged, handle_lua_error, CEvent, Editor, Feedback, KeyEvent, KeyEventKind, Result};
use crossterm::event::{poll, read};
use mlua::{AnyUserData, Lua};
use std::time::Duration;

#[allow(unused_variables)]
pub fn term_force(editor: &AnyUserData) -> bool {
    #[cfg(not(target_os = "windows"))]
    return ged!(mut &editor).files.terminal_rerender();
    #[cfg(target_os = "windows")]
    return false;
}

pub fn mm_active(editor: &AnyUserData) -> bool {
    ged!(mut &editor).macro_man.playing
}

/// (should hold event, triggered by term force?)
pub fn hold_event(editor: &AnyUserData) -> (bool, bool) {
    let tf = term_force(editor);
    (
        matches!(
            (mm_active(editor), tf, poll(Duration::from_millis(50))),
            (false, false, Ok(false))
        ),
        !tf,
    )
}

#[allow(unused_variables)]
pub fn wait_for_event(editor: &AnyUserData, lua: &Lua) -> Result<CEvent> {
    loop {
        // While waiting for an event to come along, service the task manager
        if !mm_active(editor) {
            while let (true, was_term) = hold_event(editor) {
                let exec = ged!(mut &editor)
                    .config
                    .task_manager
                    .lock()
                    .unwrap()
                    .execution_list();
                for task in exec {
                    if let Ok(target) = lua.globals().get::<mlua::Function>(task.clone()) {
                        // Run the code
                        handle_lua_error("task", target.call(()), &mut ged!(mut &editor).feedback);
                    } else {
                        ged!(mut &editor).feedback =
                            Feedback::Warning(format!("Function '{task}' was not found"));
                    }
                }
                // If a terminal dictates, force a rerender
                #[cfg(not(target_os = "windows"))]
                if was_term {
                    ged!(mut &editor).needs_rerender = true;
                    ged!(mut &editor).render(lua)?;
                }
            }
        }

        // Attempt to get an event
        let Some(event) = get_event(&mut ged!(mut &editor)) else {
            // No event available, back to the beginning
            continue;
        };

        // Block certain events from passing through
        if !matches!(
            event,
            CEvent::Key(KeyEvent {
                kind: KeyEventKind::Release,
                ..
            })
        ) {
            return Ok(event);
        }
    }
}

/// Wait for event, but without the task manager (and it hogs editor)
pub fn wait_for_event_hog(editor: &mut Editor) -> CEvent {
    loop {
        // Attempt to get an event
        let Some(event) = get_event(editor) else {
            // No event available, back to the beginning
            continue;
        };

        // Block certain events from passing through
        if !matches!(
            event,
            CEvent::Key(KeyEvent {
                kind: KeyEventKind::Release,
                ..
            })
        ) {
            return event;
        }
    }
}

// Find out where to source an event from and source it
pub fn get_event(editor: &mut Editor) -> Option<CEvent> {
    if let Some(ev) = editor.macro_man.next() {
        // Take from macro man
        Some(ev)
    } else if let Ok(true) = poll(Duration::from_millis(50)) {
        if let Ok(ev) = read() {
            // Use standard crossterm event
            Some(ev)
        } else {
            None
        }
    } else {
        None
    }
}
