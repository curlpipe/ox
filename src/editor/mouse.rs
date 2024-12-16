use crate::editor::FileLayout;
/// For handling mouse events
use crate::{config, Result};
use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use kaolinite::{utils::width, Loc};
use mlua::Lua;
use std::time::{Duration, Instant};

use super::Editor;

/// Represents where the mouse has clicked / been dragged
enum MouseLocation {
    /// Where the mouse has clicked within a file
    File(Vec<usize>, Loc),
    /// Where the mouse has clicked on a tab
    Tabs(Vec<usize>, usize),
    /// Where the mouse has clicked in the file tree
    FileTree(usize),
    /// Where the mouse has clicked in the terminal
    Terminal(Vec<usize>),
    /// Mouse has clicked nothing of importance
    Out,
}

impl Editor {
    /// Finds the position of the mouse within the viewport
    fn find_mouse_location(&mut self, lua: &Lua, event: MouseEvent) -> MouseLocation {
        // Calculate various things beforehand
        let row = event.row as usize;
        let col = event.column as usize;
        let tab_enabled = config!(self.config, tab_line).enabled;
        let tab = usize::from(tab_enabled);
        // From a mouse click, locate the split that the user has clicked on
        let at_idx = self
            .render_cache
            .span
            .iter()
            .find(|(_, rows, cols)| rows.contains(&row) && cols.contains(&col));
        if let Some((idx, rows, cols)) = at_idx {
            let idx = idx.clone();
            // Calculate the current dent in this split
            match self.files.get_raw(idx.clone()) {
                Some(FileLayout::Atom(_, doc_idx)) => {
                    let dent = self.dent_for(&idx, *doc_idx);
                    // Split that user clicked in located - adjust event location
                    let clicked = Loc {
                        x: col.saturating_sub(cols.start),
                        y: row.saturating_sub(rows.start),
                    };
                    // Work out where the user clicked
                    if clicked.y == 0 && tab_enabled {
                        // Clicked on tab line
                        let (tabs, _, offset) =
                            self.get_tab_parts(&idx, lua, cols.end.saturating_sub(cols.start));
                        // Try to work out which tab we clicked on
                        let mut c = u16::try_from(clicked.x).unwrap_or(u16::MAX) + 2;
                        for (i, header) in tabs.iter().enumerate() {
                            let header_len = width(header, 4) + 1;
                            c = c.saturating_sub(u16::try_from(header_len).unwrap_or(u16::MAX));
                            if c == 0 {
                                // This tab was clicked on
                                return MouseLocation::Tabs(idx.clone(), i + offset);
                            }
                        }
                        // Did not click on a tab
                        MouseLocation::Out
                    } else if clicked.y == rows.end.saturating_sub(1) {
                        // Clicked on status line
                        MouseLocation::Out
                    } else if clicked.x < dent {
                        // Clicked on line numbers
                        MouseLocation::Out
                    } else if let Some((fcs, ptr)) = self.files.get_atom(idx.clone()) {
                        // Clicked on document
                        let offset = fcs[ptr].doc.offset;
                        MouseLocation::File(
                            idx.clone(),
                            Loc {
                                x: clicked.x.saturating_sub(dent) + offset.x,
                                y: clicked.y.saturating_sub(tab) + offset.y,
                            },
                        )
                    } else {
                        // We can't seem to get the atom for some reason, just default to Out
                        MouseLocation::Out
                    }
                }
                Some(FileLayout::FileTree) => MouseLocation::FileTree(row),
                Some(FileLayout::Terminal(_)) => MouseLocation::Terminal(idx),
                _ => MouseLocation::Out,
            }
        } else {
            MouseLocation::Out
        }
    }

