use crate::editor::{FileContainer, FileLayout};
/// Functions for rendering the UI
use crate::error::{OxError, Result};
use crate::events::wait_for_event_hog;
use crate::ui::{key_event, size, Feedback};
use crate::{config, display, handle_lua_error};
use crossterm::{
    event::{KeyCode as KCode, KeyModifiers as KMod},
    style::{Attribute, Color, SetAttribute, SetBackgroundColor as Bg, SetForegroundColor as Fg},
};
use kaolinite::utils::{file_or_dir, get_cwd, get_parent, list_dir, width, Loc, Size};
use mlua::Lua;
use std::ops::Range;
use synoptic::{trim_fit, Highlighter, TokOpt};

use super::Editor;

/// Render cache to store the results of any calculations during rendering
#[derive(Default)]
pub struct RenderCache {
    greeting_message: (String, Vec<usize>),
    span: Vec<(Vec<usize>, Range<usize>, Range<usize>)>,
    help_message: Vec<(bool, String)>,
    help_message_width: usize,
    help_message_span: Range<usize>,
}

impl Editor {
    /// Update the render cache
    pub fn update_render_cache(&mut self, lua: &Lua, size: Size) {
        // Calculate greeting message
        if config!(self.config, tab_line).enabled && self.greet {
            if let Ok(gm) = config!(self.config, greeting_message).render(lua) {
                self.render_cache.greeting_message = gm;
            }
        }
        // Calculate span
        self.render_cache.span = self.files.span(vec![], size);
        // Calculate help message information
        let tab_width = config!(self.config, document).tab_width;
        self.render_cache.help_message = config!(self.config, help_message).render(lua);
        self.render_cache.help_message_width = self
            .render_cache
            .help_message
            .iter()
            .map(|(_, line)| width(line, tab_width))
            .max()
            .unwrap_or(0)
            + 5;
        let help_length = self.render_cache.help_message.len();
        let help_start =
            usize::try_from((size.h / 2).saturating_sub(help_length / 2) + 1).unwrap_or(usize::MAX);
        let help_end = help_start + usize::try_from(help_length).unwrap_or(usize::MAX) as usize;
        self.render_cache.help_message_span = help_start..help_end + 1;
    }

    /// Render a specific line
    pub fn render_line(&mut self, y: usize, size: Size, lua: &Lua) -> Result<String> {
        let tab_line_enabled = config!(self.config, tab_line).enabled;
        let mut result = String::new();
        let fcs = FileLayout::line(y, &self.render_cache.span);
        for (mut c, (fc, rows, range)) in fcs.iter().enumerate() {
            let length = range.end.saturating_sub(range.start);
            // Insert horizontal bar where appropriate (horribly janky implementation, but it works)
            if vec![42].repeat(100) == *fc {
                if y == rows.end.saturating_sub(1) {
                    result += &"─".repeat(length);
                }
                continue;
            }
            let rel_y = y.saturating_sub(rows.start);
            if y == rows.start && tab_line_enabled {
                // Tab line
                result += &self.render_tab_line(&fc, lua, length)?;
            } else if y == rows.end.saturating_sub(1) {
                // Status line
                result += &self.render_status_line(&fc, lua, length)?;
            } else {
                // Line of file
                result += &self.render_document(
                    &fc,
                    rel_y.saturating_sub(self.push_down),
                    lua,
                    Size {
                        w: length,
                        h: size.h,
                    },
                )?;
            }
            let editor_bg = Bg(config!(self.config, colors).editor_bg.to_color()?);
            let editor_fg = Fg(config!(self.config, colors).editor_fg.to_color()?);
            // Insert vertical bar where appropriate
            if c != fcs.len().saturating_sub(1) {
                result += &format!("{editor_bg}{editor_fg}│");
            }
        }
        Ok(result)
    }

