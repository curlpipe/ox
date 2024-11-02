/// For changing around syntax highlighting in the config file
use crate::error::{OxError, Result};
use crossterm::style::Color as CColor;
use mlua::prelude::*;
use std::collections::HashMap;
use synoptic::Highlighter;

use super::Color;

type BoundedInterpArgs = (String, String, String, String, String, bool);

/// For storing configuration information related to syntax highlighting
#[derive(Debug)]
#[allow(clippy::module_name_repetitions)]
pub struct SyntaxHighlighting {
    pub theme: HashMap<String, Color>,
    pub user_rules: HashMap<String, Highlighter>,
}

impl Default for SyntaxHighlighting {
    fn default() -> Self {
        let mut theme = HashMap::default();
        theme.insert("string".to_string(), Color::Rgb(39, 222, 145));
        theme.insert("comment".to_string(), Color::Rgb(113, 113, 169));
        theme.insert("digit".to_string(), Color::Rgb(40, 198, 232));
        theme.insert("keyword".to_string(), Color::Rgb(134, 76, 232));
        theme.insert("attribute".to_string(), Color::Rgb(40, 198, 232));
        theme.insert("character".to_string(), Color::Rgb(40, 198, 232));
        theme.insert("type".to_string(), Color::Rgb(47, 141, 252));
        theme.insert("function".to_string(), Color::Rgb(47, 141, 252));
        theme.insert("header".to_string(), Color::Rgb(40, 198, 232));
        theme.insert("macro".to_string(), Color::Rgb(223, 52, 249));
        theme.insert("namespace".to_string(), Color::Rgb(47, 141, 252));
        theme.insert("struct".to_string(), Color::Rgb(47, 141, 252));
        theme.insert("operator".to_string(), Color::Rgb(113, 113, 169));
        theme.insert("boolean".to_string(), Color::Rgb(86, 217, 178));
        theme.insert("table".to_string(), Color::Rgb(47, 141, 252));
        theme.insert("reference".to_string(), Color::Rgb(134, 76, 232));
        theme.insert("tag".to_string(), Color::Rgb(40, 198, 232));
        theme.insert("heading".to_string(), Color::Rgb(47, 141, 252));
        theme.insert("link".to_string(), Color::Rgb(223, 52, 249));
        theme.insert("key".to_string(), Color::Rgb(223, 52, 249));
        theme.insert("quote".to_string(), Color::Rgb(113, 113, 169));
        theme.insert("bold".to_string(), Color::Rgb(40, 198, 232));
        theme.insert("italic".to_string(), Color::Rgb(40, 198, 232));
        theme.insert("block".to_string(), Color::Rgb(40, 198, 232));
        theme.insert("image".to_string(), Color::Rgb(40, 198, 232));
        theme.insert("list".to_string(), Color::Rgb(86, 217, 178));
        theme.insert("insertion".to_string(), Color::Rgb(39, 222, 145));
        theme.insert("deletion".to_string(), Color::Rgb(255, 100, 100));
        Self {
            theme,
            user_rules: HashMap::default(),
        }
    }
}

impl SyntaxHighlighting {
    /// Get a colour from the theme
    pub fn get_theme(&self, name: &str) -> Result<CColor> {
        if let Some(col) = self.theme.get(name) {
            col.to_color()
        } else {
            let msg = format!("{name} has not been given a colour in the theme");
            Err(OxError::Config { msg })
        }
    }
}

impl LuaUserData for SyntaxHighlighting {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method_mut(
            "keywords",
            |lua, _, (name, pattern): (String, Vec<String>)| {
                let table = lua.create_table()?;
                table.set("kind", "keyword")?;
                table.set("name", name)?;
                table.set("pattern", format!("({})", pattern.join("|")))?;
                Ok(table)
            },
        );
        methods.add_method_mut("keyword", |lua, _, (name, pattern): (String, String)| {
            let table = lua.create_table()?;
            table.set("kind", "keyword")?;
            table.set("name", name)?;
            table.set("pattern", pattern)?;
            Ok(table)
        });
        methods.add_method_mut(
            "bounded",
            |lua, _, (name, start, end, escape): (String, String, String, bool)| {
                let table = lua.create_table()?;
                table.set("kind", "bounded")?;
                table.set("name", name)?;
                table.set("start", start)?;
                table.set("end", end)?;
                table.set("escape", escape.to_string())?;
                Ok(table)
            },
        );
        methods.add_method_mut(
            "bounded_interpolation",
            |lua, _, (name, start, end, i_start, i_end, escape): BoundedInterpArgs| {
                let table = lua.create_table()?;
                table.set("kind", "bounded_interpolation")?;
                table.set("name", name)?;
                table.set("start", start)?;
                table.set("end", end)?;
                table.set("i_start", i_start)?;
                table.set("i_end", i_end)?;
                table.set("escape", escape.to_string())?;
                Ok(table)
            },
        );
        methods.add_method_mut(
            "new",
            |_, syntax_highlighting, (name, rules): (String, LuaTable)| {
                // Create highlighter
                let mut highlighter = Highlighter::new(4);
                // Add rules one by one
                for rule_idx in 1..=(rules.len()?) {
                    // Get rule
                    let rule = rules.get::<HashMap<String, String>>(rule_idx)?;
                    // Find type of rule and attatch it to the highlighter
                    match rule["kind"].as_str() {
                        "keyword" => {
                            highlighter.keyword(rule["name"].clone(), &rule["pattern"]);
                        }
                        "bounded" => highlighter.bounded(
                            rule["name"].clone(),
                            rule["start"].clone(),
                            rule["end"].clone(),
                            rule["escape"] == "true",
                        ),
                        "bounded_interpolation" => highlighter.bounded_interp(
                            rule["name"].clone(),
                            rule["start"].clone(),
                            rule["end"].clone(),
                            rule["i_start"].clone(),
                            rule["i_end"].clone(),
                            rule["escape"] == "true",
                        ),
                        _ => unreachable!(),
                    }
                }
                syntax_highlighting.user_rules.insert(name, highlighter);
                Ok(())
            },
        );
        methods.add_method_mut("set", |_, syntax_highlighting, (name, value)| {
            syntax_highlighting
                .theme
                .insert(name, Color::from_lua(value));
            Ok(())
        });
    }
}
