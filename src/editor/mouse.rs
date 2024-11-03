/// For handling mouse events
use crate::config;
use crate::ui::size;
use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use kaolinite::{utils::width, Loc};
use mlua::Lua;
use std::time::{Duration, Instant};

use super::Editor;

/// Represents where the mouse has clicked / been dragged
enum MouseLocation {
    /// Where the mouse has clicked within a file
    File(Loc),
    /// Where the mouse has clicked on a tab
    Tabs(usize),
    /// Mouse has clicked nothing of importance
    Out,
}

impl Editor {
    /// Finds the position of the mouse within the viewport
    fn find_mouse_location(&mut self, lua: &Lua, event: MouseEvent) -> MouseLocation {
        let tab_enabled = config!(self.config, tab_line).enabled;
        let tab = usize::from(tab_enabled);
        if event.row == 0 && tab_enabled {
            let (tabs, _, offset) = self.get_tab_parts(lua, size().map_or(0, |s| s.w));
            let mut c = event.column + 2;
            for (i, header) in tabs.iter().enumerate() {
                let header_len = width(header, 4) + 1;
                c = c.saturating_sub(u16::try_from(header_len).unwrap_or(u16::MAX));
                if c == 0 {
                    return MouseLocation::Tabs(i + offset);
                }
            }
            MouseLocation::Out
        } else if (event.column as usize) < self.dent() {
            MouseLocation::Out
        } else {
            let offset = self.doc().offset;
            MouseLocation::File(Loc {
                x: (event.column as usize).saturating_sub(self.dent()) + offset.x,
                y: (event.row as usize).saturating_sub(tab) + offset.y,
            })
        }
    }

    /// Handles a mouse event (dragging / clicking)
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
                            self.in_dbl_click = true;
                            self.handle_double_click(lua, event);
                            return;
                        }
                    }
                    match self.find_mouse_location(lua, event) {
                        MouseLocation::File(mut loc) => {
                            loc.x = self.doc_mut().character_idx(&loc);
                            self.doc_mut().move_to(&loc);
                            self.doc_mut().old_cursor = self.doc().loc().x;
                        }
                        MouseLocation::Tabs(i) => {
                            self.ptr = i;
                            self.update_cwd();
                        }
                        MouseLocation::Out => (),
                    }
                }
                MouseEventKind::Down(MouseButton::Right) => {
                    // Select the current line
                    if let MouseLocation::File(loc) = self.find_mouse_location(lua, event) {
                        self.doc_mut().select_line_at(loc.y);
                    }
                }
                // Double click detection
                MouseEventKind::Up(MouseButton::Left) => {
                    self.in_dbl_click = false;
                    let now = Instant::now();
                    // Register this click as having happened
                    self.last_click = Some((now, event));
                }
                // Mouse drag
                MouseEventKind::Drag(MouseButton::Left) => {
                    match self.find_mouse_location(lua, event) {
                        MouseLocation::File(mut loc) => {
                            loc.x = self.doc_mut().character_idx(&loc);
                            if self.in_dbl_click {
                                if loc.x >= self.doc().cursor.selection_end.x {
                                    // Find boundary of next word
                                    let next = self.doc().next_word_close(loc);
                                    self.doc_mut().select_to(&Loc { x: next, y: loc.y });
                                } else {
                                    // Find boundary of previous word
                                    let next = self.doc().prev_word_close(loc);
                                    self.doc_mut().select_to(&Loc { x: next, y: loc.y });
                                }
                            } else {
                                loc.x = self.doc_mut().character_idx(&loc);
                                self.doc_mut().select_to(&loc);
                            }
                        }
                        MouseLocation::Tabs(_) | MouseLocation::Out => (),
                    }
                }
                MouseEventKind::Drag(MouseButton::Right) => {
                    match self.find_mouse_location(lua, event) {
                        MouseLocation::File(mut loc) => {
                            loc.x = self.doc_mut().character_idx(&loc);
                            self.doc_mut().select_to_y(loc.y);
                        }
                        MouseLocation::Tabs(_) | MouseLocation::Out => (),
                    }
                }
                // Mouse scroll behaviour
                MouseEventKind::ScrollDown | MouseEventKind::ScrollUp => {
                    if let MouseLocation::File(_) = self.find_mouse_location(lua, event) {
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
                    if let MouseLocation::File(loc) = self.find_mouse_location(lua, event) {
                        self.doc_mut().new_cursor(loc);
                    }
                }
            }
            _ => (),
        }
    }

    /// Handle a double-click event
    pub fn handle_double_click(&mut self, lua: &Lua, event: MouseEvent) {
        // Select the current word
        if let MouseLocation::File(loc) = self.find_mouse_location(lua, event) {
            self.doc_mut().select_word_at(&loc);
        }
    }
}
