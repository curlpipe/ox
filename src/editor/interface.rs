/// Functions for rendering the UI
use crate::config::SyntaxHighlighting as SH;
use crate::editor::{FTParts, FileLayout};
use crate::error::{OxError, Result};
use crate::events::wait_for_event_hog;
use crate::ui::{key_event, size, Feedback};
#[cfg(not(target_os = "windows"))]
use crate::ui::{remove_ansi_codes, replace_reset, strip_escape_codes};
use crate::{config, display, handle_lua_error};
use crossterm::{
    event::{KeyCode as KCode, KeyModifiers as KMod},
    style::{Attribute, Color, SetAttribute, SetBackgroundColor as Bg, SetForegroundColor as Fg},
};
use kaolinite::utils::{file_or_dir, get_cwd, get_parent, list_dir, width, width_char, Loc, Size};
use mlua::Lua;
use std::ops::Range;
use synoptic::{trim_fit, Highlighter, TokOpt};

use super::Editor;

/// Render cache to store the results of any calculations during rendering
#[derive(Default)]
pub struct RenderCache {
    pub greeting_message: (String, Vec<usize>),
    pub span: Vec<(Vec<usize>, Range<usize>, Range<usize>)>,
    pub help_message: Vec<(bool, String)>,
    pub help_message_width: usize,
    pub help_message_span: Range<usize>,
    pub file_tree: FTParts,
    pub file_tree_selection: Option<usize>,
    pub term_cursor: Option<Loc>,
}

impl Editor {
    /// Update the render cache
    #[allow(clippy::range_plus_one)]
    pub fn update_render_cache(&mut self, lua: &Lua, size: Size) {
        // Calculate greeting message
        if config!(self.config, tab_line).enabled && self.greet {
            self.render_cache.greeting_message = config!(self.config, greeting_message).render(lua);
        }
        // Calculate span
        self.render_cache.span = self.files.span(vec![], size, Loc::at(0, 0));
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
        let help_start = (size.h / 2).saturating_sub(help_length / 2) + 1;
        let help_end = help_start + help_length;
        self.render_cache.help_message_span = help_start..help_end + 1;
        // Calculate file tree display representation
        let fts = &config!(self.config, document).file_types;
        let ft_config = &config!(self.config, file_tree);
        if let Some(file_tree) = self.file_tree.as_ref() {
            let (files, sel) = file_tree.display(
                self.file_tree_selection.as_ref().unwrap_or(&String::new()),
                fts,
                ft_config,
            );
            self.render_cache.file_tree = files;
            self.render_cache.file_tree_selection = sel;
        }
        // Clear the terminal cursor position
        self.render_cache.term_cursor = None;
    }

