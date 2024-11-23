/// Tools for placing all information about open files into one place
use crate::editor::{get_absolute_path, FileType};
use kaolinite::Document;
use kaolinite::Size;
use std::ops::Range;
use synoptic::Highlighter;

pub type Span = Vec<(Vec<usize>, Range<usize>, Range<usize>)>;

// File split structure
#[derive(Debug, Clone)]
pub enum FileLayout {
    /// Side-by-side documents (with proportions)
    SideBySide(Vec<(FileLayout, f64)>),
    /// Top-to-bottom documents (with proportions)
    TopToBottom(Vec<(FileLayout, f64)>),
    /// Single file container (and pointer for tabs)
    Atom(Vec<FileContainer>, usize),
    /// Placeholder for an empty file split
    None,
}

impl Default for FileLayout {
    fn default() -> Self {
        Self::None
    }
}

impl FileLayout {
    /// Will return file containers and what span of columns and rows they take up
    /// In the format of (container, rows, columns)
    pub fn span(&self, idx: Vec<usize>, size: Size) -> Span {
        match self {
            Self::None => vec![],
            Self::Atom(containers, ptr) => vec![(idx, 0..size.h, 0..size.w)],
            Self::SideBySide(layouts) => {
                let mut result = vec![];
                let mut at = 0;
                for (c, (layout, props)) in layouts.iter().enumerate() {
                    let mut subidx = idx.clone();
                    subidx.push(c);
                    let this_size = Size {
                        w: at + (size.w as f64 * props) as usize,
                        h: size.h,
                    };
                    for mut sub in layout.span(subidx, this_size) {
                        let mut end = sub.2.end;
                        if c == layouts.len().saturating_sub(1) {
                            end += size.w.saturating_sub(sub.2.end)
                        } else {
                            end -= 1;
                        }
                        sub.2 = at..end;
                        result.push(sub);
                    }
                    at = this_size.w;
                }
                result
            }
            Self::TopToBottom(layouts) => {
                let mut result = vec![];
                let mut at = 0;
                for (c, (layout, props)) in layouts.iter().enumerate() {
                    let mut subidx = idx.clone();
                    subidx.push(c);
                    let this_size = Size {
                        w: size.w,
                        h: at + (size.h as f64 * props) as usize,
                    };
                    for mut sub in layout.span(subidx, this_size) {
                        let mut end = sub.1.end;
                        if c == layouts.len().saturating_sub(1) {
                            end += size.h.saturating_sub(sub.1.end)
                        } else {
                            end -= 1;
                            result.push((vec![42].repeat(100), sub.1.clone(), sub.2.clone()));
                        }
                        sub.1 = at..end;
                        result.push(sub);
                    }
                    at = this_size.h;
                }
                result
            }
        }
    }

    /// Work out which file containers to render where on a particular line and in what order
    pub fn line(y: usize, spans: &Span) -> Span {
        let mut appropriate: Vec<_> = spans
            .iter()
            .filter_map(|(ptr, rows, columns)| {
                if rows.contains(&y) {
                    Some((ptr.clone(), rows.clone(), columns.clone()))
                } else {
                    None
                }
            })
            .collect();
        appropriate.sort_by(|a, b| a.1.start.cmp(&b.1.start));
        appropriate
    }

    /// Work out how many files are currently open
    pub fn len(&self) -> usize {
        match self {
            Self::None => 0,
            Self::Atom(containers, _) => containers.len(),
            Self::SideBySide(layouts) => layouts.iter().map(|(layout, _)| layout.len()).sum(),
            Self::TopToBottom(layouts) => layouts.iter().map(|(layout, _)| layout.len()).sum(),
        }
    }

