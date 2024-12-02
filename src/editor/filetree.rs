use crate::editor::FileLayout;
/// Utilities for handling the file tree
use crate::Editor;

impl Editor {
    /// Open the file tree
    #[allow(clippy::cast_precision_loss)]
    pub fn open_file_tree(&mut self) {
        if !self.file_tree_is_open() {
            // Wrap existing file layout in new file layout
            if let FileLayout::SideBySide(ref mut layouts) = &mut self.files {
                // Shrink existing splits
                let redistribute = 0.2 / layouts.len() as f64;
                for (_, prop) in &mut *layouts {
                    if *prop >= redistribute {
                        *prop -= redistribute;
                    } else {
                        *prop = 0.0;
                    }
                }
                // Insert file tree
                layouts.insert(0, (FileLayout::FileTree, 0.2));
            } else {
                self.files = FileLayout::SideBySide(vec![
                    (FileLayout::FileTree, 0.2),
                    (self.files.clone(), 0.8),
                ]);
            }
            self.ptr = vec![0];
        }
    }

    /// Close the file tree
    pub fn close_file_tree(&mut self) {
        if let Some(FileLayout::SideBySide(layouts)) = self.files.get_raw(vec![]) {
            // Locate where the file tree is
            let ftp = layouts
                .iter()
                .position(|(l, _)| matches!(l, FileLayout::FileTree));
            if let Some(at) = ftp {
                // Delete the file tree
                self.files.remove(vec![at]);
            }
        }
    }

    /// Toggle the file tree
    pub fn toggle_file_tree(&mut self) {
        if self.file_tree_is_open() {
            self.close_file_tree();
        } else {
            self.open_file_tree();
        }
    }

    /// Determine whether the file tree is open
    pub fn file_tree_is_open(&self) -> bool {
        if let Some(FileLayout::SideBySide(layouts)) = self.files.get_raw(vec![]) {
            layouts
                .iter()
                .any(|(layout, _)| matches!(layout, FileLayout::FileTree))
        } else {
            false
        }
    }
}