    /// Render a specific line
    #[allow(clippy::similar_names)]
    pub fn render_line(&mut self, y: usize, size: Size, lua: &Lua, sh: &SH) -> Result<String> {
        let tab_line_enabled = config!(self.config, tab_line).enabled;
        let split_bg = Bg(config!(self.config, colors).split_bg.to_color()?);
        let split_fg = Fg(config!(self.config, colors).split_fg.to_color()?);
        let mut result = String::new();
        let fcs = FileLayout::line(y, &self.render_cache.span);
        // Accounted for is used to detect gaps in lines (which should be filled with vertical bars)
        let mut accounted_for = 0;
        // Render each component of this line
        for (c, (fc, rows, range)) in fcs.iter().enumerate() {
            let in_file_tree = matches!(
                self.files.get_raw(fc.to_owned()),
                Some(FileLayout::FileTree)
            );
            let in_terminal = matches!(
                self.files.get_raw(fc.to_owned()),
                Some(FileLayout::Terminal(_))
            );
            // Check if we have encountered an area of discontinuity in the line
            if range.start != accounted_for {
                // Discontinuity detected, fill with vertical bar!
                let fill_length = range.start.saturating_sub(accounted_for);
                result += &format!("{split_bg}{split_fg}");
                for at in 0..fill_length.saturating_sub(1) {
                    let empty_below = FileLayout::is_empty_at(
                        y.saturating_add(1),
                        at + accounted_for,
                        &self.render_cache.span,
                    );
                    let empty_above = FileLayout::is_empty_at(
                        y.saturating_sub(1),
                        at + accounted_for,
                        &self.render_cache.span,
                    );
                    if empty_below && empty_above && at != fill_length.saturating_sub(1) {
                        result += "┼";
                    } else {
                        result += "─";
                    }
                }
                result += "┤";
            }
            // Render this part of the line
            let length = range.end.saturating_sub(range.start);
            let height = rows.end.saturating_sub(rows.start);
            let rel_y = y.saturating_sub(rows.start);
            if in_file_tree {
                // Part of file tree!
                result += &self.render_file_tree(y, length)?;
            } else if in_terminal {
                // Part of terminal!
                result += &self.render_terminal(fc, rel_y, length, height)?;
            } else if y == rows.start && tab_line_enabled {
                // Tab line
                result += &self.render_tab_line(fc, lua, length)?;
            } else if y == rows.end.saturating_sub(1) {
                // Status line
                result += &self.render_status_line(fc, lua, length)?;
            } else {
                // Line of file
                result += &self.render_file(
                    fc,
                    rel_y.saturating_sub(self.push_down),
                    Size {
                        w: length,
                        h: height,
                    },
                    sh,
                )?;
            }
            // Insert vertical bar where appropriate
            if c == fcs.len().saturating_sub(1) {
                accounted_for = range.end;
            } else {
                result += &format!("{split_bg}{split_fg}");
                result += if fcs[c + 1].2.start == range.end + 1 {
                    // There is no vertical bar after this part
                    "│"
                } else {
                    // There is a vertical bar after this part
                    "├"
                };
                accounted_for = range.end + 1;
            }
        }
        // Tack on any last vertical bar that is needed
        if size.w != accounted_for {
            // Discontinuity detected at the end, fill with vertical bar!
            let fill_length = (size.w + 1).saturating_sub(accounted_for);
            result += &format!("{split_bg}{split_fg}");
            for at in 0..fill_length {
                let empty_below = FileLayout::is_empty_at(
                    y.saturating_add(1),
                    at + accounted_for,
                    &self.render_cache.span,
                );
                let empty_above = FileLayout::is_empty_at(
                    y.saturating_sub(1),
                    at + accounted_for,
                    &self.render_cache.span,
                );
                result += if at == 0 && accounted_for != 0 {
                    "├"
                } else if empty_below && empty_above && at != fill_length.saturating_sub(1) {
                    "┼"
                } else if empty_below && at != fill_length.saturating_sub(1) {
                    "┬"
                } else if empty_above && at != fill_length.saturating_sub(1) {
                    "┴"
                } else {
                    "─"
                };
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
        // Get size information
        let size = size()?;
        let Size { w, mut h } = size;
        h = h.saturating_sub(1 + self.push_down);
        // Update the cache before rendering
        self.update_render_cache(lua, size);
        // Update all document's size
        let updates = self.files.update_doc_sizes(&self.render_cache.span, self);
        for (ptr, doc_idx, new_size) in updates {
            let doc = &mut self.files.get_atom_mut(ptr.clone()).unwrap().0[doc_idx].doc;
            doc.size = new_size;
            doc.load_to(doc.offset.y + doc.size.h + 1);
            self.update_highlighter_for(&ptr, doc_idx);
        }
        // Hide the cursor before rendering
        self.terminal.hide_cursor();
        // Render each line of the document
        let syntax = config!(self.config, syntax);
        for y in 0..size.h {
            let line = self.render_line(y, size, lua, &syntax)?;
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
        let in_file_tree = matches!(
            self.files.get_raw(self.ptr.clone()),
            Some(FileLayout::FileTree)
        );
        let in_terminal = matches!(
            self.files.get_raw(self.ptr.clone()),
            Some(FileLayout::Terminal(_))
        );
        match (in_file_tree, in_terminal) {
            // Move cursor to location within file
            (false, false) => {
                let Loc { x, y } = self.try_doc().unwrap().cursor_loc_in_screen()?;
                for (ptr, rows, cols) in &self.render_cache.span {
                    if ptr == &self.ptr {
                        return Some(Loc {
                            x: cols.start + x + self.dent(),
                            y: rows.start + y + self.push_down,
                        });
                    }
                }
            }
            // Move cursor to location within a terminal
            (false, true) => {
                if let Some(loc) = self.render_cache.term_cursor {
                    for (ptr, rows, cols) in &self.render_cache.span {
                        if ptr == &self.ptr {
                            return Some(Loc {
                                x: cols.start + loc.x,
                                y: rows.start + loc.y,
                            });
                        }
                    }
                }
            }
            _ => (),
        }
        None
    }

    /// Render the lines of the document
    #[allow(clippy::similar_names, clippy::too_many_lines)]
    pub fn render_file(&mut self, ptr: &[usize], y: usize, size: Size, sh: &SH) -> Result<String> {
        let Size { mut w, h } = size;
        let mut result = String::new();
        // Get various information
        let editor_bg = Bg(config!(self.config, colors).editor_bg.to_color()?);
        let editor_fg = Fg(config!(self.config, colors).editor_fg.to_color()?);
        let line_number_bg = Bg(config!(self.config, colors).line_number_bg.to_color()?);
        let line_number_fg = Fg(config!(self.config, colors).line_number_fg.to_color()?);
        let selection_bg = Bg(config!(self.config, colors).selection_bg.to_color()?);
        let selection_fg = Fg(config!(self.config, colors).selection_fg.to_color()?);
        let underline = SetAttribute(Attribute::Underlined);
        let no_underline = SetAttribute(Attribute::NoUnderline);
        let tab_width = config!(self.config, document).tab_width;
        let line_numbers_enabled = config!(self.config, line_numbers).enabled;
        let ln_pad_left = config!(self.config, line_numbers).padding_left;
        let ln_pad_right = config!(self.config, line_numbers).padding_right;
        let fc = self.files.get(ptr.to_owned()).unwrap();
        let doc = &fc.doc;
        let selection = doc.selection_loc_bound_disp();
        let has_file = doc.file_name.is_none();
        // Refuse to render help message on splits - awkward edge case
        let help_message_here = config!(self.config, help_message).enabled
            && self.render_cache.help_message_span.contains(&y)
            && self.files.n_atoms() == 1;
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
            // Run some more calcs
            let is_focus = self.ptr == ptr;
            let has_selection_somewhere = doc.cursor.selection_end != doc.cursor.loc;
            for token in tokens {
                // Find out the text (and colour of that text)
                let (text, colour, feedback) = self.breakdown_token(token, sh)?;
                if let Some(fb) = feedback {
                    self.feedback = fb;
                }
                // Do the rendering (including selection where applicable)
                for c in text.chars() {
                    let disp_loc = Loc::at(x_disp, at_line);
                    let char_loc = Loc::at(x_char, at_line);
                    // Work out selection
                    let is_selected = is_focus
                        && has_selection_somewhere
                        && doc.is_this_loc_selected_disp(disp_loc, selection);
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
                    let multi_cursor_here = doc.has_cursor(char_loc).is_some();
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
                    let c_width = width_char(&c, tab_width);
                    x_disp += c_width;
                    total_width += c_width;
                }
            }
            result += &format!("{editor_fg}{editor_bg}{cache_fg}");
            result += &" ".repeat(w.saturating_sub(total_width));
        } else if config!(self.config, greeting_message).enabled && self.greet && has_file {
            // Render the greeting message (if enabled)
            result += &self.render_greeting(y, w, h)?;
        } else {
            // Empty line, just pad out with spaces to prevent artefacts
            result += &" ".repeat(w);
        }
        // Add on help message if applicable
        if help_message_here {
            result += &self.render_help_message(y)?;
        }
        // Send out the result
        Ok(result)
    }

    /// Render help message
    pub fn render_help_message(&self, y: usize) -> Result<String> {
        let tab_width = config!(self.config, document).tab_width;
        let colors = Fg(config!(self.config, colors).highlight.to_color()?);
        let editor_fg = Fg(config!(self.config, colors).editor_fg.to_color()?);
        let at = y.saturating_sub(self.render_cache.help_message_span.start);
        let max_width = self.render_cache.help_message_width;
        let (hl, msg) = self
            .render_cache
            .help_message
            .get(at)
            .map_or((false, " ".repeat(max_width)), |(hl, content)| {
                (*hl, content.to_string())
            });
        let extra_padding = " ".repeat(max_width.saturating_sub(width(&msg, tab_width)));
        if hl {
            Ok(format!("{colors}{msg}{extra_padding}{editor_fg}"))
        } else {
            Ok(format!("{editor_fg}{msg}{extra_padding}"))
        }
    }

    /// Take a token and try to break it down into a colour and text
    pub fn breakdown_token(
        &self,
        token: TokOpt,
        sh: &SH,
    ) -> Result<(String, Fg, Option<Feedback>)> {
        let editor_fg = Fg(config!(self.config, colors).editor_fg.to_color()?);
        match token {
            // Non-highlighted text
            TokOpt::Some(text, kind) => {
                let colour = sh.get_theme(&kind);
                let mut feedback = None;
                let colour = match colour {
                    // Success, write token
                    Ok(col) => Fg(col),
                    // Failure, show error message and don't highlight this token
                    Err(err) => {
                        feedback = Some(Feedback::Error(err.to_string()));
                        editor_fg
                    }
                };
                Ok((text, colour, feedback))
            }
            // Highlighted text
            TokOpt::None(text) => Ok((text, editor_fg, None)),
        }
    }

    /// Get list of tabs
    pub fn get_tab_parts(
        &mut self,
        ptr: &[usize],
        lua: &Lua,
        w: usize,
    ) -> (Vec<String>, usize, usize) {
        let mut headers: Vec<String> = vec![];
        let mut idx = 0;
        let mut length = 0;
        let mut offset = 0;
        let tab_line = config!(self.config, tab_line);
        let doc_idx = self
            .files
            .get_atom(ptr.to_owned())
            .map_or(0, |(_, doc_idx)| doc_idx);
        for (c, file) in self.files.get_all(ptr.to_vec()).iter().enumerate() {
            let render = tab_line.render(lua, file, &mut self.feedback);
            length += width(&render, 4) + 1;
            headers.push(render);
            if c == doc_idx {
                idx = headers.len().saturating_sub(1);
            }
            while c == doc_idx && length >= w && headers.len() > 1 {
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
    pub fn render_tab_line(&mut self, ptr: &[usize], lua: &Lua, w: usize) -> Result<String> {
        let tab_inactive_bg = Bg(config!(self.config, colors).tab_inactive_bg.to_color()?);
        let tab_inactive_fg = Fg(config!(self.config, colors).tab_inactive_fg.to_color()?);
        let tab_active_bg = Bg(config!(self.config, colors).tab_active_bg.to_color()?);
        let tab_active_fg = Fg(config!(self.config, colors).tab_active_fg.to_color()?);
        let tab_width = config!(self.config, document).tab_width;
        let separator_enabled = config!(self.config, tab_line).separators;
        let mut current_width = 0;
        let (tabs, idx, _) = self.get_tab_parts(ptr, lua, w);
        let mut result = format!("{tab_inactive_fg}{tab_inactive_bg}");
        for (c, header) in tabs.iter().enumerate() {
            // Work out what to render and what not to render based on situation
            let pushes_over =
                (current_width + width(header, tab_width) + usize::from(separator_enabled))
                    .saturating_sub(w);
            let render_sep = separator_enabled && pushes_over == 0;
            // Calculate the string format
            if c == idx {
                result += &format!(
                    "{tab_active_bg}{tab_active_fg}{}{header}{}{tab_inactive_fg}{tab_inactive_bg}{}",
                    SetAttribute(Attribute::Bold),
                    SetAttribute(Attribute::Reset),
                    if render_sep { "│" } else { "" },
                );
            } else {
                result += &format!("{header}{}", if render_sep { "│" } else { "" });
            }
            current_width += width(header, tab_width) + usize::from(render_sep);
            // Don't bother continuing to render if we've gone over
            if pushes_over > 0 {
                break;
            }
        }
        // Pad out
        result += &" ".to_string().repeat(w.saturating_sub(current_width));
        Ok(result)
    }

    /// Render the status line at the bottom of the document
    #[allow(clippy::similar_names)]
    pub fn render_status_line(&mut self, ptr: &[usize], lua: &Lua, w: usize) -> Result<String> {
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
    fn render_greeting(&mut self, y: usize, w: usize, h: usize) -> Result<String> {
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

    /// Render a line in the file tree
    #[allow(clippy::similar_names)]
    fn render_file_tree(&mut self, y: usize, length: usize) -> Result<String> {
        let selected = self.render_cache.file_tree_selection == Some(y);
        let ft_bg = Bg(config!(self.config, colors).file_tree_bg.to_color()?);
        let ft_fg = Fg(config!(self.config, colors).file_tree_fg.to_color()?);
        let ft_selection_bg = Bg(config!(self.config, colors)
            .file_tree_selection_bg
            .to_color()?);
        let ft_selection_fg = Fg(config!(self.config, colors)
            .file_tree_selection_fg
            .to_color()?);
        let ft_colors = config!(self.config, colors);
        // Perform the rendering
        let mut total_length = 0;
        let line = self.render_cache.file_tree.get(y);
        let mut line = if let Some((padding, icon, icon_colour, name)) = line {
            total_length = padding * 2 + width(icon, 4) + width(name, 4);
            if let (Some(colour), false) = (icon_colour, selected) {
                let colour = Fg(match colour.as_str() {
                    "red" => ft_colors.file_tree_red.to_color()?,
                    "orange" => ft_colors.file_tree_orange.to_color()?,
                    "yellow" => ft_colors.file_tree_yellow.to_color()?,
                    "green" => ft_colors.file_tree_green.to_color()?,
                    "lightblue" => ft_colors.file_tree_lightblue.to_color()?,
                    "darkblue" => ft_colors.file_tree_darkblue.to_color()?,
                    "purple" => ft_colors.file_tree_purple.to_color()?,
                    "pink" => ft_colors.file_tree_pink.to_color()?,
                    "brown" => ft_colors.file_tree_brown.to_color()?,
                    "grey" => ft_colors.file_tree_grey.to_color()?,
                    _ => Color::White,
                });
                format!("{}{colour}{icon}{ft_fg}{name}", "  ".repeat(*padding))
            } else {
                format!("{}{icon}{name}", "  ".repeat(*padding))
            }
        } else {
            String::new()
        };
        while total_length > length {
            if let Some(ch) = line.pop() {
                total_length -= width_char(&ch, 4);
            } else {
                break;
            }
        }
        line += &" ".repeat(length.saturating_sub(total_length));
        // Return result
        if selected {
            Ok(format!("{ft_selection_bg}{ft_selection_fg}{line}"))
        } else {
            Ok(format!("{ft_bg}{ft_fg}{line}"))
        }
    }

    /// Render the line of a terminal
    #[allow(clippy::similar_names)]
    #[cfg(not(target_os = "windows"))]
    fn render_terminal(&mut self, fc: &Vec<usize>, y: usize, l: usize, h: usize) -> Result<String> {
        if let Some(FileLayout::Terminal(term)) = self.files.get_raw(fc.to_owned()) {
            let term = term.lock().unwrap();
            let editor_fg = Fg(config!(self.config, colors).editor_fg.to_color()?).to_string();
            let editor_bg = Bg(config!(self.config, colors).editor_bg.to_color()?).to_string();
            let reset = SetAttribute(Attribute::NoBold);
            let n_lines = term.output.matches('\n').count();
            let shift_down = n_lines.saturating_sub(h.saturating_sub(1));
            // Calculate the contents and amount of padding for this line of the terminal
            let (line, pad) = if let Some(line) = term.output.split('\n').nth(shift_down + y) {
                // Calculate line and padding
                let line = line.replace(['\n', '\r'], "");
                let mut visible_line = strip_escape_codes(&line);
                // Replace resets with editor style
                visible_line = replace_reset(&visible_line, &editor_bg, &editor_fg);
                let mut w = width(&remove_ansi_codes(&line), 4);
                // Work out if this is where the cursor should be
                if n_lines.saturating_sub(shift_down) == y && self.ptr == *fc {
                    visible_line += &format!("{editor_fg}{reset}{}", term.input);
                    w += width(&term.input, 4);
                    self.render_cache.term_cursor = Some(Loc { x: w, y });
                }
                // Return the result
                (visible_line, l.saturating_sub(w))
            } else {
                (" ".repeat(l), 0)
            };
            std::mem::drop(term);
            // Calculate the final result
            Ok(format!(
                "{reset}{editor_fg}{editor_bg}{line}{}",
                " ".repeat(pad)
            ))
        } else {
            unreachable!()
        }
    }

    /// Just render a blank space in place of terminal if on windows
    #[cfg(target_os = "windows")]
    fn render_terminal(&mut self, _: &Vec<usize>, _: usize, l: usize, _: usize) -> Result<String> {
        Ok(" ".repeat(l))
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
                    // Go up a directory
                    (KMod::NONE, KCode::Left) => {
                        // Find the /
                        let dir_no_sep = input
                            .chars()
                            .take(input.chars().count().saturating_sub(1))
                            .collect::<String>();
                        if let Some(parent_cut) = dir_no_sep.rfind('/') {
                            input = input.chars().take(parent_cut + 1).collect();
                            offset = 0;
                        }
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
        if let Some((_, doc_idx)) = self.files.get_atom(self.ptr.clone()) {
            self.update_highlighter_for(&self.ptr.clone(), doc_idx);
        }
    }

    /// Update highlighter of a certain document
    pub fn update_highlighter_for(&mut self, ptr: &[usize], doc: usize) {
        let percieved = self.highlighter_for(ptr.to_owned(), doc).line_ref.len();
        if self.active {
            if let Some((ref mut fcs, _)) = self.files.get_atom_mut(ptr.to_owned()) {
                let actual = fcs[doc].doc.info.loaded_to;
                if percieved < actual {
                    let diff = actual.saturating_sub(percieved);
                    for i in 0..diff {
                        let line = fcs[doc].doc.lines[percieved + i].clone();
                        fcs[doc].highlighter.append(&line);
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

    /// Gets a mutable reference to the current document
    pub fn highlighter_for(&self, ptr: Vec<usize>, doc: usize) -> &Highlighter {
        &self.files.get_atom(ptr).unwrap().0[doc].highlighter
    }

    /// Reload the whole document in the highlighter
    pub fn reload_highlight(&mut self) {
        if let Some(file) = self.files.get_mut(self.ptr.clone()) {
            file.highlighter.run(&file.doc.lines);
        }
    }

    /// Work out how much to push the document to the right (to make way for line numbers)
    pub fn dent(&self) -> usize {
        if let Some((_, doc)) = self.files.get_atom(self.ptr.clone()) {
            self.dent_for(&self.ptr, doc)
        } else {
            0
        }
    }

    /// Work out how much to push the document to the right (to make way for line numbers)
    pub fn dent_for(&self, at: &[usize], doc: usize) -> usize {
        if config!(self.config, line_numbers).enabled {
            let padding_left = config!(self.config, line_numbers).padding_left;
            let padding_right = config!(self.config, line_numbers).padding_right;
            if let Some((fcs, _)) = self.files.get_atom(at.to_owned()) {
                fcs[doc].doc.len_lines().to_string().len() + 1 + padding_left + padding_right
            } else {
                0
            }
        } else {
            0
        }
    }
}
