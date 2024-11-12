/// searching.rs - utilities to assist with searching a document
use crate::regex;
use crate::utils::Loc;
use regex::Regex;

/// Stores information about a match in a document
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Match {
    pub loc: Loc,
    pub text: String,
}

/// Struct to abstract searching
pub struct Searcher {
    pub re: Regex,
}

impl Searcher {
    /// Create a new searcher
    #[must_use]
    pub fn new(re: &str) -> Self {
        Self { re: regex!(re) }
    }

    /// Find the next match, starting from the left hand side of the string
    pub fn lfind(&mut self, st: &str) -> Option<Match> {
        for cap in self.re.captures_iter(st) {
            if let Some(c) = cap.get(cap.len().saturating_sub(1)) {
                let x = Self::raw_to_char(c.start(), st);
                return Some(Match {
                    loc: Loc::at(x, 0),
                    text: c.as_str().to_string(),
                });
            }
        }
        None
    }

    /// Find the next match, starting from the right hand side of the string
    pub fn rfind(&mut self, st: &str) -> Option<Match> {
        let mut caps: Vec<_> = self.re.captures_iter(st).collect();
        caps.reverse();
        for cap in caps {
            if let Some(c) = cap.get(cap.len().saturating_sub(1)) {
                let x = Self::raw_to_char(c.start(), st);
                return Some(Match {
                    loc: Loc::at(x, 0),
                    text: c.as_str().to_string(),
                });
            }
        }
        None
    }

    /// Finds all the matches to the left
    pub fn lfinds(&mut self, st: &str) -> Vec<Match> {
        let mut result = vec![];
        for cap in self.re.captures_iter(st) {
            if let Some(c) = cap.get(cap.len().saturating_sub(1)) {
                let x = Self::raw_to_char(c.start(), st);
                result.push(Match {
                    loc: Loc::at(x, 0),
                    text: c.as_str().to_string(),
                });
            }
        }
        result
    }

    /// Finds all the matches to the left from a certain point onwards
    pub fn lfinds_raw(&mut self, st: &str) -> Vec<Match> {
        let mut result = vec![];
        for cap in self.re.captures_iter(st) {
            if let Some(c) = cap.get(cap.len().saturating_sub(1)) {
                result.push(Match {
                    loc: Loc::at(c.start(), 0),
                    text: c.as_str().to_string(),
                });
            }
        }
        result
    }

    /// Finds all the matches to the right
    pub fn rfinds(&mut self, st: &str) -> Vec<Match> {
        let mut result = vec![];
        let mut caps: Vec<_> = self.re.captures_iter(st).collect();
        caps.reverse();
        for cap in caps {
            if let Some(c) = cap.get(cap.len().saturating_sub(1)) {
                let x = Self::raw_to_char(c.start(), st);
                result.push(Match {
                    loc: Loc::at(x, 0),
                    text: c.as_str().to_string(),
                });
            }
        }
        result
    }

    /// Converts a raw index into a character index, so that matches are in character indices
    #[must_use]
    pub fn raw_to_char(x: usize, st: &str) -> usize {
        for (acc_char, (acc_byte, _)) in st.char_indices().enumerate() {
            if acc_byte == x {
                return acc_char;
            }
        }
        st.chars().count()
    }

    /// Converts a raw index into a character index, so that matches are in character indices
    #[must_use]
    pub fn char_to_raw(x: usize, st: &str) -> usize {
        st.char_indices().nth(x).map_or(st.len(), |(byte, _)| byte)
    }
}
