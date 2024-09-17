/// searching.rs - utilities to assist with searching a document
use crate::regex;
use crate::utils::Loc;
use regex::Regex;

/// Stores information about a match in a document
#[derive(Debug, PartialEq, Eq)]
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

    /// Converts a raw index into a character index, so that matches are in character indices
    #[must_use]
    pub fn raw_to_char(x: usize, st: &str) -> usize {
        let mut raw = 0;
        for (c, ch) in st.chars().enumerate() {
            if raw == x {
                return c;
            }
            raw += ch.len_utf8();
        }
        st.chars().count()
    }
}