    /// Render a single frame of the editor in it's current state
    pub fn render(&mut self, lua: &Lua) -> Result<()> {
        // Determine if re-rendering is needed
        if !self.needs_rerender {
            return Ok(());
        }
        self.needs_rerender = false;
        // Get size information and update the document's size
        let mut size = size()?;
        let Size { w, mut h } = size.clone();
        h = h.saturating_sub(1 + self.push_down);
        let max = self.dent();
        self.doc_mut().size.w = w.saturating_sub(max);
        // Update the cache before rendering
        self.update_render_cache(lua, size);
        // Hide the cursor before rendering
        self.terminal.hide_cursor();
        // Render each line of the document
        for y in 0..size.h {
            let line = self.render_line(y, size, lua)?;
            self.terminal.goto(0, y);
            display!(self, line);
        }
        // Render the feedback line
        self.render_feedback_line(w, h)?;
        // Move cursor to the correct location and perform render
        if let Some(Loc { x, y }) = self.cursor_position() {
            self.terminal.show_cursor();
            self.terminal.goto(x, y);
        }
        self.terminal.flush()?;
        Ok(())
    }

    /// Function to calculate the cursor's position on screen
    pub fn cursor_position(&self) -> Option<Loc> {
        let Loc { x, y } = self.doc().cursor_loc_in_screen()?;
        for (ptr, rows, cols) in &self.render_cache.span {
            if ptr == &self.ptr {
                return Some(Loc {
                    x: cols.start + x + self.dent(),
                    y: rows.start + y + self.push_down,
                });
            }
        }
        None
    }

