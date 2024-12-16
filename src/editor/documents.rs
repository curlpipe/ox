/// Tools for placing all information about open files into one place
use crate::editor::{get_absolute_path, Editor, FileType};
#[cfg(not(target_os = "windows"))]
use crate::pty::Pty;
use crate::Loc;
use kaolinite::Document;
use kaolinite::Size;
use std::ops::Range;
#[cfg(not(target_os = "windows"))]
use std::sync::{Arc, Mutex};
use synoptic::Highlighter;

pub type Span = Vec<(Vec<usize>, Range<usize>, Range<usize>)>;

// File split structure
#[derive(Debug)]
pub enum FileLayout {
    /// Side-by-side documents (with proportions)
    SideBySide(Vec<(FileLayout, f64)>),
    /// Top-to-bottom documents (with proportions)
    TopToBottom(Vec<(FileLayout, f64)>),
    /// Single file container (and pointer for tabs)
    Atom(Vec<FileContainer>, usize),
    /// Placeholder for an empty file split
    None,
    /// Representing a file tree
    FileTree,
    /// Representing a terminal
    #[cfg(not(target_os = "windows"))]
    Terminal(Arc<Mutex<Pty>>),
    #[allow(dead_code)]
    #[cfg(target_os = "windows")]
    Terminal(()),
}

impl Default for FileLayout {
    fn default() -> Self {
        Self::None
    }
}

