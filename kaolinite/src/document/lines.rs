use crate::event::{Error, Result};
use crate::utils::trim;
use crate::{Document, Loc};

impl Document {
    /// Get the line at a specified index
    #[must_use]
    pub fn line(&self, line: usize) -> Option<String> {
        Some(self.lines.get(line)?.to_string())
    }

    /// Get the line at a specified index and trim it
    #[must_use]
    pub fn line_trim(&self, line: usize, start: usize, length: usize) -> Option<String> {
        let line = self.line(line);
        Some(trim(&line?, start, length, self.tab_width))
    }

    /// Returns the number of lines in the document
    #[must_use]
    pub fn len_lines(&self) -> usize {
        self.file.len_lines().saturating_sub(1) + usize::from(self.info.eol)
    }

    /// Evaluate the line number text for a specific line
    #[must_use]
    pub fn line_number(&self, request: usize) -> String {
        let total = self.len_lines().to_string().len();
        let num = if request + 1 > self.len_lines() {
            "~".to_string()
        } else {
            (request + 1).to_string()
        };
        format!("{}{}", " ".repeat(total.saturating_sub(num.len())), num)
    }

    /// Swap a line upwards
    /// # Errors
    /// When out of bounds
    pub fn swap_line_up(&mut self) -> Result<()> {
        let cursor = self.char_loc();
        let line = self.line(cursor.y).ok_or(Error::OutOfRange)?;
        self.insert_line(cursor.y.saturating_sub(1), line)?;
        self.delete_line(cursor.y + 1)?;
        self.move_to(&Loc {
            x: cursor.x,
            y: cursor.y.saturating_sub(1),
        });
        Ok(())
    }

    /// Swap a line downwards
    /// # Errors
    /// When out of bounds
    pub fn swap_line_down(&mut self) -> Result<()> {
        let cursor = self.char_loc();
        let line = self.line(cursor.y).ok_or(Error::OutOfRange)?;
        self.insert_line(cursor.y + 2, line)?;
        self.delete_line(cursor.y)?;
        self.move_to(&Loc {
            x: cursor.x,
            y: cursor.y + 1,
        });
        Ok(())
    }

    /// Select a line at a location
    pub fn select_line_at(&mut self, y: usize) {
        let len = self.line(y).unwrap_or_default().chars().count();
        self.move_to(&Loc { x: 0, y });
        self.select_to(&Loc { x: len, y });
    }
}