    /// Render the lines of the document
    pub fn render_document(
        &mut self,
        ptr: &Vec<usize>,
        y: usize,
        lua: &Lua,
        size: Size,
    ) -> Result<String> {
        let Size { mut w, h } = size;
        let mut result = String::new();
        // Get various information
        let editor_bg = Bg(config!(self.config, colors).editor_bg.to_color()?);
        let editor_fg = Fg(config!(self.config, colors).editor_fg.to_color()?);
        let line_number_bg = Bg(config!(self.config, colors).line_number_bg.to_color()?);
        let line_number_fg = Fg(config!(self.config, colors).line_number_fg.to_color()?);
        let selection_bg = Bg(config!(self.config, colors).selection_bg.to_color()?);
        let selection_fg = Fg(config!(self.config, colors).selection_fg.to_color()?);
        let colors = Fg(config!(self.config, colors).highlight.to_color()?);
        let underline = SetAttribute(Attribute::Underlined);
        let no_underline = SetAttribute(Attribute::NoUnderline);
        let tab_width = config!(self.config, document).tab_width;
        let line_numbers_enabled = config!(self.config, line_numbers).enabled;
        let ln_pad_left = config!(self.config, line_numbers).padding_left;
        let ln_pad_right = config!(self.config, line_numbers).padding_right;
        let selection = self.doc().selection_loc_bound_disp();
        let fc = self.files.get(ptr.clone()).unwrap();
        let doc = &fc.doc;
        let help_message_here = config!(self.config, help_message).enabled
            && self.render_cache.help_message_span.contains(&y);
        // Render short of the help message
        let mut total_width = if help_message_here {
            self.render_cache.help_message_width
        } else {
            0
        };
        // Render the line numbers if enabled
        if line_numbers_enabled {
            let num = doc.line_number(y + doc.offset.y);
            let padding_left = " ".repeat(ln_pad_left);
            let padding_right = " ".repeat(ln_pad_right);
            result += &format!("{line_number_bg}{line_number_fg}{padding_left}{num}{padding_right}│{editor_fg}{editor_bg}");
            total_width += ln_pad_left + ln_pad_right + width(&num, tab_width) + 1;
        } else {
            result += &format!("{editor_fg}{editor_bg}");
        }
        w = w.saturating_sub(total_width);
        // Render the body of the document if available
        let at_line = y + doc.offset.y;
        if let Some(line) = doc.line(at_line) {
            // Reset the cache
            let mut cache_bg = editor_bg;
            let mut cache_fg = editor_fg;
            // Gather the tokens
            let tokens = fc.highlighter.line(at_line, &line);
            let tokens = trim_fit(&tokens, doc.offset.x, w, tab_width);
            let mut x_disp = doc.offset.x;
            let mut x_char = doc.character_idx(&doc.offset);
            for token in tokens {
                // Find out the text (and colour of that text)
                let (text, colour) = match token {
                    // Non-highlighted text
                    TokOpt::Some(text, kind) => {
                        let colour = config!(self.config, syntax).get_theme(&kind);
                        let colour = match colour {
                            // Success, write token
                            Ok(col) => Fg(col),
                            // Failure, show error message and don't highlight this token
                            Err(err) => {
                                self.feedback = Feedback::Error(err.to_string());
                                editor_fg
                            }
                        };
                        (text, colour)
                    }
                    // Highlighted text
                    TokOpt::None(text) => (text, editor_fg),
                };
                // Do the rendering (including selection where applicable)
                for c in text.chars() {
                    let disp_loc = Loc {
                        y: at_line,
                        x: x_disp,
                    };
                    let char_loc = Loc {
                        y: at_line,
                        x: x_char,
                    };
                    // Work out selection
                    let is_selected = &self.ptr == ptr && self.doc().is_this_loc_selected_disp(disp_loc, selection);
                    // Render the correct colour
                    if is_selected {
                        if cache_bg != selection_bg {
                            result += &selection_bg.to_string();
                            cache_bg = selection_bg;
                        }
                        if cache_fg != selection_fg {
                            result += &selection_fg.to_string();
                            cache_fg = selection_fg;
                        }
                    } else {
                        if cache_bg != editor_bg {
                            result += &editor_bg.to_string();
                            cache_bg = editor_bg;
                        }
                        if cache_fg != colour {
                            result += &colour.to_string();
                            cache_fg = colour;
                        }
                    }
                    // Render multi-cursors
                    let multi_cursor_here = self.doc().has_cursor(char_loc).is_some();
                    if multi_cursor_here {
                        result += &format!("{underline}{}{}", Bg(Color::White), Fg(Color::Black));
                    }
                    // Render the character
                    result.push(c);
                    // Reset any multi-cursor display
                    if multi_cursor_here {
                        result += &format!("{no_underline}{cache_bg}{cache_fg}");
                    }
                    x_char += 1;
                    let c_width = width(&c.to_string(), tab_width);
                    x_disp += c_width;
                    total_width += c_width;
                }
            }
            result += &format!("{editor_fg}{editor_bg}{cache_fg}");
            result += &" ".repeat(w.saturating_sub(total_width));
        } else if config!(self.config, greeting_message).enabled && self.greet {
            // Render the greeting message (if enabled)
            result += &self.render_greeting(y, lua, w, h)?;
        } else {
            // Empty line, just pad out with spaces to prevent artefacts
            result += &" ".repeat(w);
        }
        // Add on help message if applicable
        if help_message_here {
            let at = y.saturating_sub(self.render_cache.help_message_span.start);
            let max_width = self.render_cache.help_message_width;
            let (hl, msg) = self
                .render_cache
                .help_message
                .get(at)
                .map(|(hl, content)| (*hl, content.to_string()))
                .unwrap_or((false, " ".repeat(max_width)));
            let extra_padding = " ".repeat(max_width.saturating_sub(width(&msg, tab_width)));
            if hl {
                result += &format!("{colors}{msg}{extra_padding}{editor_fg}");
            } else {
                result += &format!("{editor_fg}{msg}{extra_padding}");
            }
        }
        // Send out the result
        Ok(result)
    }

    /// Get list of tabs
    pub fn get_tab_parts(
        &mut self,
        ptr: &Vec<usize>,
        lua: &Lua,
        w: usize,
    ) -> (Vec<String>, usize, usize) {
        let mut headers: Vec<String> = vec![];
        let mut idx = 0;
        let mut length = 0;
        let mut offset = 0;
        let tab_line = config!(self.config, tab_line);
        for (c, file) in self.files.get_all(ptr.to_vec()).iter().enumerate() {
            let render = tab_line.render(lua, file, &mut self.feedback);
            length += width(&render, 4) + 1;
            headers.push(render);
            let ptr = self
                .files
                .get_atom(self.ptr.clone())
                .map_or(0, |(_, ptr)| ptr);
            if c == ptr {
                idx = headers.len().saturating_sub(1);
            }
            while c == ptr && length > w && headers.len() > 1 {
                headers.remove(0);
                length = length.saturating_sub(width(&headers[0], 4) + 1);
                idx = headers.len().saturating_sub(1);
                offset += 1;
            }
        }
        (headers, idx, offset)
    }

