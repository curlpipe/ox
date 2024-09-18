use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use kaolinite::Loc;

use super::Editor;

enum MouseLocation {
    File(Loc),
    Tabs(usize),
    Out,
}

impl Editor {
    fn find_mouse_location(&mut self, event: MouseEvent) -> MouseLocation {
        if event.row == 0 {
            let mut c = event.column + 2;
            for (i, doc) in self.doc.iter().enumerate() {
                let header_len = self.render_document_tab_header(doc).len() + 1;
                c = c.saturating_sub(header_len as u16);
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
                x: event.column as usize - self.dent() + offset.x,
                y: (event.row as usize) - 1 + offset.y,
            })
        }
    }

    pub fn handle_mouse_event(&mut self, event: MouseEvent) {
        match event.kind {
            MouseEventKind::Down(MouseButton::Left) => match self.find_mouse_location(event) {
                MouseLocation::File(mut loc) => {
                    loc.x = self.doc_mut().character_idx(&loc);
                    self.doc_mut().move_to(&loc);
                }
                MouseLocation::Tabs(i) => {
                    self.ptr = i;
                }
                MouseLocation::Out => (),
            },
            MouseEventKind::Drag(MouseButton::Left) => match self.find_mouse_location(event) {
                MouseLocation::File(mut loc) => {
                    loc.x = self.doc_mut().character_idx(&loc);
                    self.doc_mut().select_to(&loc);
                }
                MouseLocation::Tabs(_) | MouseLocation::Out => (),
            },
            MouseEventKind::ScrollDown | MouseEventKind::ScrollUp => {
                match self.find_mouse_location(event) {
                    MouseLocation::File(_) => {
                        if event.kind == MouseEventKind::ScrollDown {
                            self.doc_mut().scroll_down();
                        } else {
                            self.doc_mut().scroll_up();
                        }
                    }
                    _ => (),
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
}
