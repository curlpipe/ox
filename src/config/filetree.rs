/// Related to file tree configuration options
use mlua::prelude::*;

#[derive(Debug)]
pub struct FileTree {
    pub width: usize,
    pub move_focus_to_file: bool,
    pub icons: bool,
    pub language_icons: bool,
}

impl Default for FileTree {
    fn default() -> Self {
        Self {
            width: 30,
            move_focus_to_file: true,
            icons: false,
            language_icons: true,
        }
    }
}

impl LuaUserData for FileTree {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("width", |_, this| Ok(this.width));
        fields.add_field_method_set("width", |_, this, value| {
            this.width = value;
            Ok(())
        });
        fields.add_field_method_get("move_focus_to_file", |_, this| Ok(this.move_focus_to_file));
        fields.add_field_method_set("move_focus_to_file", |_, this, value| {
            this.move_focus_to_file = value;
            Ok(())
        });
        fields.add_field_method_get("icons", |_, this| Ok(this.icons));
        fields.add_field_method_set("icons", |_, this, value| {
            this.icons = value;
            Ok(())
        });
        fields.add_field_method_get("language_icons", |_, this| Ok(this.language_icons));
        fields.add_field_method_set("language_icons", |_, this, value| {
            this.language_icons = value;
            Ok(())
        });
    }
}