    /// Render the tab line at the top of the document
    #[allow(clippy::similar_names)]
    pub fn render_tab_line(&mut self, ptr: &Vec<usize>, lua: &Lua, w: usize) -> Result<String> {
        let tab_inactive_bg = Bg(config!(self.config, colors).tab_inactive_bg.to_color()?);
        let tab_inactive_fg = Fg(config!(self.config, colors).tab_inactive_fg.to_color()?);
        let tab_active_bg = Bg(config!(self.config, colors).tab_active_bg.to_color()?);
        let tab_active_fg = Fg(config!(self.config, colors).tab_active_fg.to_color()?);
        let tab_width = config!(self.config, document).tab_width;
        let mut current_width = 0;
        let (tabs, idx, _) = self.get_tab_parts(ptr, lua, w);
        let mut result = format!("{tab_inactive_fg}{tab_inactive_bg}");
        for (c, header) in tabs.iter().enumerate() {
            if c == idx {
                result += &format!(
                    "{tab_active_bg}{tab_active_fg}{}{header}{}{tab_inactive_fg}{tab_inactive_bg}│",
                    SetAttribute(Attribute::Bold),
                    SetAttribute(Attribute::Reset),
                );
            } else {
                result += &format!("{header}│");
            }
            current_width += width(header, tab_width) + 1;
        }
        result += &" ".to_string().repeat(w.saturating_sub(current_width));
        Ok(result)
    }

    /// Render the status line at the bottom of the document
    #[allow(clippy::similar_names)]
    pub fn render_status_line(&mut self, ptr: &Vec<usize>, lua: &Lua, w: usize) -> Result<String> {
        let editor_bg = Bg(config!(self.config, colors).editor_bg.to_color()?);
        let editor_fg = Fg(config!(self.config, colors).editor_fg.to_color()?);
        let status_bg = Bg(config!(self.config, colors).status_bg.to_color()?);
        let status_fg = Fg(config!(self.config, colors).status_fg.to_color()?);
        let mut result = String::new();
        result += &format!("{status_bg}{status_fg}");
        match config!(self.config, status_line).render(ptr, self, lua, w) {
            Ok(content) => {
                if content.is_empty() {
                    result += &" ".repeat(w);
                } else {
                    result += &format!(
                        "{}{content}{}",
                        SetAttribute(Attribute::Bold),
                        SetAttribute(Attribute::Reset),
                    );
                }
            }
            Err(lua_error) => {
                result += &" ".repeat(w);
                handle_lua_error("status_line", Err(lua_error), &mut self.feedback);
            }
        }
        result += &format!("{editor_fg}{editor_bg}");
        Ok(result)
    }

    /// Render the feedback line
    pub fn render_feedback_line(&mut self, w: usize, h: usize) -> Result<()> {
        self.terminal.goto(0, h + 2);
        let content = self.feedback.render(&config!(self.config, colors), w)?;
        display!(self, content);
        Ok(())
    }

    /// Render the greeting message
    fn render_greeting(&mut self, y: usize, lua: &Lua, w: usize, h: usize) -> Result<String> {
        // Produce the greeting message
        let colors = config!(self.config, colors);
        let highlight = Fg(colors.highlight.to_color()?).to_string();
        let editor_fg = Fg(colors.editor_fg.to_color()?).to_string();
        let (message, highlights) = &self.render_cache.greeting_message;
        let message: Vec<&str> = message.split('\n').collect();
        // Select the correct line
        let greeting_span = (h / 4)..(h / 4 + message.len());
        let line = if greeting_span.contains(&y) {
            message[y.saturating_sub(h / 4)]
        } else {
            ""
        };
        let mut content = alinio::align::center(line, w).unwrap_or_default();
        if highlights.contains(&y.saturating_sub(h / 4)) {
            content = format!("{highlight}{content}{editor_fg}");
        }
        // Output
        Ok(content)
    }

