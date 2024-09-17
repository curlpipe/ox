//! # Kaolinite
//! > Kaolinite is an advanced library that handles the backend of a terminal text editor. You can
//! feel free to make your own terminal text editor using kaolinite, or see the reference
//! implementation found under the directory `examples/cactus`.
//!
//! It'll handle things like
//! - Opening and saving files
//! - Handle documents that are too long to be fitted on the whole terminal
//! - Rendering line numbers
//! - Insertion and deletion from the document
//! - File type detection
//! - Undo & Redo
//! - Moving around the document, by word, page, character or other means
//! - Searching & Replacing
//! - Handles tabs, different line endings and double width characters perfectly
//! - File buffering for larger files
//!
//! It removes a lot of complexity from your text editor and allows the creation of an advanced
//! text editor in very few lines of idiomatic code.
//!
//! To get started, check out the [Document] struct, which will allow you to open, edit and save
//! documents.
//! I also highly recommend that you check out `examples/cactus/src/main.rs` which is a full
//! implementation of kaolinite, and can be used as a base for your very own editor. It's well
//! documented and explains what it's doing.

#![warn(clippy::all, clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
pub mod document;
pub mod event;
pub mod map;
pub mod searching;
pub mod utils;

pub use document::Document;
pub use utils::{Loc, Size};
