use crate::event::{Result, Status};
use crate::searching::Match;
use crate::searching::Searcher;
use crate::{Document, Loc};

/// State of a word
pub enum WordState {
    AtStart(usize),
    AtEnd(usize),
    InCenter(usize),
    Out,
}

impl Document {
    /// Find the word boundaries
    #[must_use]
    pub fn word_boundaries(&self, line: &str) -> Vec<(usize, usize)> {
        let re = r"(\s{2,}|[A-Za-z0-9_]+|\.)";
        let mut searcher = Searcher::new(re);
        let starts: Vec<Match> = searcher.lfinds_raw(line);
        let mut ends: Vec<Match> = starts.clone();
        ends.iter_mut()
            .for_each(|m| m.loc.x += m.text.chars().count());
        let starts: Vec<usize> = starts.iter().map(|m| m.loc.x).collect();
        let ends: Vec<usize> = ends.iter().map(|m| m.loc.x).collect();
        starts.into_iter().zip(ends).collect()
    }

    /// Find the current state of the cursor in relation to words
    #[must_use]
    pub fn cursor_word_state(&self, line: &str, words: &[(usize, usize)], x: usize) -> WordState {
        let byte_x = Searcher::char_to_raw(x, line);
        let in_word = words
            .iter()
            .position(|(start, end)| *start <= byte_x && byte_x <= *end);
        if let Some(idx) = in_word {
            let (word_start, word_end) = words[idx];
            if byte_x == word_end {
                WordState::AtEnd(idx)
            } else if byte_x == word_start {
                WordState::AtStart(idx)
            } else {
                WordState::InCenter(idx)
            }
        } else {
            WordState::Out
        }
    }

    /// Find the index of the next word
    #[must_use]
    pub fn prev_word_close(&self, from: Loc) -> usize {
        let Loc { x, y } = from;
        let line = self.line(y).unwrap_or_default();
        let words = self.word_boundaries(&line);
        let state = self.cursor_word_state(&line, &words, x);
        match state {
            // Go to start of line if at beginning
            WordState::AtEnd(0) | WordState::InCenter(0) | WordState::AtStart(0) => 0,
            // Cursor is at the middle / end of a word, move to previous end
            WordState::AtEnd(idx) | WordState::InCenter(idx) => {
                Searcher::raw_to_char(words[idx.saturating_sub(1)].1, &line)
            }
            WordState::AtStart(idx) => Searcher::raw_to_char(words[idx.saturating_sub(1)].0, &line),
            WordState::Out => {
                // Cursor is not touching any words, find previous end
                let mut shift_back = x;
                while let WordState::Out = self.cursor_word_state(&line, &words, shift_back) {
                    shift_back = shift_back.saturating_sub(1);
                    if shift_back == 0 {
                        break;
                    }
                }
                match self.cursor_word_state(&line, &words, shift_back) {
                    WordState::AtEnd(idx) => Searcher::raw_to_char(words[idx].0, &line),
                    _ => 0,
                }
            }
        }
    }

    /// Find the index of the next word
    #[must_use]
    pub fn prev_word_index(&self, from: Loc) -> usize {
        let Loc { x, y } = from;
        let line = self.line(y).unwrap_or_default();
        let words = self.word_boundaries(&line);
        let state = self.cursor_word_state(&line, &words, x);
        match state {
            // Go to start of line if at beginning
            WordState::AtEnd(0) | WordState::InCenter(0) | WordState::AtStart(0) => 0,
            // Cursor is at the middle / end of a word, move to previous end
            WordState::AtEnd(idx) | WordState::InCenter(idx) => {
                Searcher::raw_to_char(words[idx.saturating_sub(1)].1, &line)
            }
            WordState::AtStart(idx) => Searcher::raw_to_char(words[idx.saturating_sub(1)].0, &line),
            WordState::Out => {
                // Cursor is not touching any words, find previous end
                let mut shift_back = x;
                while let WordState::Out = self.cursor_word_state(&line, &words, shift_back) {
                    shift_back = shift_back.saturating_sub(1);
                    if shift_back == 0 {
                        break;
                    }
                }
                match self.cursor_word_state(&line, &words, shift_back) {
                    WordState::AtEnd(idx) => Searcher::raw_to_char(words[idx].1, &line),
                    _ => 0,
                }
            }
        }
    }

    /// Moves to the previous word in the document
    pub fn move_prev_word(&mut self) -> Status {
        let Loc { x, y } = self.char_loc();
        // Handle case where we're at the beginning of the line
        if x == 0 && y != 0 {
            return Status::StartOfLine;
        }
        // Work out where to move to
        let new_x = self.prev_word_index(self.char_loc());
        // Perform the move
        self.move_to_x(new_x);
        // Clean up
        self.old_cursor = self.loc().x;
        Status::None
    }