    /// Display a prompt in the document
    pub fn prompt<S: Into<String>>(&mut self, prompt: S) -> Result<String> {
        let prompt = prompt.into();
        let mut input = String::new();
        let mut done = false;
        // Enter into a menu that asks for a prompt
        while !done {
            let h = size()?.h;
            let w = size()?.w;
            // Render prompt message
            self.terminal.prepare_line(h);
            self.terminal.show_cursor();
            let editor_bg = Bg(config!(self.config, colors).editor_bg.to_color()?);
            display!(
                self,
                editor_bg,
                prompt.clone(),
                ": ",
                input.clone(),
                " ".to_string().repeat(w)
            );
            self.terminal.goto(prompt.len() + input.len() + 2, h);
            self.terminal.flush()?;
            // Handle events
            if let Some((modifiers, code)) =
                key_event(&wait_for_event_hog(self), &mut self.macro_man)
            {
                match (modifiers, code) {
                    // Exit the menu when the enter key is pressed
                    (KMod::NONE, KCode::Enter) => done = true,
                    // Cancel operation
                    (KMod::NONE, KCode::Esc) => return Err(OxError::Cancelled),
                    // Remove from the input string if the user presses backspace
                    (KMod::NONE, KCode::Backspace) => {
                        input.pop();
                    }
                    // Add to the input string if the user presses a character
                    (KMod::NONE | KMod::SHIFT, KCode::Char(c)) => input.push(c),
                    _ => (),
                }
            }
        }
        // Return input string result
        Ok(input)
    }

    /// Prompt for selecting a file
    #[allow(clippy::similar_names)]
    pub fn path_prompt(&mut self) -> Result<String> {
        let mut input = get_cwd()
            .map(|p| {
                if p.ends_with(std::path::MAIN_SEPARATOR) {
                    p
                } else {
                    p + std::path::MAIN_SEPARATOR_STR
                }
            })
            .unwrap_or_default();
        let mut offset = 0;
        let mut done = false;
        let mut old_suggestions = vec![];
        // Enter into a menu that asks for a prompt
        while !done {
            // Find the suggested files and folders
            let parent = if input.ends_with('/') || input.ends_with('\\') {
                input.to_string()
            } else {
                get_parent(&input).unwrap_or_default()
            };
            let suggestions = list_dir(&parent)
                .unwrap_or_default()
                .iter()
                .filter(|p| p.starts_with(&input))
                .cloned()
                .collect::<Vec<_>>();
            // Reset offset if we've changed suggestions / out of bounds
            if suggestions != old_suggestions || offset >= suggestions.len() {
                offset = 0;
            }
            old_suggestions.clone_from(&suggestions);
            // Select suggestion
            let mut suggestion = suggestions
                .get(offset)
                .map(std::string::ToString::to_string)
                .unwrap_or(input.clone());
            // Render prompt message
            let h = size()?.h;
            self.terminal.prepare_line(h);
            self.terminal.show_cursor();
            let suggestion_text = suggestion
                .chars()
                .skip(input.chars().count())
                .collect::<String>();
            let editor_fg = Fg(config!(self.config, colors).editor_fg.to_color()?);
            let editor_bg = Bg(config!(self.config, colors).editor_bg.to_color()?);
            let tab_width = config!(self.config, document).tab_width;
            let total_width = width(&input, tab_width) + width(&suggestion_text, tab_width);
            let padding = " ".repeat(size()?.w.saturating_sub(total_width));
            display!(
                self,
                editor_bg,
                "Path: ",
                input.clone(),
                Fg(Color::DarkGrey),
                suggestion_text,
                padding,
                editor_fg
            );
            let tab_width = config!(self.config, document).tab_width;
            self.terminal.goto(6 + width(&input, tab_width), h);
            self.terminal.flush()?;
            // Handle events
            if let Some((modifiers, code)) =
                key_event(&wait_for_event_hog(self), &mut self.macro_man)
            {
                match (modifiers, code) {
                    // Exit the menu when the enter key is pressed
                    (KMod::NONE, KCode::Enter) => done = true,
                    // Cancel when escape key is pressed
                    (KMod::NONE, KCode::Esc) => return Err(OxError::Cancelled),
                    // Remove from the input string if the user presses backspace
                    (KMod::NONE, KCode::Backspace) => {
                        input.pop();
                    }
                    // Add to the input string if the user presses a character
                    (KMod::NONE | KMod::SHIFT, KCode::Char(c)) => input.push(c),
                    // Autocomplete path
                    (KMod::NONE, KCode::Right) => {
                        if file_or_dir(&suggestion) == "directory" {
                            suggestion.push(std::path::MAIN_SEPARATOR);
                        }
                        input = suggestion;
                        offset = 0;
                    }
                    // Cycle through suggestions
                    (KMod::SHIFT, KCode::BackTab) => offset = offset.saturating_sub(1),
                    (KMod::NONE, KCode::Tab) => {
                        if offset + 1 < suggestions.len() {
                            offset += 1;
                        }
                    }
                    _ => (),
                }
            }
        }
        // Return input string result
        Ok(input)
    }

