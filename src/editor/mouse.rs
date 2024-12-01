/// For handling mouse events
use crate::config;
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
            let doc_idx = self.files.get_atom(idx.clone()).unwrap().1;
            let dent = self.dent_for(&idx, doc_idx);
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
        } else {
            MouseLocation::Out
        }
    }

    /// Handles a mouse event (dragging / clicking)
    #[allow(clippy::too_many_lines)]
    pub fn handle_mouse_event(&mut self, lua: &Lua, event: MouseEvent) {
        match event.modifiers {
            KeyModifiers::NONE => match event.kind {
                // Single click
                MouseEventKind::Down(MouseButton::Left) => {
                    // Determine if there has been a click within 500ms
                    if let Some((time, last_event)) = self.last_click {
                        let now = Instant::now();
                        let short_period = now.duration_since(time) <= Duration::from_millis(500);
                        let same_location =
                            last_event.column == event.column && last_event.row == event.row;
                        if short_period && same_location {
                            self.handle_double_click(lua, event);
                            return;
                        }
                    }
                    match self.find_mouse_location(lua, event) {
                        MouseLocation::File(idx, mut loc) => {
                            self.ptr.clone_from(&idx);
                            self.doc_mut().clear_cursors();
                            loc.x = self.doc_mut().character_idx(&loc);
                            self.doc_mut().move_to(&loc);
                            self.doc_mut().old_cursor = self.doc().loc().x;
                        }
                        MouseLocation::Tabs(idx, i) => {
                            self.files.move_to(idx.clone(), i);
                            self.ptr.clone_from(&idx);
                            self.update_cwd();
                        }
                        MouseLocation::Out => (),
                    }
                }
                MouseEventKind::Down(MouseButton::Right) => {
                    // Select the current line
                    if let MouseLocation::File(idx, loc) = self.find_mouse_location(lua, event) {
                        self.ptr.clone_from(&idx);
                        self.doc_mut().select_line_at(loc.y);
                        let line = self.doc().line(loc.y).unwrap_or_default();
                        self.alt_click_state = Some((
                            Loc {
                                x: 0,
                                y: self.doc().loc().y,
                            },
                            Loc {
                                x: line.chars().count(),
                                y: self.doc().loc().y,
                            },
                        ));
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
                            self.ptr.clone_from(&idx);
                            loc.x = self.doc_mut().character_idx(&loc);
                            if let Some((dbl_start, dbl_end)) = self.alt_click_state {
                                if loc.x > self.doc().cursor.selection_end.x {
                                    // Find boundary of next word
                                    let next = self.doc().next_word_close(loc);
                                    self.doc_mut().move_to(&dbl_start);
                                    self.doc_mut().select_to(&Loc { x: next, y: loc.y });
                                } else {
                                    // Find boundary of previous word
                                    let next = self.doc().prev_word_close(loc);
                                    self.doc_mut().move_to(&dbl_end);
                                    self.doc_mut().select_to(&Loc { x: next, y: loc.y });
                                }
                            } else {
                                self.doc_mut().select_to(&loc);
                            }
                        }
                        MouseLocation::Tabs(_, _) | MouseLocation::Out => (),
                    }
                }
                MouseEventKind::Drag(MouseButton::Right) => {
                    match self.find_mouse_location(lua, event) {
                        MouseLocation::File(idx, mut loc) => {
                            self.ptr.clone_from(&idx);
                            loc.x = self.doc_mut().character_idx(&loc);
                            if let Some((line_start, line_end)) = self.alt_click_state {
                                if loc.y > self.doc().cursor.selection_end.y {
                                    let line = self.doc().line(loc.y).unwrap_or_default();
                                    self.doc_mut().move_to(&line_start);
                                    self.doc_mut().select_to(&Loc {
                                        x: line.chars().count(),
                                        y: loc.y,
                                    });
                                } else {
                                    self.doc_mut().move_to(&line_end);
                                    self.doc_mut().select_to(&Loc { x: 0, y: loc.y });
                                }
                            } else {
                                self.doc_mut().select_to(&loc);
                            }
                        }
                        MouseLocation::Tabs(_, _) | MouseLocation::Out => (),
                    }
                }
                // Mouse scroll behaviour
                MouseEventKind::ScrollDown | MouseEventKind::ScrollUp => {
                    if let MouseLocation::File(idx, _) = self.find_mouse_location(lua, event) {
                        self.ptr.clone_from(&idx);
                        let scroll_amount = config!(self.config, terminal).scroll_amount;
                        for _ in 0..scroll_amount {
                            if event.kind == MouseEventKind::ScrollDown {
                                self.doc_mut().scroll_down();
                            } else {
                                self.doc_mut().scroll_up();
                            }
                        }
                    }
                }
                MouseEventKind::ScrollLeft => {
                    self.doc_mut().move_left();
                }
                MouseEventKind::ScrollRight => {
                    self.doc_mut().move_right();
                }
                _ => (),
            },
            // Multi cursor behaviour
            KeyModifiers::CONTROL => {
                if let MouseEventKind::Down(MouseButton::Left) = event.kind {
                    if let MouseLocation::File(idx, loc) = self.find_mouse_location(lua, event) {
                        self.ptr.clone_from(&idx);
                        self.doc_mut().new_cursor(loc);
                        self.doc_mut().commit();
                    }
                }
            }
            _ => (),
        }
    }

    /// Handle a double-click event
    pub fn handle_double_click(&mut self, lua: &Lua, event: MouseEvent) {
        // Select the current word
        if let MouseLocation::File(idx, loc) = self.find_mouse_location(lua, event) {
            self.ptr.clone_from(&idx);
            self.doc_mut().select_word_at(&loc);
            let mut selection = self.doc().cursor.selection_end;
            let mut cursor = self.doc().cursor.loc;
            selection.x = self.doc().character_idx(&selection);
            cursor.x = self.doc().character_idx(&cursor);
            self.alt_click_state = Some((selection, cursor));
        }
    }
}