    /// Find the index of the next word
    #[must_use]
    pub fn next_word_close(&self, from: Loc) -> usize {
        let Loc { x, y } = from;
        let line = self.line(y).unwrap_or_default();
        let words = self.word_boundaries(&line);
        let state = self.cursor_word_state(&line, &words, x);
        match state {
            // Cursor is at the middle / end of a word, move to next end
            WordState::AtEnd(idx) | WordState::InCenter(idx) => {
                if let Some(word) = words.get(idx) {
                    Searcher::raw_to_char(word.1, &line)
                } else {
                    // No next word exists, just go to end of line
                    line.chars().count()
                }
            }
            WordState::AtStart(idx) => {
                // Cursor is at the start of a word, move to next start
                if let Some(word) = words.get(idx) {
                    Searcher::raw_to_char(word.0, &line)
                } else {
                    // No next word exists, just go to end of line
                    line.chars().count()
                }
            }
            WordState::Out => {
                // Cursor is not touching any words, find next start
                let mut shift_forward = x;
                while let WordState::Out = self.cursor_word_state(&line, &words, shift_forward) {
                    shift_forward += 1;
                    if shift_forward >= line.chars().count() {
                        break;
                    }
                }
                match self.cursor_word_state(&line, &words, shift_forward) {
                    WordState::AtStart(idx) => Searcher::raw_to_char(words[idx].0, &line),
                    _ => line.chars().count(),
                }
            }
        }
    }

    /// Find the index of the next word
    #[must_use]
    pub fn next_word_index(&self, from: Loc) -> usize {
        let Loc { x, y } = from;
        let line = self.line(y).unwrap_or_default();
        let words = self.word_boundaries(&line);
        let state = self.cursor_word_state(&line, &words, x);
        match state {
            // Cursor is at the middle / end of a word, move to next end
            WordState::AtEnd(idx) | WordState::InCenter(idx) => {
                if let Some(word) = words.get(idx + 1) {
                    Searcher::raw_to_char(word.1, &line)
                } else {
                    // No next word exists, just go to end of line
                    line.chars().count()
                }
            }
            WordState::AtStart(idx) => {
                // Cursor is at the start of a word, move to next start
                if let Some(word) = words.get(idx + 1) {
                    Searcher::raw_to_char(word.0, &line)
                } else {
                    // No next word exists, just go to end of line
                    line.chars().count()
                }
            }
            WordState::Out => {
                // Cursor is not touching any words, find next start
                let mut shift_forward = x;
                while let WordState::Out = self.cursor_word_state(&line, &words, shift_forward) {
                    shift_forward += 1;
                    if shift_forward >= line.chars().count() {
                        break;
                    }
                }
                match self.cursor_word_state(&line, &words, shift_forward) {
                    WordState::AtStart(idx) => Searcher::raw_to_char(words[idx].0, &line),
                    _ => line.chars().count(),
                }
            }
        }
    }

    /// Moves to the next word in the document
    pub fn move_next_word(&mut self) -> Status {
        let Loc { x, y } = self.char_loc();
        let line = self.line(y).unwrap_or_default();
        // Handle case where we're at the end of the line
        if x == line.chars().count() && y != self.len_lines() {
            return Status::EndOfLine;
        }
        // Work out where to move to
        let new_x = self.next_word_index(self.char_loc());
        // Perform the move
        self.move_to_x(new_x);
        // Clean up
        self.old_cursor = self.loc().x;
        Status::None
    }

    /// Function to delete a word at a certain location
    /// # Errors
    /// Errors if out of range
    pub fn delete_word(&mut self) -> Result<()> {
        let Loc { x, y } = self.char_loc();
        let line = self.line(y).unwrap_or_default();
        let words = self.word_boundaries(&line);
        let state = self.cursor_word_state(&line, &words, x);
        let delete_upto = match state {
            WordState::InCenter(idx) | WordState::AtEnd(idx) => {
                // Delete back to start of this word
                Searcher::raw_to_char(words[idx].0, &line)
            }
            WordState::AtStart(0) => 0,
            WordState::AtStart(idx) => {
                // Delete back to start of the previous word
                Searcher::raw_to_char(words[idx.saturating_sub(1)].0, &line)
            }
            WordState::Out => {
                // Delete back to the end of the previous word
                let mut shift_back = x;
                while let WordState::Out = self.cursor_word_state(&line, &words, shift_back) {
                    shift_back = shift_back.saturating_sub(1);
                    if shift_back == 0 {
                        break;
                    }
                }
                let char = line.chars().nth(shift_back);
                let state = self.cursor_word_state(&line, &words, shift_back);
                match (char, state) {
                    // Shift to start of previous word if there is a space
                    (Some(' '), WordState::AtEnd(idx)) => {
                        Searcher::raw_to_char(words[idx].0, &line)
                    }
                    // Shift to end of previous word if there is not a space
                    (_, WordState::AtEnd(idx)) => Searcher::raw_to_char(words[idx].1, &line),
                    _ => 0,
                }
            }
        };
        self.delete(delete_upto..=x, y)
    }

    /// Select a word at a location
    pub fn select_word_at(&mut self, loc: &Loc) {
        let y = loc.y;
        let x = self.character_idx(loc);
        let re = format!("(\t| {{{}}}|^|\\W| )", self.tab_width);
        let start = if let Some(mut mtch) = self.prev_match(&re) {
            let len = mtch.text.chars().count();
            let same = mtch.loc.x + len == x;
            if !same {
                mtch.loc.x += len;
            }
            self.move_to(&mtch.loc);
            if same && self.loc().x != 0 {
                self.move_prev_word();
            }
            mtch.loc.x
        } else {
            0
        };
        let re = format!("(\t| {{{}}}|\\W|$|^ +| )", self.tab_width);
        let end = if let Some(mtch) = self.next_match(&re, 0) {
            mtch.loc.x
        } else {
            self.line(y).unwrap_or_default().chars().count()
        };
        self.move_to(&Loc { x: start, y });
        self.select_to(&Loc { x: end, y });
        self.old_cursor = self.loc().x;
    }
}
