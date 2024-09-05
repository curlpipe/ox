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
        } else if event.column < 3 {
            MouseLocation::Out
        } else {
            MouseLocation::File(Loc { x: (event.column - 3) as usize, y: (event.row - 1) as usize })
        }
    }

    pub fn handle_mouse_event(&mut self, event: MouseEvent) {
        match event.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                match self.find_mouse_location(event) {
                    MouseLocation::File(loc) => {
                        self.doc_mut().goto(&loc);
                    },
                    MouseLocation::Tabs(i) => {
                        self.ptr = i;
                    },
                    MouseLocation::Out => (),
                }
            },
            _ => (),
        }
    }
}