impl FileLayout {
    /// Will return file containers and what span of columns and rows they take up
    /// In the format of (container, rows, columns)
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    pub fn span(&self, idx: Vec<usize>, size: Size, at: Loc) -> Span {
        match self {
            Self::None => vec![],
            // Atom file trees and terminals: stretch from starting position through to end of their containers
            Self::Atom(_, _) | Self::FileTree | Self::Terminal(_) => {
                vec![(idx, at.y..at.y + size.h, at.x..at.x + size.w)]
            }
            // SideBySide: distributes available container space to each sub-layout
            Self::SideBySide(layouts) => {
                let mut result = vec![];
                let mut remaining = size.w.saturating_sub(1);
                let mut up_to = at.x;
                for (c, (layout, prop)) in layouts.iter().enumerate() {
                    let last = c == layouts.len().saturating_sub(1);
                    // Calculate the width
                    let mut base_width = (size.w as f64 * prop) as usize;
                    if last {
                        // Tack on any remaining things
                        base_width += remaining.saturating_sub(base_width);
                    } else {
                        // Leave room for a vertical bar
                        base_width = base_width.saturating_sub(1);
                    }
                    // Calculate location and size information for this layout in particular
                    let sub_size = Size {
                        w: base_width,
                        h: size.h,
                    };
                    let sub_at = Loc { x: up_to, y: at.y };
                    // Calculate new index
                    let mut sub_idx = idx.clone();
                    sub_idx.push(c);
                    let mut sub_span = layout.span(sub_idx, sub_size, sub_at);
                    // Update values
                    result.append(&mut sub_span);
                    remaining = remaining.saturating_sub(sub_size.w);
                    up_to += sub_size.w + usize::from(!last);
                }
                result
            }
            Self::TopToBottom(layouts) => {
                let mut result = vec![];
                let mut remaining = size.h.saturating_sub(1);
                let mut up_to = at.y;
                for (c, (layout, prop)) in layouts.iter().enumerate() {
                    let last = c == layouts.len().saturating_sub(1);
                    // Calculate the height
                    let mut base_height = (size.h as f64 * prop) as usize;
                    if last {
                        // Tack on any remaining things
                        base_height += remaining.saturating_sub(base_height);
                    } else {
                        // Leave room for a horizontal bar
                        base_height = base_height.saturating_sub(1);
                    }
                    // Calculate location and size information for this layout in particular
                    let sub_size = Size {
                        h: base_height,
                        w: size.w,
                    };
                    let sub_at = Loc { x: at.x, y: up_to };
                    // Calculate new index
                    let mut sub_idx = idx.clone();
                    sub_idx.push(c);
                    let mut sub_span = layout.span(sub_idx, sub_size, sub_at);
                    // Update values
                    result.append(&mut sub_span);
                    remaining = remaining.saturating_sub(sub_size.h);
                    up_to += sub_size.h + usize::from(!last);
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

    /// Work out if a certain index on a certain line is empty
    pub fn is_empty_at(y: usize, x: usize, span: &Span) -> bool {
        let line = Self::line(y, span);
        !line.is_empty() && !line.iter().any(|(_, _, cols)| cols.contains(&x))
    }

    /// Update the sizes of documents
    pub fn update_doc_sizes(&self, span: &Span, ed: &Editor) -> Vec<(Vec<usize>, usize, Size)> {
        let mut result = vec![];
        // For each atom
        for (idx, rows, cols) in span {
            if let Some((fcs, _)) = self.get_atom(idx.clone()) {
                // For each document in this atom
                for (doc, _) in fcs.iter().enumerate() {
                    // Work out correct new document width
                    let new_size = Size {
                        h: rows.end.saturating_sub(rows.start + ed.push_down + 1),
                        w: cols.end.saturating_sub(cols.start + ed.dent_for(idx, doc)),
                    };
                    result.push((idx.clone(), doc, new_size));
                }
            }
        }
        result
    }

    /// Work out how many files are currently open
    pub fn len(&self) -> usize {
        match self {
            Self::None | Self::FileTree | Self::Terminal(_) => 0,
            Self::Atom(containers, _) => containers.len(),
            Self::SideBySide(layouts) => layouts.iter().map(|(layout, _)| layout.len()).sum(),
            Self::TopToBottom(layouts) => layouts.iter().map(|(layout, _)| layout.len()).sum(),
        }
    }

    /// Work out how many atoms are currently open
    pub fn n_atoms(&self) -> usize {
        match self {
            Self::None | Self::FileTree | Self::Terminal(_) => 0,
            Self::Atom(_, _) => 1,
            Self::SideBySide(layouts) => layouts.iter().map(|(layout, _)| layout.n_atoms()).sum(),
            Self::TopToBottom(layouts) => layouts.iter().map(|(layout, _)| layout.n_atoms()).sum(),
        }
    }

    /// Find a file container location from it's path
    pub fn find(&self, idx: Vec<usize>, path: &str) -> Option<(Vec<usize>, usize)> {
        match self {
            Self::None | Self::FileTree | Self::Terminal(_) => None,
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
            Self::SideBySide(layouts) | Self::TopToBottom(layouts) => {
                // Recursively scan
                for (nth, (layout, _)) in layouts.iter().enumerate() {
                    let mut this_idx = idx.clone();
                    this_idx.push(nth);
                    let result = layout.find(this_idx, path);
                    if result.is_some() {
                        return result;
                    }
                }
                None
            }
        }
    }

    /// Get the `FileLayout` at a certain index
    pub fn get_raw(&self, mut idx: Vec<usize>) -> Option<&FileLayout> {
        match self {
            Self::None | Self::Atom(_, _) | Self::FileTree | Self::Terminal(_) => Some(self),
            Self::SideBySide(layouts) => {
                if idx.is_empty() {
                    Some(self)
                } else {
                    let subidx = idx.remove(0);
                    layouts.get(subidx)?.0.get_raw(idx)
                }
            }
            Self::TopToBottom(layouts) => {
                if idx.is_empty() {
                    Some(self)
                } else {
                    let subidx = idx.remove(0);
                    layouts.get(subidx)?.0.get_raw(idx)
                }
            }
        }
    }

    /// Get the `FileLayout` at a certain index (mutable)
    pub fn get_raw_mut(&mut self, mut idx: Vec<usize>) -> Option<&mut FileLayout> {
        if idx.is_empty() {
            Some(self)
        } else {
            match self {
                Self::None | Self::Atom(_, _) | Self::FileTree | Self::Terminal(_) => Some(self),
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

    /// Get the `FileLayout` at a certain index
    pub fn set(&mut self, mut idx: Vec<usize>, fl: FileLayout) {
        match self {
            Self::None | Self::Atom(_, _) | Self::FileTree | Self::Terminal(_) => *self = fl,
            Self::SideBySide(layouts) | Self::TopToBottom(layouts) => {
                if idx.is_empty() {
                    *self = fl;
                } else {
                    let subidx = idx.remove(0);
                    layouts[subidx].0.set(idx, fl);
                }
            }
        }
    }

    /// Given an index, find the file containers in the tree
    pub fn get_atom(&self, mut idx: Vec<usize>) -> Option<(&[FileContainer], usize)> {
        match self {
            Self::None | Self::FileTree | Self::Terminal(_) => None,
            Self::Atom(containers, ptr) => Some((containers, *ptr)),
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
            Self::None | Self::FileTree | Self::Terminal(_) => None,
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
    pub fn get_all(&self, idx: Vec<usize>) -> &[FileContainer] {
        self.get_atom(idx).map_or(&[], |(fcs, _)| fcs)
    }

    /// Given an index, find the file container in the tree
    pub fn get(&self, idx: Vec<usize>) -> Option<&FileContainer> {
        let (fcs, ptr) = self.get_atom(idx)?;
        fcs.get(ptr)
    }

    /// Given an index, find the file container in the tree
    pub fn get_mut(&mut self, idx: Vec<usize>) -> Option<&mut FileContainer> {
        let (fcs, ptr) = self.get_atom_mut(idx)?;
        fcs.get_mut(*ptr)
    }

    /// In the currently active atom, move to a different document
    pub fn move_to(&mut self, mut idx: Vec<usize>, ptr: usize) {
        match self {
            Self::None | Self::FileTree | Self::Terminal(_) => (),
            Self::Atom(_, ref mut old_ptr) => *old_ptr = ptr,
            Self::SideBySide(layouts) | Self::TopToBottom(layouts) => {
                let subidx = idx.remove(0);
                layouts[subidx].0.move_to(idx, ptr);
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

    /// Remove any empty atoms
    pub fn clean_up_multis(&mut self, mut idx: Vec<usize>) -> Vec<usize> {
        // Continue checking for redundant sidebyside / toptobottom
        while let Some(redundant_idx) = self.redundant_multis(vec![]) {
            let multi = self.get_raw_mut(redundant_idx.clone());
            if let Some(layout) = multi {
                if let Self::SideBySide(layouts) | Self::TopToBottom(layouts) = layout {
                    let retain = std::mem::take(&mut layouts[0].0);
                    *layout = retain;
                    if idx.starts_with(&redundant_idx) {
                        idx.remove(redundant_idx.len());
                        return idx;
                    }
                }
            }
        }
        idx
    }

    /// Remove a certain index from this tree
    #[allow(clippy::cast_precision_loss)]
    pub fn remove(&mut self, at: Vec<usize>) {
        // Get parent of the node we wish to delete
        let mut at_parent = at.clone();
        if let Some(within_parent) = at_parent.pop() {
            // Determine behaviour based on parent
            if let Some(parent) = self.get_raw_mut(at_parent) {
                match parent {
                    Self::None | Self::Atom(_, _) | Self::FileTree | Self::Terminal(_) => {
                        unreachable!()
                    }
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
            Self::None | Self::FileTree | Self::Terminal(_) => None,
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

    /// Traverse the tree and return a list of indices to redundant sidebyside/toptobottom
    pub fn redundant_multis(&self, at: Vec<usize>) -> Option<Vec<usize>> {
        match self {
            Self::None | Self::FileTree | Self::Atom(_, _) | Self::Terminal(_) => None,
            Self::SideBySide(layouts) | Self::TopToBottom(layouts) => {
                if layouts.len() == 1 {
                    Some(at)
                } else {
                    for (c, layout) in layouts.iter().enumerate() {
                        let mut idx = at.clone();
                        idx.push(c);
                        if let Some(result) = layout.0.redundant_multis(idx) {
                            return Some(result);
                        }
                    }
                    None
                }
            }
        }
    }

    /// Traverse the tree and return a list of indices to empty atoms
    #[cfg(not(target_os = "windows"))]
    pub fn terminal_rerender(&mut self) -> bool {
        match self {
            Self::None | Self::FileTree | Self::Atom(_, _) => false,
            Self::Terminal(term) => {
                let mut term = term.lock().unwrap();
                if term.force_rerender {
                    term.force_rerender = false;
                    true
                } else {
                    false
                }
            }
            Self::SideBySide(layouts) | Self::TopToBottom(layouts) => {
                for layout in layouts.iter_mut() {
                    if layout.0.terminal_rerender() {
                        return true;
                    }
                }
                false
            }
        }
    }

    /// Find a new pointer position when something is removed
    pub fn new_pointer_position(&self, old: &[usize]) -> Vec<usize> {
        // Zoom out until a sidebyside or toptobottom is found
        let mut copy = old.to_owned();
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
        if let Some(old_fl) = self.get_raw_mut(at.clone()) {
            let new_fl = match old_fl {
                Self::None => fl,
                Self::Atom(_, _)
                | Self::SideBySide(_)
                | Self::TopToBottom(_)
                | Self::Terminal(_) => {
                    new_ptr.push(0);
                    let old_fl = std::mem::replace(old_fl, FileLayout::None);
                    Self::TopToBottom(vec![(fl, 0.5), (old_fl, 0.5)])
                }
                Self::FileTree => return at,
            };
            self.set(at, new_fl);
        }
        new_ptr
    }

    /// Open a split below the current pointer
    pub fn open_down(&mut self, at: Vec<usize>, fl: FileLayout) -> Vec<usize> {
        let mut new_ptr = at.clone();
        if let Some(old_fl) = self.get_raw_mut(at.clone()) {
            let new_fl = match old_fl {
                Self::None => fl,
                Self::Atom(_, _)
                | Self::SideBySide(_)
                | Self::TopToBottom(_)
                | Self::Terminal(_) => {
                    new_ptr.push(1);
                    let old_fl = std::mem::replace(old_fl, FileLayout::None);
                    Self::TopToBottom(vec![(old_fl, 0.5), (fl, 0.5)])
                }
                Self::FileTree => return at,
            };
            self.set(at, new_fl);
        }
        new_ptr
    }

    /// Open a split to the left of the current pointer
    pub fn open_left(&mut self, at: Vec<usize>, fl: FileLayout) -> Vec<usize> {
        let mut new_ptr = at.clone();
        if let Some(old_fl) = self.get_raw_mut(at.clone()) {
            let new_fl = match old_fl {
                Self::None => fl,
                Self::Atom(_, _)
                | Self::SideBySide(_)
                | Self::TopToBottom(_)
                | Self::Terminal(_) => {
                    new_ptr.push(0);
                    let old_fl = std::mem::replace(old_fl, FileLayout::None);
                    Self::SideBySide(vec![(fl, 0.5), (old_fl, 0.5)])
                }
                Self::FileTree => return at,
            };
            self.set(at, new_fl);
        }
        new_ptr
    }

    /// Open a split to the right of the current pointer
    pub fn open_right(&mut self, at: Vec<usize>, fl: FileLayout) -> Vec<usize> {
        let mut new_ptr = at.clone();
        if let Some(old_fl) = self.get_raw_mut(at.clone()) {
            let new_fl = match old_fl {
                Self::None => fl,
                Self::Atom(_, _)
                | Self::SideBySide(_)
                | Self::TopToBottom(_)
                | Self::Terminal(_) => {
                    new_ptr.push(1);
                    let old_fl = std::mem::replace(old_fl, FileLayout::None);
                    Self::SideBySide(vec![(old_fl, 0.5), (fl, 0.5)])
                }
                Self::FileTree => return at,
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
    #[allow(clippy::cast_precision_loss)]
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

    /// Shrink this split's width
    pub fn shrink_width(&mut self, at: &[usize], amount: f64) {
        // Find the parent
        if let Some((idx, one_down)) = self.get_sidebyside_parent(at.to_vec()) {
            // Got a side by side parent! Adjust the proportion
            let mut child = idx.clone();
            child.push(one_down);
            let current_prop = self.get_proportion(child.clone());
            if current_prop > amount {
                self.set_proportion(child, current_prop - amount);
            }
        }
    }

    /// Grow this split's width
    pub fn grow_width(&mut self, at: &[usize], amount: f64) {
        // Find the parent
        if let Some((idx, one_down)) = self.get_sidebyside_parent(at.to_vec()) {
            // Got a side by side parent! Adjust the proportion
            let mut child = idx.clone();
            child.push(one_down);
            let current_prop = self.get_proportion(child.clone());
            if current_prop + amount < 1.0 {
                self.set_proportion(child, current_prop + amount);
            }
        }
    }

    /// Shrink this split's height
    pub fn shrink_height(&mut self, at: &[usize], amount: f64) {
        // Find the parent
        if let Some((idx, one_down)) = self.get_toptobottom_parent(at.to_vec()) {
            // Got a top to bottom parent! Adjust the proportion
            let mut child = idx.clone();
            child.push(one_down);
            let current_prop = self.get_proportion(child.clone());
            if current_prop > amount {
                self.set_proportion(child, current_prop - amount);
            }
        }
    }

    /// Grow this split's height
    pub fn grow_height(&mut self, at: &[usize], amount: f64) {
        // Find the parent
        if let Some((idx, one_down)) = self.get_toptobottom_parent(at.to_vec()) {
            // Got a top to bottom parent! Adjust the proportion
            let mut child = idx.clone();
            child.push(one_down);
            let current_prop = self.get_proportion(child.clone());
            if current_prop + amount < 1.0 {
                self.set_proportion(child, current_prop + amount);
            }
        }
    }

    /// Find the nearest parent sidebyside, returns the pointer and where we were in it
    pub fn get_sidebyside_parent(&self, mut at: Vec<usize>) -> Option<(Vec<usize>, usize)> {
        // "Zoom out" to try and find a sidebyside parent
        let mut subidx = None;
        while let Some(FileLayout::TopToBottom(_) | FileLayout::Atom(_, _) | FileLayout::None) =
            self.get_raw(at.clone())
        {
            if at.is_empty() {
                return None;
            }
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
            if at.is_empty() {
                return None;
            }
            subidx = at.pop();
        }
        subidx.map(|s| (at, s))
    }

    /// Find the current location in a particular span
    pub fn get_line_pos(at: &[usize], span: &Span) -> Option<(usize, usize)> {
        // Find the first line that has this split rendered in
        let mut blank_count = 0;
        let mut y = 0;
        while blank_count < 2 {
            let line = Self::line(y, span);
            // Update blank detection
            if line.is_empty() {
                blank_count += 1;
            } else {
                blank_count = 0;
            }
            // Check whether this line contains the current split
            let find_position = line
                .iter()
                .enumerate()
                .find(|(_, (idx, _, _))| *idx == at)
                .map(|(c, _)| c);
            if let Some(our_idx) = find_position {
                return Some((y, our_idx));
            }
            // Ready for next iteration
            y += 1;
        }
        None
    }

    /// Find the current location in a particular span (last line)
    pub fn get_line_pos_last(at: &[usize], span: &Span) -> Option<(usize, usize)> {
        // Find the last line that has this split rendered in
        let mut blank_count = 0;
        let mut y = 0;
        let mut result = None;
        while blank_count < 2 {
            let line = Self::line(y, span);
            // Update blank detection
            if line.is_empty() {
                blank_count += 1;
            } else {
                blank_count = 0;
            }
            // Check whether this line contains the current split
            let find_position = line
                .iter()
                .enumerate()
                .find(|(_, (idx, _, _))| *idx == at)
                .map(|(c, _)| c);
            if let Some(our_idx) = find_position {
                result = Some((y, our_idx));
            } else if result.is_some() {
                break;
            }
            // Ready for next iteration
            y += 1;
        }
        result
    }

    /// Find the new cursor position when moving left
    pub fn move_left(at: Vec<usize>, span: &Span) -> Vec<usize> {
        // Get the geometric location
        if let Some((y, our_idx)) = Self::get_line_pos(&at, span) {
            // Try to find the one before it
            let prior_idx = our_idx.saturating_sub(1);
            if let Some((new_idx, _, _)) = Self::line(y, span).get(prior_idx) {
                new_idx.clone()
            } else {
                at
            }
        } else {
            at
        }
    }

    /// Find the new cursor position when moving right
    pub fn move_right(at: Vec<usize>, span: &Span) -> Vec<usize> {
        // Get the geometric location
        if let Some((y, our_idx)) = Self::get_line_pos(&at, span) {
            // Try to find the one after it
            let next_idx = our_idx + 1;
            if let Some((new_idx, _, _)) = Self::line(y, span).get(next_idx) {
                return new_idx.clone();
            }
            at
        } else {
            at
        }
    }

    /// Find the new cursor position when moving up
    pub fn move_up(at: Vec<usize>, span: &Span) -> Vec<usize> {
        // Get geometric location
        if let Some((y, our_idx)) = Self::get_line_pos(&at, span) {
            if let Some((_, _, cols)) = Self::line(y, span).get(our_idx) {
                // Get the starting column index
                let at_x = cols.start;
                // Work from this y position upwards
                let mut at_y = y.saturating_sub(1);
                loop {
                    // Attempt to find part containing matching at_x value
                    if let Some((idx, _, _)) = Self::line(at_y, span)
                        .iter()
                        .find(|(_, _, cols)| cols.contains(&at_x))
                    {
                        // Found match!
                        return idx.clone();
                    }
                    if at_y == 0 {
                        break;
                    }
                    at_y = at_y.saturating_sub(1);
                }
            }
        }
        at
    }

    /// Find the new cursor position when moving down
    pub fn move_down(at: Vec<usize>, span: &Span) -> Vec<usize> {
        // Get geometric location
        if let Some((y, our_idx)) = Self::get_line_pos_last(&at, span) {
            if let Some((_, _, cols)) = Self::line(y, span).get(our_idx) {
                // Get the starting column index
                let at_x = cols.start;
                // Work from this y position downwards
                let mut at_y = y + 1;
                let mut blanks = 0;
                loop {
                    // Attempt to find part containing matching at_x value
                    let line_reg = Self::line(at_y, span);
                    if let Some((idx, _, _)) =
                        line_reg.iter().find(|(_, _, cols)| cols.contains(&at_x))
                    {
                        // Found match!
                        return idx.clone();
                    } else if line_reg.is_empty() {
                        blanks += 1;
                    }
                    if blanks >= 2 {
                        break;
                    }
                    at_y += 1;
                }
            }
        }
        at
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
