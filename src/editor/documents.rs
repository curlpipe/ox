use crate::editor::FileType;
use kaolinite::Document;
use synoptic::Highlighter;

pub struct FileContainer {
    pub doc: Document,
    pub highlighter: Highlighter,
    pub file_type: Option<FileType>,
}