    /// Find a file container location from it's path
    pub fn find(&self, idx: Vec<usize>, path: &str) -> Option<(Vec<usize>, usize)> {
        match self {
            Self::None => None,
            Self::Atom(containers, _) => {
                // Scan this atom for any documents
                for (ptr, container) in containers.iter().enumerate() {
                    let file_path = container.doc.file_name.as_ref();
                    let file_path = file_path.map(|f| get_absolute_path(f).unwrap_or_default());
                    if file_path == Some(path.to_string()) {
                        return Some((idx, ptr));
                    }
                }
                None
            }
            Self::SideBySide(layouts) => {
                // Recursively scan
                for (nth, (layout, _)) in layouts.iter().enumerate() {
                    let mut this_idx = idx.clone();
                    this_idx.push(nth);
                    let result = layout.find(this_idx, path.clone());
                    if result.is_some() {
                        return result;
                    }
                }
                None
            }
            Self::TopToBottom(layouts) => {
                // Recursively scan
                for (nth, (layout, _)) in layouts.iter().enumerate() {
                    let mut this_idx = idx.clone();
                    this_idx.push(nth);
                    let result = layout.find(this_idx, path.clone());
                    if result.is_some() {
                        return result;
                    }
                }
                None
            }
        }
    }

    /// Get the FileLayout at a certain index
    pub fn get_raw(&self, mut idx: Vec<usize>) -> Option<&FileLayout> {
        match self {
            Self::None => Some(self),
            Self::Atom(containers, ptr) => Some(self),
            Self::SideBySide(layouts) => {
                if idx.get(0).is_some() {
                    let subidx = idx.remove(0);
                    layouts[subidx].0.get_raw(idx)
                } else {
                    Some(self)
                }
            }
            Self::TopToBottom(layouts) => {
                if idx.get(0).is_some() {
                    let subidx = idx.remove(0);
                    layouts[subidx].0.get_raw(idx)
                } else {
                    Some(self)
                }
            }
        }
    }

    /// Get the FileLayout at a certain index
    pub fn set(&mut self, mut idx: Vec<usize>, fl: FileLayout) {
        match self {
            Self::None => *self = fl,
            Self::Atom(_, _) => *self = fl,
            Self::SideBySide(layouts) => {
                if idx.get(0).is_some() {
                    let subidx = idx.remove(0);
                    layouts[subidx].0.set(idx, fl)
                } else {
                    *self = fl;
                }
            }
            Self::TopToBottom(layouts) => {
                if idx.get(0).is_some() {
                    let subidx = idx.remove(0);
                    layouts[subidx].0.set(idx, fl)
                } else {
                    *self = fl;
                }
            }
        }
    }

    /// Given an index, find the file containers in the tree
    pub fn get_atom(&self, mut idx: Vec<usize>) -> Option<(Vec<&FileContainer>, usize)> {
        match self {
            Self::None => None,
            Self::Atom(containers, ptr) => Some((containers.iter().collect(), *ptr)),
            Self::SideBySide(layouts) => {
                let subidx = idx.remove(0);
                layouts[subidx].0.get_atom(idx)
            }
            Self::TopToBottom(layouts) => {
                let subidx = idx.remove(0);
                layouts[subidx].0.get_atom(idx)
            }
        }
    }

    /// Given an index, find the file containers in the tree
    pub fn get_atom_mut(
        &mut self,
        mut idx: Vec<usize>,
    ) -> Option<(&mut Vec<FileContainer>, &mut usize)> {
        match self {
            Self::None => None,
            Self::Atom(ref mut containers, ref mut ptr) => Some((containers, ptr)),
            Self::SideBySide(layouts) => {
                let subidx = idx.remove(0);
                layouts[subidx].0.get_atom_mut(idx)
            }
            Self::TopToBottom(layouts) => {
                let subidx = idx.remove(0);
                layouts[subidx].0.get_atom_mut(idx)
            }
        }
    }

    /// Given an index, find the file container in the tree
    pub fn get_all(&self, idx: Vec<usize>) -> Vec<&FileContainer> {
        self.get_atom(idx).map_or(vec![], |(fcs, _)| fcs)
    }

    /// Given an index, find the file container in the tree
    pub fn get_all_mut(&mut self, idx: Vec<usize>) -> Vec<&mut FileContainer> {
        self.get_atom_mut(idx)
            .map_or(vec![], |(fcs, _)| fcs.iter_mut().collect())
    }

    /// Given an index, find the file container in the tree
    pub fn get(&self, idx: Vec<usize>) -> Option<&FileContainer> {
        let (fcs, ptr) = self.get_atom(idx)?;
        Some(fcs.get(ptr)?)
    }