    /// Confirmation dialog
    pub fn confirm(&mut self, msg: &str) -> Result<bool> {
        let mut done = false;
        let mut result = false;
        // Enter into the confirmation menu
        self.terminal.hide_cursor();
        while !done {
            let h = size()?.h;
            let w = size()?.w;
            // Render message
            self.feedback = Feedback::Warning(msg.to_string());
            self.render_feedback_line(w, h)?;
            self.terminal.flush()?;
            // Handle events
            if let Some((modifiers, code)) =
                key_event(&wait_for_event_hog(self), &mut self.macro_man)
            {
                match (modifiers, code) {
                    // Exit the menu when the enter key is pressed
                    (KMod::NONE, KCode::Esc) => {
                        done = true;
                        self.feedback = Feedback::None;
                    }
                    // Add to the input string if the user presses a character
                    (KMod::CONTROL, KCode::Char('q')) => {
                        done = true;
                        result = true;
                        self.feedback = Feedback::None;
                    }
                    _ => (),
                }
            }
        }
        self.terminal.show_cursor();
        Ok(result)
    }

    /// Append any missed lines to the syntax highlighter
    pub fn update_highlighter(&mut self) {
        if self.active {
            let actual = self
                .files
                .get(self.ptr.clone())
                .map_or(0, |fc| fc.doc.info.loaded_to);
            let percieved = self.highlighter().line_ref.len();
            if percieved < actual {
                let diff = actual.saturating_sub(percieved);
                for i in 0..diff {
                    if let Some(file) = self.files.get_mut(self.ptr.clone()) {
                        let line = &file.doc.lines[percieved + i];
                        file.highlighter.append(line);
                    }
                }
            }
        }
    }

    /// Returns a highlighter at a certain index
    pub fn get_highlighter(&mut self, idx: usize) -> &mut Highlighter {
        &mut self.files.get_atom_mut(self.ptr.clone()).unwrap().0[idx].highlighter
    }

    /// Gets a mutable reference to the current document
    pub fn highlighter(&mut self) -> &mut Highlighter {
        &mut self.files.get_mut(self.ptr.clone()).unwrap().highlighter
    }

    /// Reload the whole document in the highlighter
    pub fn reload_highlight(&mut self) {
        if let Some(file) = self.files.get_mut(self.ptr.clone()) {
            file.highlighter.run(&file.doc.lines);
        }
    }

    /// Work out how much to push the document to the right (to make way for line numbers)
    pub fn dent(&self) -> usize {
        if config!(self.config, line_numbers).enabled {
            let padding_left = config!(self.config, line_numbers).padding_left;
            let padding_right = config!(self.config, line_numbers).padding_right;
            self.doc().len_lines().to_string().len() + 1 + padding_left + padding_right
        } else {
            0
        }
    }
}
