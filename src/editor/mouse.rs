use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use kaolinite::Loc;
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
    fn find_mouse_location(&mut self, event: MouseEvent) -> MouseLocation {
        let tab_enabled = self.config.tab_line.borrow().enabled;
        let tab = usize::from(tab_enabled);
        if event.row == 0 && tab_enabled {
            let mut c = event.column + 2;
            for (i, file) in self.files.iter().enumerate() {
                let header_len = self.config.tab_line.borrow().render(file).len() + 1;
                c = c.saturating_sub(u16::try_from(header_len).unwrap_or(u16::MAX));
                if c == 0 {
                    return MouseLocation::Tabs(i);
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
    pub fn handle_mouse_event(&mut self, event: MouseEvent) {
        match event.kind {
            // Single click
            MouseEventKind::Down(MouseButton::Left) => {
                // Determine if there has been a click within 500ms
                if let Some((time, last_event)) = self.last_click {
                    let now = Instant::now();
                    let short_period = now.duration_since(time) <= Duration::from_millis(500);
                    let same_location =
                        last_event.column == event.column && last_event.row == event.row;
                    if short_period && same_location {
                        self.handle_double_click(event);
                        return;
                    }
                }
                match self.find_mouse_location(event) {
                    MouseLocation::File(mut loc) => {
                        loc.x = self.doc_mut().character_idx(&loc);
                        self.doc_mut().move_to(&loc);
                        self.doc_mut().old_cursor = self.doc().loc().x;
                    }
                    MouseLocation::Tabs(i) => {
                        self.ptr = i;
                    }
                    MouseLocation::Out => (),
                }
            }
            MouseEventKind::Down(MouseButton::Right) => {
                // Select the current line
                if let MouseLocation::File(loc) = self.find_mouse_location(event) {
                    self.doc_mut().select_line_at(loc.y);
                }
            }
            // Double click detection
            MouseEventKind::Up(MouseButton::Left) => {
                let now = Instant::now();
                // Register this click as having happened
                self.last_click = Some((now, event));
            }
            // Mouse drag
            MouseEventKind::Drag(MouseButton::Left) => match self.find_mouse_location(event) {
                MouseLocation::File(mut loc) => {
                    loc.x = self.doc_mut().character_idx(&loc);
                    self.doc_mut().select_to(&loc);
                }
                MouseLocation::Tabs(_) | MouseLocation::Out => (),
            },
            MouseEventKind::Drag(MouseButton::Right) => match self.find_mouse_location(event) {
                MouseLocation::File(mut loc) => {
                    loc.x = self.doc_mut().character_idx(&loc);
                    self.doc_mut().select_to_y(loc.y);
                }
                MouseLocation::Tabs(_) | MouseLocation::Out => (),
            },
            // Mouse scroll behaviour
            MouseEventKind::ScrollDown | MouseEventKind::ScrollUp => {
                if let MouseLocation::File(_) = self.find_mouse_location(event) {
                    if event.kind == MouseEventKind::ScrollDown {
                        self.doc_mut().scroll_down();
                    } else {
                        self.doc_mut().scroll_up();
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
        }
    }

    /// Handle a double-click event
    pub fn handle_double_click(&mut self, event: MouseEvent) {
        // Select the current word
        if let MouseLocation::File(loc) = self.find_mouse_location(event) {
            self.doc_mut().select_word_at(&loc);
        }
    }
}
