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
                        w: (size.w as f64 * props) as usize,
                        h: size.h,
                    };
                    for mut sub in layout.span(subidx, this_size) {
                        // Shift this range up to it's correct location
                        sub.2.start += at;
                        sub.2.end += at;
                        if c != layouts.len().saturating_sub(1) {
                            sub.2.end -= 1;
                        }
                        result.push(sub);
                    }
                    at += this_size.w;
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
                        h: (size.h as f64 * props) as usize,
                    };
                    for mut sub in layout.span(subidx, this_size) {
                        sub.1.start += at;
                        sub.1.end += at;
                        if c != layouts.len().saturating_sub(1) {
                            sub.1.end -= 1;
                        }
                        result.push(sub.clone());
                    }
                    at += this_size.h;
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
        appropriate.sort_by(|a, b| a.2.start.cmp(&b.2.start));
        appropriate
    }

    /// Fixes span underflow (where nodes are shorter than desired due to division errors)
    pub fn fix_underflow(mut span: Span, desired: Size) -> Span {
        // FIX FOR WIDTH
        // Go through each line in a span
        for y in 0..desired.h {
            let line = Self::line(y, &span);
            if let Some((idx, rows, cols)) = line.get(line.len().saturating_sub(1)) {
                // If this line has the width shorter than desired (and is the first of it's kind)
                if cols.end < desired.w && y == rows.start {
                    if let Some((_, _, ref mut col_span)) = span
                        .iter_mut()
                        .find(|(checking_idx, _, _)| checking_idx == idx)
                    {
                        // Take the idx of the last node and push it up to ensure it fits
                        let shift_by = desired.w.saturating_sub(cols.end);
                        col_span.end += shift_by;
                    }
                }
            }
        }

        // FIX FOR HEIGHT
        // Work out:
        // - The number of vacant line entries at the end of the desired height
        // - The last non-empty entry in the line registry
        let mut last_active_line = 0;
        let mut empty_last_lines = 0;
        for y in 0..desired.h {
            let line = Self::line(y, &span);
            if line.is_empty() {
                empty_last_lines += 1;
            } else {
                last_active_line = y;
                empty_last_lines = 0;
            }
        }
        let last_panes = Self::line(last_active_line, &span)
            .into_iter()
            .map(|(idx, cols, rows)| idx)
            .collect::<Vec<_>>();
        // For each pane on the last non-empty line:
        for pane_idx in last_panes {
            if let Some((_, ref mut row_span, _)) = span
                .iter_mut()
                .find(|(checking_idx, _, _)| *checking_idx == pane_idx)
            {
                // Set the end of the rows range to the desired height (in effect expanding them downwards)
                let shift_by = desired.h.saturating_sub(1 + last_active_line);
                row_span.end += shift_by;
            }
        }

        // Return the modified result
        span
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
                    layouts.get(subidx)?.0.get_raw(idx)
                } else {
                    Some(self)
                }
            }
            Self::TopToBottom(layouts) => {
                if idx.get(0).is_some() {
                    let subidx = idx.remove(0);
                    layouts.get(subidx)?.0.get_raw(idx)
                } else {
                    Some(self)
                }
            }
        }
    }

    /// Get the FileLayout at a certain index (mutable)
    pub fn get_raw_mut(&mut self, mut idx: Vec<usize>) -> Option<&mut FileLayout> {
        if idx.get(0).is_none() {
            Some(self)
        } else {
            match self {
                Self::None => Some(self),
                Self::Atom(containers, ptr) => Some(self),
                Self::SideBySide(layouts) => {
                    let subidx = idx.remove(0);
                    layouts.get_mut(subidx)?.0.get_raw_mut(idx)
                }
                Self::TopToBottom(layouts) => {
                    let subidx = idx.remove(0);
                    layouts.get_mut(subidx)?.0.get_raw_mut(idx)
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
                layouts.get(subidx)?.0.get_atom(idx)
            }
            Self::TopToBottom(layouts) => {
                let subidx = idx.remove(0);
                layouts.get(subidx)?.0.get_atom(idx)
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
                layouts.get_mut(subidx)?.0.get_atom_mut(idx)
            }
            Self::TopToBottom(layouts) => {
                let subidx = idx.remove(0);
                layouts.get_mut(subidx)?.0.get_atom_mut(idx)
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

    /// Remove any empty atoms
    pub fn clean_up(&mut self) {
        // Continue checking for obselete nodes until none are remaining
        while let Some(empty_idx) = self.empty_atoms(vec![]) {
            // Delete the empty node
            self.remove(empty_idx.clone());
        }
    }

    /// Remove a certain index from this tree
    pub fn remove(&mut self, at: Vec<usize>) {
        // Get parent of the node we wish to delete
        let mut at_parent = at.clone();
        if let Some(within_parent) = at_parent.pop() {
            // Determine behaviour based on parent
            if let Some(parent) = self.get_raw_mut(at_parent) {
                match parent {
                    Self::None | Self::Atom(_, _) => unreachable!(),
                    Self::SideBySide(layouts) | Self::TopToBottom(layouts) => {
                        // Get the proportion of what we're removing
                        let removed_prop = layouts[within_parent].1;
                        // Remove from the parent
                        layouts.remove(within_parent);
                        // Redistribute proportions
                        let redistributed = removed_prop / layouts.len() as f64;
                        for (_, prop) in layouts.iter_mut() {
                            *prop += redistributed;
                        }
                    }
                }
            }
        } else {
            // This is the root node of the entire tree!
            // In this case, we just set the whole thing to FileLayout::None
            self.set(at, FileLayout::None);
        }
    }

    /// Traverse the tree and return a list of indices to empty atoms
    pub fn empty_atoms(&self, at: Vec<usize>) -> Option<Vec<usize>> {
        match self {
            Self::None => None,
            Self::Atom(fcs, _) => {
                if fcs.is_empty() {
                    Some(at)
                } else {
                    None
                }
            }
            Self::SideBySide(layouts) | Self::TopToBottom(layouts) => {
                if layouts.is_empty() {
                    Some(at)
                } else {
                    for (c, layout) in layouts.iter().enumerate() {
                        let mut idx = at.clone();
                        idx.push(c);
                        if let Some(result) = layout.0.empty_atoms(idx) {
                            return Some(result);
                        }
                    }
                    None
                }
            }
        }
    }

    /// Find a new pointer position when something is removed
    pub fn new_pointer_position(&self, old: Vec<usize>) -> Vec<usize> {
        // Zoom out until a sidebyside or toptobottom is found
        let mut copy = old.clone();
        while let Some(Self::None | Self::Atom(_, _)) | None = self.get_raw(copy.clone()) {
            copy.pop();
            if copy.is_empty() {
                break;
            }
        }
        // Zoom in to find a new cursor position
        while let Some(FileLayout::TopToBottom(_) | FileLayout::SideBySide(_)) =
            self.get_raw(copy.clone())
        {
            copy.push(0);
        }
        copy
    }

    /// Open a split above the current pointer
    pub fn open_up(&mut self, at: Vec<usize>, fl: FileLayout) -> Vec<usize> {
        let mut new_ptr = at.clone();
        if let Some(old_fl) = self.get_raw(at.clone()) {
            let new_fl = match old_fl {
                Self::None => fl,
                Self::Atom(containers, ptr) => {
                    new_ptr.push(0);
                    Self::TopToBottom(vec![(fl, 0.5), (old_fl.clone(), 0.5)])
                }
                Self::SideBySide(layouts) => {
                    new_ptr.push(0);
                    Self::TopToBottom(vec![(fl, 0.5), (old_fl.clone(), 0.5)])
                }
                Self::TopToBottom(layouts) => {
                    new_ptr.push(0);
                    Self::TopToBottom(vec![(fl, 0.5), (old_fl.clone(), 0.5)])
                }
            };
            self.set(at, new_fl);
        }
        new_ptr
    }

    /// Open a split below the current pointer
    pub fn open_down(&mut self, at: Vec<usize>, fl: FileLayout) -> Vec<usize> {
        let mut new_ptr = at.clone();
        if let Some(old_fl) = self.get_raw(at.clone()) {
            let new_fl = match old_fl {
                Self::None => fl,
                Self::Atom(containers, ptr) => {
                    new_ptr.push(1);
                    Self::TopToBottom(vec![(old_fl.clone(), 0.5), (fl, 0.5)])
                }
                Self::SideBySide(layouts) => {
                    new_ptr.push(1);
                    Self::TopToBottom(vec![(old_fl.clone(), 0.5), (fl, 0.5)])
                }
                Self::TopToBottom(layouts) => {
                    new_ptr.push(1);
                    Self::TopToBottom(vec![(old_fl.clone(), 0.5), (fl, 0.5)])
                }
            };
            self.set(at, new_fl);
        }
        new_ptr
    }

    /// Open a split to the left of the current pointer
    pub fn open_left(&mut self, at: Vec<usize>, fl: FileLayout) -> Vec<usize> {
        let mut new_ptr = at.clone();
        if let Some(old_fl) = self.get_raw(at.clone()) {
            let new_fl = match old_fl {
                Self::None => fl,
                Self::Atom(containers, ptr) => {
                    new_ptr.push(0);
                    Self::SideBySide(vec![(fl, 0.5), (old_fl.clone(), 0.5)])
                }
                Self::SideBySide(layouts) => {
                    new_ptr.push(0);
                    Self::SideBySide(vec![(fl, 0.5), (old_fl.clone(), 0.5)])
                }
                Self::TopToBottom(layouts) => {
                    new_ptr.push(0);
                    Self::SideBySide(vec![(fl, 0.5), (old_fl.clone(), 0.5)])
                }
            };
            self.set(at, new_fl);
        }
        new_ptr
    }

    /// Open a split to the right of the current pointer
    pub fn open_right(&mut self, at: Vec<usize>, fl: FileLayout) -> Vec<usize> {
        let mut new_ptr = at.clone();
        if let Some(old_fl) = self.get_raw(at.clone()) {
            let new_fl = match old_fl {
                Self::None => fl,
                Self::Atom(containers, ptr) => {
                    new_ptr.push(1);
                    Self::SideBySide(vec![(old_fl.clone(), 0.5), (fl, 0.5)])
                }
                Self::SideBySide(layouts) => {
                    new_ptr.push(1);
                    Self::SideBySide(vec![(old_fl.clone(), 0.5), (fl, 0.5)])
                }
                Self::TopToBottom(layouts) => {
                    new_ptr.push(1);
                    Self::SideBySide(vec![(old_fl.clone(), 0.5), (fl, 0.5)])
                }
            };
            self.set(at, new_fl);
        }
        new_ptr
    }

    /// Get the proportion of a certain node in the tree
    pub fn get_proportion(&self, mut at: Vec<usize>) -> f64 {
        if let Some(last_idx) = at.pop() {
            if let Some(FileLayout::SideBySide(layouts) | FileLayout::TopToBottom(layouts)) =
                self.get_raw(at)
            {
                layouts[last_idx].1
            } else {
                1.0
            }
        } else {
            1.0
        }
    }

    /// Get the proportion of a certain node in the tree
    pub fn set_proportion(&mut self, mut at: Vec<usize>, amount: f64) {
        if let Some(last_idx) = at.pop() {
            if let Some(FileLayout::SideBySide(layouts) | FileLayout::TopToBottom(layouts)) =
                self.get_raw_mut(at)
            {
                layouts[last_idx].1 = amount;
                // Potential problem with this algorithm - it could cause underflow (resulting in props not adding up to 1)
                let reduce = layouts.iter().map(|(_, prop)| prop).sum::<f64>() - 1.0;
                let reduce_each = reduce / layouts.len().saturating_sub(1) as f64;
                layouts
                    .iter_mut()
                    .enumerate()
                    .filter(|(c, _)| *c != last_idx)
                    .for_each(|(_, (_, prop))| *prop -= reduce_each);
            }
        }
    }

    /// Find the nearest parent sidebyside, returns the pointer and we were in it
    pub fn get_sidebyside_parent(&self, mut at: Vec<usize>) -> Option<(Vec<usize>, usize)> {
        // "Zoom out" to try and find a sidebyside parent
        let mut subidx = None;
        while let Some(FileLayout::TopToBottom(_) | FileLayout::Atom(_, _) | FileLayout::None) =
            self.get_raw(at.clone())
        {
            subidx = at.pop();
        }
        subidx.map(|s| (at, s))
    }

    /// Find the nearest parent toptobottom, returns the pointer and we were in it
    pub fn get_toptobottom_parent(&self, mut at: Vec<usize>) -> Option<(Vec<usize>, usize)> {
        // "Zoom out" to try and find a sidebyside parent
        let mut subidx = None;
        while let Some(FileLayout::SideBySide(_) | FileLayout::Atom(_, _) | FileLayout::None) =
            self.get_raw(at.clone())
        {
            subidx = at.pop();
        }
        subidx.map(|s| (at, s))
    }

    /// Find the new cursor position when moving left
    pub fn move_left(&self, at: Vec<usize>) -> Vec<usize> {
        if let Some((mut parent_idx, to_change)) = self.get_sidebyside_parent(at.clone()) {
            // Move backward from where we were
            let to_change = to_change.saturating_sub(1);
            parent_idx.push(to_change);
            // "Zoom in" down to atom level
            while let Some(FileLayout::TopToBottom(_) | FileLayout::SideBySide(_)) =
                self.get_raw(parent_idx.clone())
            {
                parent_idx.push(0);
            }
            parent_idx
        } else {
            at
        }
    }

    /// Find the new cursor position when moving right
    pub fn move_right(&self, at: Vec<usize>) -> Vec<usize> {
        if let Some((mut parent_idx, mut to_change)) = self.get_sidebyside_parent(at.clone()) {
            // Move backward from where we were
            if let Some(FileLayout::SideBySide(layouts)) = self.get_raw(parent_idx.clone()) {
                if to_change + 1 < layouts.len() {
                    to_change = to_change + 1;
                }
            }
            parent_idx.push(to_change);
            // "Zoom in" down to atom level
            while let Some(FileLayout::TopToBottom(_) | FileLayout::SideBySide(_)) =
                self.get_raw(parent_idx.clone())
            {
                parent_idx.push(0);
            }
            parent_idx
        } else {
            at
        }
    }

    /// Find the new cursor position when moving up
    pub fn move_up(&self, at: Vec<usize>) -> Vec<usize> {
        if let Some((mut parent_idx, to_change)) = self.get_toptobottom_parent(at.clone()) {
            // Move backward from where we were
            let to_change = to_change.saturating_sub(1);
            parent_idx.push(to_change);
            // "Zoom in" down to atom level
            while let Some(
                FileLayout::TopToBottom(_) | FileLayout::SideBySide(_) | FileLayout::None,
            ) = self.get_raw(parent_idx.clone())
            {
                parent_idx.push(0);
            }
            parent_idx
        } else {
            at
        }
    }

    /// Find the new cursor position when moving down
    pub fn move_down(&self, at: Vec<usize>) -> Vec<usize> {
        if let Some((mut parent_idx, mut to_change)) = self.get_toptobottom_parent(at.clone()) {
            // Move backward from where we were
            if let Some(FileLayout::TopToBottom(layouts)) = self.get_raw(parent_idx.clone()) {
                if to_change + 1 < layouts.len() {
                    to_change = to_change + 1;
                }
            }
            parent_idx.push(to_change);
            // "Zoom in" down to atom level
            while let Some(FileLayout::TopToBottom(_) | FileLayout::SideBySide(_)) =
                self.get_raw(parent_idx.clone())
            {
                parent_idx.push(0);
            }
            parent_idx
        } else {
            at
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