    /// Given an index, find the file container in the tree
    pub fn get_mut(&mut self, idx: Vec<usize>) -> Option<&mut FileContainer> {
        let (fcs, ptr) = self.get_atom_mut(idx)?;
        Some(fcs.get_mut(*ptr)?)
    }

    /// In the currently active atom, move to a different document
    pub fn move_to(&mut self, mut idx: Vec<usize>, ptr: usize) {
        match self {
            Self::None => (),
            Self::Atom(_, ref mut old_ptr) => *old_ptr = ptr,
            Self::SideBySide(layouts) => {
                let subidx = idx.remove(0);
                layouts[subidx].0.move_to(idx, ptr)
            }
            Self::TopToBottom(layouts) => {
                let subidx = idx.remove(0);
                layouts[subidx].0.move_to(idx, ptr)
            }
        }
    }

    /// Open a split above the current pointer
    pub fn open_top(&mut self, at: Vec<usize>, fl: FileLayout) {
        if let Some(old_fl) = self.get_raw(at.clone()) {
            let new_fl = match old_fl {
                Self::None => fl,
                Self::Atom(containers, ptr) => {
                    Self::TopToBottom(vec![(fl, 0.5), (self.clone(), 0.5)])
                }
                Self::SideBySide(layouts) => Self::TopToBottom(vec![(fl, 0.5), (self.clone(), 0.5)]),
                Self::TopToBottom(layouts) => {
                    Self::TopToBottom(vec![(fl, 0.5), (self.clone(), 0.5)])
                }
            };
            self.set(at, new_fl);
        }
    }

    /// Open a split below the current pointer
    pub fn open_bottom(&mut self, at: Vec<usize>, fl: FileLayout) {
        if let Some(old_fl) = self.get_raw(at.clone()) {
            let new_fl = match old_fl {
                Self::None => fl,
                Self::Atom(containers, ptr) => {
                    Self::TopToBottom(vec![(self.clone(), 0.5), (fl, 0.5)])
                }
                Self::SideBySide(layouts) => Self::TopToBottom(vec![(self.clone(), 0.5), (fl, 0.5)]),
                Self::TopToBottom(layouts) => {
                    Self::TopToBottom(vec![(self.clone(), 0.5), (fl, 0.5)])
                }
            };
            self.set(at, new_fl);
        }
    }

    /// Open a split to the left of the current pointer
    pub fn open_left(&mut self, at: Vec<usize>, fl: FileLayout) {
        if let Some(old_fl) = self.get_raw(at.clone()) {
            let new_fl = match old_fl {
                Self::None => fl,
                Self::Atom(containers, ptr) => {
                    Self::SideBySide(vec![(fl, 0.5), (self.clone(), 0.5)])
                }
                Self::SideBySide(layouts) => Self::SideBySide(vec![(fl, 0.5), (self.clone(), 0.5)]),
                Self::TopToBottom(layouts) => {
                    Self::SideBySide(vec![(fl, 0.5), (self.clone(), 0.5)])
                }
            };
            self.set(at, new_fl);
        }
    }

    /// Open a split to the right of the current pointer
    pub fn open_right(&mut self, at: Vec<usize>, fl: FileLayout) {
        if let Some(old_fl) = self.get_raw(at.clone()) {
            let new_fl = match old_fl {
                Self::None => fl,
                Self::Atom(containers, ptr) => {
                    Self::SideBySide(vec![(self.clone(), 0.5), (fl, 0.5)])
                }
                Self::SideBySide(layouts) => Self::SideBySide(vec![(self.clone(), 0.5), (fl, 0.5)]),
                Self::TopToBottom(layouts) => {
                    Self::SideBySide(vec![(self.clone(), 0.5), (fl, 0.5)])
                }
            };
            self.set(at, new_fl);
        }
    }
}

/// Container for a file
#[derive(Debug, Clone)]
pub struct FileContainer {
    /// Document (stores kaolinite information)
    pub doc: Document,
    /// Highlighter (stores synoptic information)
    pub highlighter: Highlighter,
    /// File type (stores which file type this file is)
    pub file_type: Option<FileType>,
}

impl Default for FileContainer {
    fn default() -> Self {
        Self {
            doc: Document::new(Size { w: 10, h: 10 }),
            highlighter: Highlighter::new(4),
            file_type: None,
        }
    }
}
