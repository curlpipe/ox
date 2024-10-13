use crate::cli::VERSION;
use crate::editor::Editor;
use crate::ui::Feedback;
use crate::{PLUGIN_BOOTSTRAP, PLUGIN_MANAGER, PLUGIN_NETWORKING, PLUGIN_RUN};
use kaolinite::utils::{get_absolute_path, get_file_ext, get_file_name};
use kaolinite::{Loc, Size};
use mlua::prelude::*;

impl LuaUserData for Editor {
    fn add_fields<'lua, F: LuaUserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field_method_get("cursor", |_, editor| {
            let loc = editor.doc().char_loc();
            Ok(LuaLoc {
                x: loc.x,
                y: loc.y + 1,
            })
        });
        fields.add_field_method_get("document_name", |_, editor| {
            let name = editor.doc().file_name.clone();
            Ok(name)
        });
        fields.add_field_method_get("document_length", |_, editor| {
            let len = editor.doc().len_lines();
            Ok(len)
        });
        fields.add_field_method_get("version", |_, _| Ok(VERSION));
        fields.add_field_method_get("current_document_id", |_, editor| Ok(editor.ptr));
        fields.add_field_method_get("document_count", |_, editor| Ok(editor.files.len()));
        fields.add_field_method_get("document_type", |_, editor| {
            Ok(editor.files[editor.ptr]
                .file_type
                .clone()
                .map_or("Unknown".to_string(), |t| t.name))
        });
        fields.add_field_method_get("file_name", |_, editor| {
            let name = get_file_name(&editor.doc().file_name.clone().unwrap_or_default());
            Ok(name)
        });
        fields.add_field_method_get("file_extension", |_, editor| {
            let name = get_file_ext(&editor.doc().file_name.clone().unwrap_or_default());
            Ok(name)
        });
        fields.add_field_method_get("file_path", |_, editor| {
            let name = get_absolute_path(&editor.doc().file_name.clone().unwrap_or_default());
            Ok(name)
        });
    }

    #[allow(clippy::too_many_lines)]
    fn add_methods<'lua, M: LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        // Reload the configuration file
        methods.add_method_mut("reload_config", |lua, editor, ()| {
            let path = editor.config_path.clone();
            if editor.load_config(&path, lua).is_some() {
                editor.feedback = Feedback::Error("Failed to reload config".to_string());
            }
            Ok(())
        });
        methods.add_method_mut("reload_plugins", |lua, editor, ()| {
            // Provide plug-in bootstrap
            let _ = lua.load(PLUGIN_BOOTSTRAP).exec();
            // Provide networking to plug-ins and configuration file
            let _ = lua.load(PLUGIN_NETWORKING).exec();
            // Reload the configuration file
            let path = editor.config_path.clone();
            if editor.load_config(&path, lua).is_some() {
                editor.feedback = Feedback::Error("Failed to reload config".to_string());
            }
            // Run plug-ins
            let _ = lua.load(PLUGIN_RUN).exec();
            // Attach plugin manager
            let _ = lua.load(PLUGIN_MANAGER).exec();
            Ok(())
        });
        // Display messages
        methods.add_method_mut("display_error", |_, editor, message: String| {
            editor.feedback = Feedback::Error(message);
            Ok(())
        });
        methods.add_method_mut("display_warning", |_, editor, message: String| {
            editor.feedback = Feedback::Warning(message);
            Ok(())
        });
        methods.add_method_mut("display_info", |_, editor, message: String| {
            editor.feedback = Feedback::Info(message);
            Ok(())
        });
        // Prompt the user
        methods.add_method_mut("prompt", |_, editor, question: String| {
            Ok(editor
                .prompt(question)
                .unwrap_or_else(|_| "error".to_string()))
        });
        // Edit commands (relative)
        methods.add_method_mut("insert", |_, editor, text: String| {
            editor.plugin_active = true;
            for ch in text.chars() {
                if let Err(err) = editor.character(ch) {
                    editor.feedback = Feedback::Error(err.to_string());
                }
            }
            editor.update_highlighter();
            editor.plugin_active = false;
            Ok(())
        });
        methods.add_method_mut("remove", |_, editor, ()| {
            editor.plugin_active = true;
            if let Err(err) = editor.backspace() {
                editor.feedback = Feedback::Error(err.to_string());
            }
            editor.update_highlighter();
            editor.plugin_active = false;
            Ok(())
        });
        methods.add_method_mut("insert_line", |_, editor, ()| {
            editor.plugin_active = true;
            if let Err(err) = editor.enter() {
                editor.feedback = Feedback::Error(err.to_string());
            }
            editor.update_highlighter();
            editor.plugin_active = false;
            Ok(())
        });
        methods.add_method_mut("remove_line", |_, editor, ()| {
            editor.plugin_active = true;
            if let Err(err) = editor.delete_line() {
                editor.feedback = Feedback::Error(err.to_string());
            }
            editor.update_highlighter();
            editor.plugin_active = false;
            Ok(())
        });
        // Cursor moving
        methods.add_method_mut("move_to", |_, editor, (x, y): (usize, usize)| {
            let y = y.saturating_sub(1);
            editor.doc_mut().move_to(&Loc { y, x });
            editor.update_highlighter();
            Ok(())
        });
        methods.add_method_mut("move_up", |_, editor, ()| {
            editor.up();
            editor.update_highlighter();
            Ok(())
        });
        methods.add_method_mut("move_down", |_, editor, ()| {
            editor.down();
            editor.update_highlighter();
            Ok(())
        });
        methods.add_method_mut("move_left", |_, editor, ()| {
            editor.left();
            editor.update_highlighter();
            Ok(())
        });
        methods.add_method_mut("move_right", |_, editor, ()| {
            editor.right();
            editor.update_highlighter();
            Ok(())
        });
        methods.add_method_mut("move_home", |_, editor, ()| {
            editor.doc_mut().move_home();
            editor.update_highlighter();
            Ok(())
        });
        methods.add_method_mut("move_end", |_, editor, ()| {
            editor.doc_mut().move_end();
            editor.update_highlighter();
            Ok(())
        });
        methods.add_method_mut("move_page_up", |_, editor, ()| {
            editor.doc_mut().move_page_up();
            editor.update_highlighter();
            Ok(())
        });
        methods.add_method_mut("move_page_down", |_, editor, ()| {
            editor.doc_mut().move_page_down();
            editor.update_highlighter();
            Ok(())
        });
        methods.add_method_mut("move_top", |_, editor, ()| {
            editor.doc_mut().move_top();
            editor.update_highlighter();
            Ok(())
        });
        methods.add_method_mut("move_bottom", |_, editor, ()| {
            editor.doc_mut().move_bottom();
            editor.update_highlighter();
            Ok(())
        });
        methods.add_method_mut("move_previous_word", |_, editor, ()| {
            editor.prev_word();
            editor.update_highlighter();
            Ok(())
        });
        methods.add_method_mut("move_next_word", |_, editor, ()| {
            editor.next_word();
            editor.update_highlighter();
            Ok(())
        });
        // Cursor selection and clipboard
        methods.add_method_mut("select_up", |_, editor, ()| {
            editor.select_up();
            editor.update_highlighter();
            Ok(())
        });
        methods.add_method_mut("select_down", |_, editor, ()| {
            editor.select_down();
            editor.update_highlighter();
            Ok(())
        });
        methods.add_method_mut("select_left", |_, editor, ()| {
            editor.select_left();
            editor.update_highlighter();
            Ok(())
        });
        methods.add_method_mut("select_right", |_, editor, ()| {
            editor.select_right();
            editor.update_highlighter();
            Ok(())
        });
        methods.add_method_mut("select_all", |_, editor, ()| {
            editor.select_all();
            editor.update_highlighter();
            Ok(())
        });
        methods.add_method_mut("cut", |_, editor, ()| {
            editor.plugin_active = true;
            if let Err(err) = editor.cut() {
                editor.feedback = Feedback::Error(err.to_string());
            } else {
                editor.feedback = Feedback::Info("Text cut to clipboard".to_owned());
            }
            editor.plugin_active = false;
            Ok(())
        });
        methods.add_method_mut("copy", |_, editor, ()| {
            if let Err(err) = editor.copy() {
                editor.feedback = Feedback::Error(err.to_string());
            } else {
                editor.feedback = Feedback::Info("Text copied to clipboard".to_owned());
            }
            Ok(())
        });
        // Document editing
        methods.add_method_mut(
            "insert_at",
            |_, editor, (text, x, y): (String, usize, usize)| {
                editor.plugin_active = true;
                let y = y.saturating_sub(1);
                let location = editor.doc_mut().char_loc();
                editor.doc_mut().move_to(&Loc { y, x });
                for ch in text.chars() {
                    if let Err(err) = editor.character(ch) {
                        editor.feedback = Feedback::Error(err.to_string());
                    }
                }
                editor.doc_mut().move_to(&location);
                editor.update_highlighter();
                editor.plugin_active = false;
                Ok(())
            },
        );
        methods.add_method_mut("remove_at", |_, editor, (x, y): (usize, usize)| {
            editor.plugin_active = true;
            let y = y.saturating_sub(1);
            let location = editor.doc_mut().char_loc();
            editor.doc_mut().move_to(&Loc { y, x });
            if let Err(err) = editor.delete() {
                editor.feedback = Feedback::Error(err.to_string());
            }
            editor.doc_mut().move_to(&location);
            editor.update_highlighter();
            editor.plugin_active = false;
            Ok(())
        });
        methods.add_method_mut("insert_line_at", |_, editor, (text, y): (String, usize)| {
            editor.plugin_active = true;
            let y = y.saturating_sub(1);
            let location = editor.doc_mut().char_loc();
            if y < editor.doc().len_lines() {
                editor.doc_mut().move_to_y(y);
                editor.doc_mut().move_home();
                if let Err(err) = editor.enter() {
                    editor.feedback = Feedback::Error(err.to_string());
                }
                editor.up();
            } else {
                editor.doc_mut().move_bottom();
                if let Err(err) = editor.enter() {
                    editor.feedback = Feedback::Error(err.to_string());
                }
            }
            for ch in text.chars() {
                if let Err(err) = editor.character(ch) {
                    editor.feedback = Feedback::Error(err.to_string());
                }
            }
            editor.doc_mut().move_to(&location);
            editor.update_highlighter();
            editor.plugin_active = false;
            Ok(())
        });
        methods.add_method_mut("remove_line_at", |_, editor, y: usize| {
            editor.plugin_active = true;
            let y = y.saturating_sub(1);
            let location = editor.doc_mut().char_loc();
            editor.doc_mut().move_to_y(y);
            if let Err(err) = editor.delete_line() {
                editor.feedback = Feedback::Error(err.to_string());
            }
            editor.doc_mut().move_to(&location);
            editor.update_highlighter();
            editor.plugin_active = false;
            Ok(())
        });
        methods.add_method("get", |_, editor, ()| {
            let contents = editor.doc().lines.join("\n");
            Ok(contents)
        });
        methods.add_method("get_character", |_, editor, ()| {
            let loc = editor.doc().char_loc();
            let ch = editor
                .doc()
                .line(loc.y)
                .unwrap_or_default()
                .chars()
                .nth(loc.x)
                .map(|ch| ch.to_string())
                .unwrap_or_default();
            Ok(ch)
        });
        methods.add_method_mut("get_character_at", |_, editor, (x, y): (usize, usize)| {
            editor.doc_mut().load_to(y);
            let y = y.saturating_sub(1);
            let ch = editor
                .doc()
                .line(y)
                .unwrap_or_default()
                .chars()
                .nth(x)
                .map_or_else(String::new, |ch| ch.to_string());
            editor.update_highlighter();
            Ok(ch)
        });
        methods.add_method("get_line", |_, editor, ()| {
            let loc = editor.doc().char_loc();
            let line = editor.doc().line(loc.y).unwrap_or_default();
            Ok(line)
        });
        methods.add_method_mut("get_line_at", |_, editor, y: usize| {
            editor.doc_mut().load_to(y);
            let y = y.saturating_sub(1);
            let line = editor.doc().line(y).unwrap_or_default();
            editor.update_highlighter();
            Ok(line)
        });
        // Document management
        methods.add_method_mut("previous_tab", |_, editor, ()| {
            editor.prev();
            Ok(())
        });
        methods.add_method_mut("next_tab", |_, editor, ()| {
            editor.next();
            Ok(())
        });
        methods.add_method_mut("move_to_document", |_, editor, id: usize| {
            editor.ptr = id;
            Ok(())
        });
        methods.add_method_mut("new", |_, editor, ()| {
            if let Err(err) = editor.new_document() {
                editor.feedback = Feedback::Error(err.to_string());
            }
            Ok(())
        });
        methods.add_method_mut("open", |_, editor, ()| {
            if let Err(err) = editor.open_document() {
                editor.feedback = Feedback::Error(err.to_string());
            }
            Ok(())
        });
        methods.add_method_mut("save", |_, editor, ()| {
            if let Err(err) = editor.save() {
                editor.feedback = Feedback::Error(err.to_string());
            }
            Ok(())
        });
        methods.add_method_mut("save_as", |_, editor, ()| {
            if let Err(err) = editor.save_as() {
                editor.feedback = Feedback::Error(err.to_string());
            }
            Ok(())
        });
        methods.add_method_mut("save_all", |_, editor, ()| {
            if let Err(err) = editor.save_all() {
                editor.feedback = Feedback::Error(err.to_string());
            }
            Ok(())
        });
        methods.add_method_mut("quit", |_, editor, ()| {
            if let Err(err) = editor.quit() {
                editor.feedback = Feedback::Error(err.to_string());
            }
            Ok(())
        });
        methods.add_method_mut("undo", |_, editor, ()| {
            if let Err(err) = editor.undo() {
                editor.feedback = Feedback::Error(err.to_string());
            }
            editor.update_highlighter();
            Ok(())
        });
        methods.add_method_mut("redo", |_, editor, ()| {
            if let Err(err) = editor.redo() {
                editor.feedback = Feedback::Error(err.to_string());
            }
            editor.update_highlighter();
            Ok(())
        });
        methods.add_method_mut("commit", |_, editor, ()| {
            editor.doc_mut().commit();
            Ok(())
        });
        // Searching and replacing
        methods.add_method_mut("search", |_, editor, ()| {
            if let Err(err) = editor.search() {
                editor.feedback = Feedback::Error(err.to_string());
            }
            editor.update_highlighter();
            Ok(())
        });
        methods.add_method_mut("replace", |_, editor, ()| {
            if let Err(err) = editor.replace() {
                editor.feedback = Feedback::Error(err.to_string());
            }
            editor.update_highlighter();
            Ok(())
        });
        methods.add_method_mut("move_next_match", |_, editor, query: String| {
            editor.next_match(&query);
            editor.update_highlighter();
            Ok(())
        });
        methods.add_method_mut("move_previous_match", |_, editor, query: String| {
            editor.prev_match(&query);
            editor.update_highlighter();
            Ok(())
        });
        // Document state modification
        methods.add_method_mut("set_read_only", |_, editor, status: bool| {
            editor.doc_mut().info.read_only = status;
            Ok(())
        });
        methods.add_method_mut("set_file_type", |_, editor, name: String| {
            if let Some(file_type) = editor.config.document.borrow().file_types.get_name(&name) {
                let mut highlighter = file_type.get_highlighter(&editor.config, 4);
                highlighter.run(&editor.doc().lines);
                editor.files[editor.ptr].highlighter = highlighter;
                editor.files[editor.ptr].file_type = Some(file_type);
            } else {
                editor.feedback = Feedback::Error(format!("Invalid file type: {name}"));
            }
            Ok(())
        });
        // Rerendering
        methods.add_method_mut("rerender", |lua, editor, ()| {
            // Force a re-render
            editor.needs_rerender = true;
            // If you can't render the editor, you're pretty much done for anyway
            let _ = editor.render(lua);
            Ok(())
        });
        methods.add_method_mut("rerender_feedback_line", |_, editor, ()| {
            // If you can't render the editor, you're pretty much done for anyway
            let Size { w, mut h } = crate::ui::size().unwrap_or(Size { w: 0, h: 0 });
            h = h.saturating_sub(1 + editor.push_down);
            let _ = editor.terminal.hide_cursor();
            let _ = editor.render_feedback_line(w, h);
            // Apply render and restore cursor
            let max = editor.dent();
            if let Some(Loc { x, y }) = editor.doc().cursor_loc_in_screen() {
                let _ = editor.terminal.goto(x + max, y + editor.push_down);
            }
            let _ = editor.terminal.show_cursor();
            let _ = editor.terminal.flush();
            Ok(())
        });
        methods.add_method_mut("rerender_status_line", |lua, editor, ()| {
            // If you can't render the editor, you're pretty much done for anyway
            let Size { w, mut h } = crate::ui::size().unwrap_or(Size { w: 0, h: 0 });
            h = h.saturating_sub(1 + editor.push_down);
            let _ = editor.terminal.hide_cursor();
            let _ = editor.render_status_line(lua, w, h);
            // Apply render and restore cursor
            let max = editor.dent();
            if let Some(Loc { x, y }) = editor.doc().cursor_loc_in_screen() {
                let _ = editor.terminal.goto(x + max, y + editor.push_down);
            }
            let _ = editor.terminal.show_cursor();
            let _ = editor.terminal.flush();
            Ok(())
        });
        // Miscellaneous
        methods.add_method_mut("open_command_line", |_, editor, ()| {
            match editor.prompt("Command") {
                Ok(command) => {
                    editor.command = Some(command);
                }
                Err(err) => {
                    editor.feedback = Feedback::Error(err.to_string());
                }
            }
            Ok(())
        });
    }
}

/// For representing a cursor location object within lua
pub struct LuaLoc {
    x: usize,
    y: usize,
}

impl IntoLua<'_> for LuaLoc {
    /// Convert this rust struct so the plug-in and configuration system can use it
    fn into_lua(self, lua: &Lua) -> std::result::Result<LuaValue<'_>, LuaError> {
        let table = lua.create_table()?;
        table.set("x", self.x)?;
        table.set("y", self.y)?;
        Ok(LuaValue::Table(table))
    }
}
