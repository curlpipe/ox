use crate::utils::{width, Loc};
/// map.rs - provides an easy interface to manage characters with large widths
use std::collections::HashMap;
use unicode_width::UnicodeWidthChar;

/// This is a type for making a note of the location of different characters
type CharHashMap = HashMap<usize, Vec<(usize, usize)>>;

/// Keeps notes of specific characters within a document for the purposes of double width and
/// tab characters, which have display widths different to that of their character width
#[derive(Default, Clone, PartialEq, Eq, Debug)]
pub struct CharMap {
    pub map: CharHashMap,
}

impl CharMap {
    /// Create a new character map
    #[must_use]
    pub fn new(map: CharHashMap) -> Self {
        Self { map }
    }

    /// Add a value to a line in the map
    pub fn add(&mut self, idx: usize, val: (usize, usize)) {
        if let Some(map) = self.map.get_mut(&idx) {
            map.push(val);
        } else {
            self.map.insert(idx, vec![val]);
        }
    }

    /// Add a line to the map
    pub fn insert(&mut self, idx: usize, slice: Vec<(usize, usize)>) {
        if !slice.is_empty() {
            self.map.insert(idx, slice);
        }
    }

    /// Delete a line from the map
    pub fn delete(&mut self, idx: usize) {
        self.map.remove(&idx);
    }

    /// Get a line from the map
    #[must_use]
    pub fn get(&self, idx: usize) -> Option<&Vec<(usize, usize)>> {
        self.map.get(&idx)
    }

    /// Verify whether this line is in the map
    #[must_use]
    pub fn contains(&self, idx: usize) -> bool {
        self.map.contains_key(&idx)
    }

    /// Add a slice to the map
    pub fn splice(&mut self, loc: &Loc, start: usize, slice: Vec<(usize, usize)>) {
        if let Some(map) = self.map.get_mut(&loc.y) {
            map.splice(start..start, slice);
        } else if !slice.is_empty() {
            self.map.insert(loc.y, slice);
        }
    }

    /// Shift entries up in the character map
    #[allow(clippy::missing_panics_doc)]
    pub fn shift_insertion(&mut self, loc: &Loc, st: &str, tab_width: usize) -> usize {
        if !self.map.contains_key(&loc.y) {
            return 0;
        }
        // Gather context
        let char_shift = st.chars().count();
        let disp_shift = width(st, tab_width);
        // Find point of insertion
        let start = self.count(loc, false).unwrap();
        // Shift subsequent characters up
        let line_map = self.map.get_mut(&loc.y).unwrap();
        for (display, ch) in line_map.iter_mut().skip(start) {
            *display += disp_shift;
            *ch += char_shift;
        }
        start
    }

    /// Shift entries down in the character map
    #[allow(clippy::missing_panics_doc)]
    pub fn shift_deletion(&mut self, loc: &Loc, x: (usize, usize), st: &str, tab_width: usize) {
        if !self.map.contains_key(&loc.y) {
            return;
        }
        // Gather context
        let char_shift = st.chars().count();
        let disp_shift = width(st, tab_width);
        let (start, end) = x;
        let Loc { x: line_start, y } = loc;
        // Work out indices of deletion
        let start_map = self
            .count(
                &Loc {
                    x: start - line_start,
                    y: *y,
                },
                false,
            )
            .unwrap();
        let map_count = self
            .count(
                &Loc {
                    x: end - line_start,
                    y: *y,
                },
                false,
            )
            .unwrap();
        let line_map = self.map.get_mut(y).unwrap();
        // Update subsequent map characters
        for (display, ch) in line_map.iter_mut().skip(map_count) {
            *display -= disp_shift;
            *ch -= char_shift;
        }
        // Remove entries for the range
        line_map.drain(start_map..map_count);
        // Remove entry if no map characters exist anymore
        if line_map.is_empty() {
            self.map.remove(y);
        }
    }

    /// Shift lines in the character map up one
    #[allow(clippy::missing_panics_doc)]
    pub fn shift_up(&mut self, loc: usize) {
        let mut keys: Vec<usize> = self.map.keys().copied().collect();
        keys.sort_unstable();
        for k in keys {
            if k >= loc {
                let v = self.map.remove(&k).unwrap();
                self.map.insert(k - 1, v);
            }
        }
    }

    /// Shift lines in the character map down one
    #[allow(clippy::missing_panics_doc)]
    pub fn shift_down(&mut self, loc: usize) {
        let mut keys: Vec<usize> = self.map.keys().copied().collect();
        keys.sort_unstable();
        keys.reverse();
        for k in keys {
            if k >= loc {
                let v = self.map.remove(&k).unwrap();
                self.map.insert(k + 1, v);
            }
        }
    }

    /// Count the number of characters before an index, useful for conversion of indices
    #[must_use]
    pub fn count(&self, loc: &Loc, display: bool) -> Option<usize> {
        let mut ctr = 0;
        for i in self.get(loc.y)? {
            let i = if display { i.0 } else { i.1 };
            if i >= loc.x {
                break;
            }
            ctr += 1;
        }
        Some(ctr)
    }
}

/// Vector that takes two usize values
pub type DblUsize = Vec<(usize, usize)>;

/// Work out the map contents from a string
#[must_use]
pub fn form_map(st: &str, tab_width: usize) -> (DblUsize, DblUsize) {
    let mut dbl = vec![];
    let mut tab = vec![];
    let mut idx = 0;
    for (char_idx, ch) in st.chars().enumerate() {
        if ch == '\t' {
            tab.push((idx, char_idx));
            idx += tab_width;
        } else if ch.width().unwrap_or(1) == 1 {
            idx += 1;
        } else {
            dbl.push((idx, char_idx));
            idx += 2;
        }
    }
    (dbl, tab)
}
