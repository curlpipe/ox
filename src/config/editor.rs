/// Defines the Editor API for plug-ins to use
use crate::cli::VERSION;
#[cfg(not(target_os = "windows"))]
use crate::config::runner::RunCommand;
use crate::editor::{Editor, FileContainer, FileLayout};
#[cfg(not(target_os = "windows"))]
use crate::pty::Pty;
use crate::ui::Feedback;
use crate::{config, fatal_error, PLUGIN_BOOTSTRAP, PLUGIN_MANAGER, PLUGIN_NETWORKING, PLUGIN_RUN};
use kaolinite::utils::{get_absolute_path, get_cwd, get_file_ext, get_file_name};
use kaolinite::Loc;
use mlua::prelude::*;
#[cfg(not(target_os = "windows"))]
use std::collections::HashMap;

impl LuaUserData for Editor {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("cursor", |_, editor| {
            if let Some(doc) = editor.try_doc() {
                let loc = doc.char_loc();
                Ok(Some(LuaLoc {
                    x: loc.x,
                    y: loc.y + 1,
                }))
            } else {
                Ok(None)
            }
        });
        fields.add_field_method_get("selection", |_, editor| {
            if let Some(doc) = editor.try_doc() {
                let loc = doc.cursor.selection_end;
                Ok(Some(LuaLoc {
                    x: doc.character_idx(&loc),
                    y: loc.y + 1,
                }))
            } else {
                Ok(None)
            }
        });
        fields.add_field_method_get("document_name", |_, editor| {
            if let Some(doc) = editor.try_doc() {
                Ok(Some(doc.file_name.clone()))
            } else {
                Ok(None)
            }
        });
        fields.add_field_method_get("document_length", |_, editor| {
            if let Some(doc) = editor.try_doc() {
                Ok(Some(doc.len_lines()))
            } else {
                Ok(None)
            }
        });
        fields.add_field_method_get("version", |_, _| Ok(VERSION));
        fields.add_field_method_get("current_document_id", |_, editor| {
            Ok(editor.files.get_atom(editor.ptr.clone()).map(|a| a.1))
        });
        fields.add_field_method_get("document_count", |_, editor| {
            Ok(editor.files.get_all(editor.ptr.clone()).len())
        });
        fields.add_field_method_get("document_type", |_, editor| {
            Ok(editor
                .files
                .get(editor.ptr.clone())
                .unwrap_or(&FileContainer::default())
                .file_type
                .clone()
                .map_or("Unknown".to_string(), |ft| ft.name))
        });
        fields.add_field_method_get("file_name", |_, editor| {
            if let Some(doc) = editor.try_doc() {
                Ok(Some(get_file_name(
                    &doc.file_name.clone().unwrap_or_default(),
                )))
            } else {
                Ok(None)
            }
        });
        fields.add_field_method_get("file_extension", |_, editor| {
            if let Some(doc) = editor.try_doc() {
                Ok(Some(get_file_ext(
                    &doc.file_name.clone().unwrap_or_default(),
                )))
            } else {
                Ok(None)
            }
        });
        fields.add_field_method_get("file_path", |_, editor| {
            if let Some(doc) = editor.try_doc() {
                Ok(Some(get_absolute_path(
                    &doc.file_name.clone().unwrap_or_default(),
                )))
            } else {
                Ok(None)
            }
        });
        fields.add_field_method_get("cwd", |_, _| Ok(get_cwd()));
        fields.add_field_method_get("macro_recording", |_, editor| {
            Ok(editor.macro_man.recording)
        });
        fields.add_field_method_get("macro_playing", |_, editor| Ok(editor.macro_man.playing));
    }

    #[allow(clippy::too_many_lines)]
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        // Debugging methods
        methods.add_method_mut("panic", |_, _, msg: String| {
            fatal_error(&msg);
            Ok(())
        });
        // Reload the configuration file
        methods.add_method_mut("reset_terminal", |_, editor, ()| {
            let _ = editor.terminal.start();
            Ok(())
        });
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
        methods.add_method_mut("remove_word", |_, editor, ()| {
            if let Some(doc) = editor.try_doc_mut() {
                let _ = doc.delete_word();
                let y = doc.loc().y;
                editor.update_highlighter();
                editor.hl_edit(y);
            }
            Ok(())
        });
        // Cursor moving
        methods.add_method_mut("move_to", |_, editor, (x, y): (usize, usize)| {
            if let Some(doc) = editor.try_doc_mut() {
                let y = y.saturating_sub(1);
                doc.move_to(&Loc { y, x });
                editor.update_highlighter();
            }
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
            if let Some(doc) = editor.try_doc_mut() {
                doc.move_home();
                editor.update_highlighter();
            }
            Ok(())
        });
        methods.add_method_mut("move_end", |_, editor, ()| {
            if let Some(doc) = editor.try_doc_mut() {
                doc.move_end();
                editor.update_highlighter();
            }
            Ok(())
        });
        methods.add_method_mut("move_page_up", |_, editor, ()| {
            if let Some(doc) = editor.try_doc_mut() {
                doc.move_page_up();
                editor.update_highlighter();
            }
            Ok(())
        });
        methods.add_method_mut("move_page_down", |_, editor, ()| {
            if let Some(doc) = editor.try_doc_mut() {
                doc.move_page_down();
                editor.update_highlighter();
            }
            Ok(())
        });
        methods.add_method_mut("move_top", |_, editor, ()| {
            if let Some(doc) = editor.try_doc_mut() {
                doc.move_top();
                editor.update_highlighter();
            }
            Ok(())
        });
        methods.add_method_mut("move_bottom", |_, editor, ()| {
            if let Some(doc) = editor.try_doc_mut() {
                doc.move_bottom();
                editor.update_highlighter();
            }
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
        methods.add_method_mut("cursor_snap", |_, editor, ()| {
            if let Some(doc) = editor.try_doc_mut() {
                doc.old_cursor = doc.loc().x;
            }
            Ok(())
        });
        methods.add_method_mut("move_line_up", |_, editor, ()| {
            if let Some(doc) = editor.try_doc_mut() {
                let _ = doc.swap_line_up();
                let y = doc.loc().y;
                editor.hl_edit(y);
                editor.hl_edit(y + 1);
                editor.update_highlighter();
            }
            Ok(())
        });
        methods.add_method_mut("move_line_down", |_, editor, ()| {
            if let Some(doc) = editor.try_doc_mut() {
                let _ = doc.swap_line_down();
                let y = doc.loc().y;
                editor.hl_edit(y.saturating_sub(1));
                editor.hl_edit(y);
                editor.update_highlighter();
            }
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
        methods.add_method_mut("select_to", |_, editor, (x, y): (usize, usize)| {
            if let Some(doc) = editor.try_doc_mut() {
                let y = y.saturating_sub(1);
                doc.select_to(&Loc { y, x });
                editor.update_highlighter();
            }
            Ok(())
        });
        methods.add_method_mut("cancel_selection", |_, editor, ()| {
            if let Some(doc) = editor.try_doc_mut() {
                doc.cancel_selection();
            }
            Ok(())
        });
        methods.add_method_mut("cursor_to_viewport", |_, editor, ()| {
            if let Some(doc) = editor.try_doc_mut() {
                doc.bring_cursor_in_viewport();
            }
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
                if editor.try_doc().is_some() {
                    editor.plugin_active = true;
                    let y = y.saturating_sub(1);
                    let location = editor.try_doc().unwrap().char_loc();
                    if let Some(doc) = editor.try_doc_mut() {
                        doc.move_to(&Loc { y, x });
                    }
                    for ch in text.chars() {
                        if let Err(err) = editor.character(ch) {
                            editor.feedback = Feedback::Error(err.to_string());
                        }
                    }
                    if let Some(doc) = editor.try_doc_mut() {
                        doc.move_to(&location);
                    }
                    editor.update_highlighter();
                    editor.plugin_active = false;
                }
                Ok(())
            },
        );
        methods.add_method_mut("remove_at", |_, editor, (x, y): (usize, usize)| {
            if editor.try_doc().is_some() {
                editor.plugin_active = true;
                let y = y.saturating_sub(1);
                let location = editor.try_doc().unwrap().char_loc();
                if let Some(doc) = editor.try_doc_mut() {
                    doc.move_to(&Loc { y, x });
                }
                if let Err(err) = editor.delete() {
                    editor.feedback = Feedback::Error(err.to_string());
                }
                if let Some(doc) = editor.try_doc_mut() {
                    doc.move_to(&location);
                }
                editor.update_highlighter();
                editor.plugin_active = false;
            }
            Ok(())
        });
        methods.add_method_mut("insert_line_at", |_, editor, (text, y): (String, usize)| {
            if editor.try_doc().is_some() {
                editor.plugin_active = true;
                let y = y.saturating_sub(1);
                let location = editor.try_doc().unwrap().char_loc();
                if let Some(doc) = editor.try_doc_mut() {
                    if y < doc.len_lines() {
                        doc.move_to_y(y);
                        doc.move_home();
                        if let Err(err) = editor.enter() {
                            editor.feedback = Feedback::Error(err.to_string());
                        }
                        editor.up();
                    } else {
                        doc.move_bottom();
                        if let Err(err) = editor.enter() {
                            editor.feedback = Feedback::Error(err.to_string());
                        }
                    }
                    for ch in text.chars() {
                        if let Err(err) = editor.character(ch) {
                            editor.feedback = Feedback::Error(err.to_string());
                        }
                    }
                }
                editor.try_doc_mut().unwrap().move_to(&location);
                editor.update_highlighter();
                editor.plugin_active = false;
            }
            Ok(())
        });
        methods.add_method_mut("remove_line_at", |_, editor, y: usize| {
            if editor.try_doc().is_some() {
                editor.plugin_active = true;
                let y = y.saturating_sub(1);
                let location = editor.try_doc().unwrap().char_loc();
                editor.try_doc_mut().unwrap().move_to_y(y);
                if let Err(err) = editor.delete_line() {
                    editor.feedback = Feedback::Error(err.to_string());
                }
                editor.try_doc_mut().unwrap().move_to(&location);
                editor.update_highlighter();
                editor.plugin_active = false;
            }
            Ok(())
        });
        methods.add_method_mut("get", |_, editor, ()| {
            if let Some(doc) = editor.try_doc_mut() {
                let lines = doc.len_lines();
                doc.load_to(lines);
                let contents = doc.lines.join("\n");
                Ok(Some(contents))
            } else {
                Ok(None)
            }
        });
        methods.add_method("get_character", |_, editor, ()| {
            if let Some(doc) = editor.try_doc() {
                let loc = doc.char_loc();
                let ch = doc
                    .line(loc.y)
                    .unwrap_or_default()
                    .chars()
                    .nth(loc.x)
                    .map(|ch| ch.to_string())
                    .unwrap_or_default();
                Ok(Some(ch))
            } else {
                Ok(None)
            }
        });
        methods.add_method_mut("get_character_at", |_, editor, (x, y): (usize, usize)| {
            if let Some(doc) = editor.try_doc_mut() {
                doc.load_to(y);
                let y = y.saturating_sub(1);
                let ch = doc
                    .line(y)
                    .unwrap_or_default()
                    .chars()
                    .nth(x)
                    .map_or_else(String::new, |ch| ch.to_string());
                editor.update_highlighter();
                Ok(Some(ch))
            } else {
                Ok(None)
            }
        });
        methods.add_method("get_line", |_, editor, ()| {
            if let Some(doc) = editor.try_doc() {
                let loc = doc.char_loc();
                let line = doc.line(loc.y).unwrap_or_default();
                Ok(Some(line))
            } else {
                Ok(None)
            }
        });
        methods.add_method_mut("get_line_at", |_, editor, y: usize| {
            if let Some(doc) = editor.try_doc_mut() {
                doc.load_to(y);
                let y = y.saturating_sub(1);
                let line = doc.line(y).unwrap_or_default();
                editor.update_highlighter();
                Ok(Some(line))
            } else {
                Ok(None)
            }
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
            editor.files.move_to(editor.ptr.clone(), id);
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
            if let Some(doc) = editor.try_doc_mut() {
                doc.commit();
            }
            Ok(())
        });
        // Split management
        methods.add_method_mut("open_split_up", |_, editor, file: String| {
            if let Ok(fc) = editor.open_fc(&file) {
                editor.ptr = editor
                    .files
                    .open_up(editor.ptr.clone(), FileLayout::Atom(vec![fc], 0));
                editor.cache_old_ptr(&editor.ptr.clone());
                editor.update_cwd();
                Ok(true)
            } else {
                Ok(false)
            }
        });
        methods.add_method_mut("open_split_down", |_, editor, file: String| {
            if let Ok(fc) = editor.open_fc(&file) {
                editor.ptr = editor
                    .files
                    .open_down(editor.ptr.clone(), FileLayout::Atom(vec![fc], 0));
                editor.cache_old_ptr(&editor.ptr.clone());
                editor.update_cwd();
                Ok(true)
            } else {
                Ok(false)
            }
        });
        methods.add_method_mut("open_split_left", |_, editor, file: String| {
            if let Ok(fc) = editor.open_fc(&file) {
                editor.ptr = editor
                    .files
                    .open_left(editor.ptr.clone(), FileLayout::Atom(vec![fc], 0));
                editor.cache_old_ptr(&editor.ptr.clone());
                editor.update_cwd();
                Ok(true)
            } else {
                Ok(false)
            }
        });
        methods.add_method_mut("open_split_right", |_, editor, file: String| {
            if let Ok(fc) = editor.open_fc(&file) {
                editor.ptr = editor
                    .files
                    .open_right(editor.ptr.clone(), FileLayout::Atom(vec![fc], 0));
                editor.cache_old_ptr(&editor.ptr.clone());
                editor.update_cwd();
                Ok(true)
            } else {
                Ok(false)
            }
        });
        methods.add_method_mut(
            "grow_split",
            |_, editor, (amount, direction): (f64, String)| {
                match direction.as_str() {
                    "width" => editor.files.grow_width(&editor.ptr, amount),
                    "height" => editor.files.grow_height(&editor.ptr, amount),
                    _ => (),
                }
                Ok(())
            },
        );
        methods.add_method_mut(
            "shrink_split",
            |_, editor, (amount, direction): (f64, String)| {
                match direction.as_str() {
                    "width" => editor.files.shrink_width(&editor.ptr, amount),
                    "height" => editor.files.shrink_height(&editor.ptr, amount),
                    _ => (),
                }
                Ok(())
            },
        );
        methods.add_method_mut("focus_split_up", |_, editor, ()| {
            editor.ptr = FileLayout::move_up(editor.ptr.clone(), &editor.render_cache.span);
            editor.cache_old_ptr(&editor.ptr.clone());
            editor.update_cwd();
            Ok(())
        });
        methods.add_method_mut("focus_split_down", |_, editor, ()| {
            editor.ptr = FileLayout::move_down(editor.ptr.clone(), &editor.render_cache.span);
            editor.cache_old_ptr(&editor.ptr.clone());
            editor.update_cwd();
            Ok(())
        });
        methods.add_method_mut("focus_split_left", |_, editor, ()| {
            let new_ptr = FileLayout::move_left(editor.ptr.clone(), &editor.render_cache.span);
            let in_file_tree = matches!(
                editor.files.get_raw(new_ptr.clone()),
                Some(FileLayout::FileTree)
            );
            if in_file_tree {
                // We are about to enter a file tree, cache where we were (minus the file tree itself)
                editor.old_ptr = editor.ptr.clone();
            } else {
                editor.old_ptr = new_ptr.clone();
            }
            if editor.file_tree_is_open() && !editor.old_ptr.is_empty() {
                editor.old_ptr.remove(0);
            }
            editor.ptr = new_ptr;
            editor.update_cwd();
            Ok(())
        });
        methods.add_method_mut("focus_split_right", |_, editor, ()| {
            editor.ptr = FileLayout::move_right(editor.ptr.clone(), &editor.render_cache.span);
            editor.cache_old_ptr(&editor.ptr.clone());
            editor.update_cwd();
            Ok(())
        });
        // Searching and replacing
        methods.add_method_mut("search", |lua, editor, ()| {
            if let Err(err) = editor.search(lua) {
                editor.feedback = Feedback::Error(err.to_string());
            }
            editor.update_highlighter();
            editor.needs_rerender = true;
            let _ = editor.render(lua);
            Ok(())
        });
        methods.add_method_mut("replace", |lua, editor, ()| {
            if let Err(err) = editor.replace(lua) {
                editor.feedback = Feedback::Error(err.to_string());
            }
            editor.update_highlighter();
            editor.needs_rerender = true;
            let _ = editor.render(lua);
            Ok(())
        });
        methods.add_method_mut("move_next_match", |_, editor, query: String| {
            editor.next_match(&query);
            if let Some(doc) = editor.try_doc_mut() {
                doc.cancel_selection();
            }
            editor.update_highlighter();
            Ok(())
        });
        methods.add_method_mut("move_previous_match", |_, editor, query: String| {
            editor.prev_match(&query);
            if let Some(doc) = editor.try_doc_mut() {
                doc.cancel_selection();
            }
            editor.update_highlighter();
            Ok(())
        });
        // Document state modification
        methods.add_method_mut("set_read_only", |_, editor, status: bool| {
            if let Some(doc) = editor.try_doc_mut() {
                doc.info.read_only = status;
            }
            Ok(())
        });
        methods.add_method_mut("set_file_type", |_, editor, name: String| {
            if let Some(actual_doc) = editor.try_doc() {
                let doc = config!(editor.config, document);
                if let Some(file_type) = doc.file_types.get_name(&name) {
                    let mut highlighter = file_type.get_highlighter(&editor.config, 4);
                    highlighter.run(&actual_doc.lines);
                    if let Some(file) = editor.files.get_mut(editor.ptr.clone()) {
                        file.highlighter = highlighter;
                        file.file_type = Some(file_type);
                    }
                } else {
                    editor.feedback = Feedback::Error(format!("Invalid file type: {name}"));
                }
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
        methods.add_method_mut("rerender_feedback_line", |lua, editor, ()| {
            // Force a re-render
            editor.needs_rerender = true;
            // If you can't render the editor, you're pretty much done for anyway
            let _ = editor.render(lua);
            Ok(())
        });
        methods.add_method_mut("rerender_status_line", |lua, editor, ()| {
            // Force a re-render
            editor.needs_rerender = true;
            // If you can't render the editor, you're pretty much done for anyway
            let _ = editor.render(lua);
            Ok(())
        });
        // File Tree
        methods.add_method_mut("toggle_file_tree", |_, editor, ()| {
            editor.toggle_file_tree();
            Ok(())
        });
        // Terminal
        #[cfg(not(target_os = "windows"))]
        methods.add_method_mut("open_terminal_up", |_, editor, cmd: Option<String>| {
            if let Ok(term) = Pty::new(config!(editor.config, terminal).shell) {
                if let Some(cmd) = cmd {
                    term.lock()
                        .unwrap()
                        .silent_run_command(&format!("{cmd}\n"))?;
                }
                editor.ptr = editor
                    .files
                    .open_up(editor.ptr.clone(), FileLayout::Terminal(term));
                editor.cache_old_ptr(&editor.ptr.clone());
                Ok(true)
            } else {
                Ok(false)
            }
        });
        #[cfg(not(target_os = "windows"))]
        methods.add_method_mut("open_terminal_down", |_, editor, cmd: Option<String>| {
            if let Ok(term) = Pty::new(config!(editor.config, terminal).shell) {
                if let Some(cmd) = cmd {
                    term.lock()
                        .unwrap()
                        .silent_run_command(&format!("{cmd}\n"))?;
                }
                editor.ptr = editor
                    .files
                    .open_down(editor.ptr.clone(), FileLayout::Terminal(term));
                editor.cache_old_ptr(&editor.ptr.clone());
                Ok(true)
            } else {
                Ok(false)
            }
        });
        #[cfg(not(target_os = "windows"))]
        methods.add_method_mut("open_terminal_left", |_, editor, cmd: Option<String>| {
            if let Ok(term) = Pty::new(config!(editor.config, terminal).shell) {
                if let Some(cmd) = cmd {
                    term.lock()
                        .unwrap()
                        .silent_run_command(&format!("{cmd}\n"))?;
                }
                editor.ptr = editor
                    .files
                    .open_left(editor.ptr.clone(), FileLayout::Terminal(term));
                editor.cache_old_ptr(&editor.ptr.clone());
                Ok(true)
            } else {
                Ok(false)
            }
        });
        #[cfg(not(target_os = "windows"))]
        methods.add_method_mut("open_terminal_right", |_, editor, cmd: Option<String>| {
            if let Ok(term) = Pty::new(config!(editor.config, terminal).shell) {
                if let Some(cmd) = cmd {
                    term.lock()
                        .unwrap()
                        .silent_run_command(&format!("{cmd}\n"))?;
                }
                editor.ptr = editor
                    .files
                    .open_right(editor.ptr.clone(), FileLayout::Terminal(term));
                editor.cache_old_ptr(&editor.ptr.clone());
                Ok(true)
            } else {
                Ok(false)
            }
        });
        #[cfg(not(target_os = "windows"))]
        methods.add_method_mut("run_file", |lua, editor, ()| {
            if let Some(doc) = editor.try_doc() {
                // Get file type
                let kind = editor
                    .files
                    .get(editor.ptr.clone())
                    .unwrap_or(&FileContainer::default())
                    .file_type
                    .clone()
                    .map_or("Unknown".to_string(), |ft| ft.name);
                // Get the file path
                let file_path = get_absolute_path(&doc.file_name.clone().unwrap_or_default());
                // If the file path is valid...
                if let Some(path) = file_path {
                    // ...and we know how to run the file...
                    let runcmds = lua.globals().get::<HashMap<String, RunCommand>>("runner")?;
                    if let Some(cmds) = runcmds.get(&kind) {
                        let RunCommand { compile, run } = cmds;
                        // ...open a terminal...
                        if let Ok(term) = Pty::new(config!(editor.config, terminal).shell) {
                            editor.ptr = editor
                                .files
                                .open_right(editor.ptr.clone(), FileLayout::Terminal(term));
                            editor.cache_old_ptr(&editor.ptr.clone());
                            let _ = editor.render(lua);
                            // ...then compile and run the code
                            if let Some(FileLayout::Terminal(term)) =
                                editor.files.get_raw(editor.ptr.clone())
                            {
                                if let Some(compile_cmd) = compile {
                                    let compile_cmd = compile_cmd.replace("{file_path}", &path);
                                    term.lock()
                                        .unwrap()
                                        .run_command(&format!("{compile_cmd}\n"))?;
                                }
                                if let Some(run_cmd) = run {
                                    let run_cmd = run_cmd.replace("{file_path}", &path);
                                    term.lock().unwrap().run_command(&format!("{run_cmd}\n"))?;
                                }
                            }
                        }
                    }
                }
            }
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
        // Macro
        methods.add_method_mut("macro_record_start", |_, editor, ()| {
            editor.macro_man.record();
            Ok(())
        });
        methods.add_method_mut("macro_record_stop", |_, editor, ()| {
            editor.macro_man.finish();
            Ok(())
        });
        methods.add_method_mut("macro_play", |_, editor, times: usize| {
            editor.macro_man.finish();
            editor.macro_man.play(times);
            if let Some(doc) = editor.try_doc_mut() {
                doc.commit();
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

impl IntoLua for LuaLoc {
    /// Convert this rust struct so the plug-in and configuration system can use it
    fn into_lua(self, lua: &Lua) -> std::result::Result<LuaValue, LuaError> {
        let table = lua.create_table()?;
        table.set("x", self.x)?;
        table.set("y", self.y)?;
        Ok(LuaValue::Table(table))
    }
}