    /// Handles a mouse event (dragging / clicking)
    #[allow(clippy::too_many_lines)]
    pub fn handle_mouse_event(&mut self, lua: &Lua, event: MouseEvent) -> Result<()> {
        match event.modifiers {
            KeyModifiers::NONE => match event.kind {
                // Single click
                MouseEventKind::Down(MouseButton::Left) => {
                    let location = self.find_mouse_location(lua, event);
                    let clicked_in_ft = matches!(location, MouseLocation::FileTree(_));
                    // Determine if there has been a click within 500ms
                    if let Some((time, last_event)) = self.last_click {
                        let now = Instant::now();
                        let short_period = now.duration_since(time) <= Duration::from_millis(500);
                        let same_location =
                            last_event.column == event.column && last_event.row == event.row;
                        // If the user quickly clicked twice in the same location (outside the file tree)
                        if short_period && same_location && !clicked_in_ft {
                            self.handle_double_click(lua, event);
                            return Ok(());
                        }
                    }
                    match location {
                        MouseLocation::File(idx, mut loc) => {
                            self.cache_old_ptr(&idx);
                            self.ptr.clone_from(&idx);
                            self.update_cwd();
                            if let Some(doc) = self.try_doc_mut() {
                                doc.clear_cursors();
                                loc.x = doc.character_idx(&loc);
                                doc.move_to(&loc);
                                doc.old_cursor = doc.loc().x;
                            }
                        }
                        MouseLocation::Tabs(idx, i) => {
                            self.files.move_to(idx.clone(), i);
                            self.cache_old_ptr(&idx);
                            self.ptr.clone_from(&idx);
                            self.update_cwd();
                        }
                        MouseLocation::FileTree(y) => {
                            // Handle the click
                            if let Some(ft) = &self.file_tree {
                                // Move selection to where we clicked
                                if let Some(item) = ft.flatten().get(y) {
                                    self.file_tree_selection = Some(item.to_string());
                                    // Toggle the node
                                    self.file_tree_open_node()?;
                                }
                            }
                        }
                        MouseLocation::Terminal(idx) => {
                            // Move focus to the index
                            self.cache_old_ptr(&idx);
                            self.ptr.clone_from(&idx);
                        }
                        MouseLocation::Out => (),
                    }
                }
                MouseEventKind::Down(MouseButton::Right) => {
                    // Select the current line
                    if let MouseLocation::File(idx, loc) = self.find_mouse_location(lua, event) {
                        self.cache_old_ptr(&idx);
                        self.ptr.clone_from(&idx);
                        self.update_cwd();
                        if let Some(doc) = self.try_doc_mut() {
                            doc.select_line_at(loc.y);
                            let line = doc.line(loc.y).unwrap_or_default();
                            self.alt_click_state = Some((
                                Loc {
                                    x: 0,
                                    y: doc.loc().y,
                                },
                                Loc {
                                    x: line.chars().count(),
                                    y: doc.loc().y,
                                },
                            ));
                        }
                    }
                }
                MouseEventKind::Up(MouseButton::Right) => {
                    self.alt_click_state = None;
                }
                // Double click detection
                MouseEventKind::Up(MouseButton::Left) => {
                    self.alt_click_state = None;
                    let now = Instant::now();
                    // Register this click as having happened
                    self.last_click = Some((now, event));
                }
                // Mouse drag
                MouseEventKind::Drag(MouseButton::Left) => {
                    match self.find_mouse_location(lua, event) {
                        MouseLocation::File(idx, mut loc) => {
                            if self.try_doc().is_some() {
                                self.cache_old_ptr(&idx);
                                self.ptr.clone_from(&idx);
                                self.update_cwd();
                                let doc = self.try_doc().unwrap();
                                loc.x = doc.character_idx(&loc);
                                if let Some((dbl_start, dbl_end)) = self.alt_click_state {
                                    let doc = self.try_doc().unwrap();
                                    if loc.x > doc.cursor.selection_end.x {
                                        // Find boundary of next word
                                        let next = doc.next_word_close(loc);
                                        let doc = self.try_doc_mut().unwrap();
                                        doc.move_to(&dbl_start);
                                        doc.select_to(&Loc { x: next, y: loc.y });
                                    } else {
                                        // Find boundary of previous word
                                        let next = doc.prev_word_close(loc);
                                        let doc = self.try_doc_mut().unwrap();
                                        doc.move_to(&dbl_end);
                                        doc.select_to(&Loc { x: next, y: loc.y });
                                    }
                                } else {
                                    let doc = self.try_doc_mut().unwrap();
                                    doc.select_to(&loc);
                                }
                            }
                        }
                        MouseLocation::Tabs(_, _)
                        | MouseLocation::Out
                        | MouseLocation::FileTree(_)
                        | MouseLocation::Terminal(_) => (),
                    }
                }
                MouseEventKind::Drag(MouseButton::Right) => {
                    match self.find_mouse_location(lua, event) {
                        MouseLocation::File(idx, mut loc) => {
                            if self.try_doc().is_some() {
                                self.cache_old_ptr(&idx);
                                self.ptr.clone_from(&idx);
                                self.update_cwd();
                                let doc = self.try_doc_mut().unwrap();
                                loc.x = doc.character_idx(&loc);
                                if let Some((line_start, line_end)) = self.alt_click_state {
                                    let doc = self.try_doc().unwrap();
                                    if loc.y > doc.cursor.selection_end.y {
                                        let line = doc.line(loc.y).unwrap_or_default();
                                        let doc = self.try_doc_mut().unwrap();
                                        doc.move_to(&line_start);
                                        doc.select_to(&Loc {
                                            x: line.chars().count(),
                                            y: loc.y,
                                        });
                                    } else {
                                        let doc = self.try_doc_mut().unwrap();
                                        doc.move_to(&line_end);
                                        doc.select_to(&Loc { x: 0, y: loc.y });
                                    }
                                } else {
                                    self.try_doc_mut().unwrap().select_to(&loc);
                                }
                            }
                        }
                        MouseLocation::Tabs(_, _)
                        | MouseLocation::Out
                        | MouseLocation::FileTree(_)
                        | MouseLocation::Terminal(_) => (),
                    }
                }
                // Mouse scroll behaviour
                MouseEventKind::ScrollDown | MouseEventKind::ScrollUp => {
                    let scroll_amount = config!(self.config, terminal).scroll_amount;
                    if let MouseLocation::File(idx, _) = self.find_mouse_location(lua, event) {
                        self.cache_old_ptr(&idx);
                        self.ptr.clone_from(&idx);
                        self.update_cwd();
                        if let Some(doc) = self.try_doc_mut() {
                            for _ in 0..scroll_amount {
                                if event.kind == MouseEventKind::ScrollDown {
                                    doc.scroll_down();
                                } else {
                                    doc.scroll_up();
                                }
                            }
                        }
                    }
                }
                MouseEventKind::ScrollLeft => {
                    if let Some(doc) = self.try_doc_mut() {
                        doc.move_left();
                    }
                }
                MouseEventKind::ScrollRight => {
                    if let Some(doc) = self.try_doc_mut() {
                        doc.move_right();
                    }
                }
                _ => (),
            },
            // Multi cursor behaviour
            KeyModifiers::CONTROL => {
                if let MouseEventKind::Down(MouseButton::Left) = event.kind {
                    if let MouseLocation::File(idx, loc) = self.find_mouse_location(lua, event) {
                        self.cache_old_ptr(&idx);
                        self.ptr.clone_from(&idx);
                        self.update_cwd();
                        if let Some(doc) = self.try_doc_mut() {
                            doc.new_cursor(loc);
                            doc.commit();
                        }
                    }
                }
            }
            _ => (),
        }
        Ok(())
    }

    /// Handle a double-click event
    pub fn handle_double_click(&mut self, lua: &Lua, event: MouseEvent) {
        // Select the current word
        if let MouseLocation::File(idx, loc) = self.find_mouse_location(lua, event) {
            self.cache_old_ptr(&idx);
            self.ptr.clone_from(&idx);
            self.update_cwd();
            if let Some(doc) = self.try_doc_mut() {
                doc.select_word_at(&loc);
                let mut selection = doc.cursor.selection_end;
                let mut cursor = doc.cursor.loc;
                selection.x = doc.character_idx(&selection);
                cursor.x = doc.character_idx(&cursor);
                self.alt_click_state = Some((selection, cursor));
            }
        }
    }

    /// Cache the old ptr
    pub fn cache_old_ptr(&mut self, idx: &Vec<usize>) {
        self.old_ptr.clone_from(idx);
        if self.file_tree_is_open() && !self.old_ptr.is_empty() {
            self.old_ptr.remove(0);
        }
    }
}
