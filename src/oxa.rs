/*
    Oxa.rs - Tools for parsing and lexing the Ox Assembly format

    Oxa is an interpreted specific purpose language inspired by x86 assembly
    It is used to write macros for the editor that make editing text painless
    It is also used for writing commands in the "macro mode" when editing

    An example usage could be writing a macro to delete the current line
*/
use crate::undo::BankType;
use crate::util::line_offset;
use crate::{Direction, Event, Position, Row};

#[derive(Debug, Copy, Clone)]
pub enum Variable {
    Saved,
}

pub fn interpret_line(
    line: &str,
    cursor: &Position,
    graphemes: usize,
    rows: &[Row],
) -> Option<Vec<Event>> {
    // Take an instruction of Oxa and interpret it
    let mut events = vec![];
    let mut line = line.split(' ');
    if let Some(instruction) = line.next() {
        let mut args: Vec<&str> = line.collect();
        let root = if let Some(&"sudo") = args.get(0) {
            args.remove(0);
            true
        } else {
            false
        };
        match instruction {
            "new" => events.push(Event::New),
            "open" => events.push(open_command(&args)),
            "undo" => events.push(Event::Undo),
            "commit" => events.push(Event::Commit),
            "redo" => events.push(Event::Redo),
            "quit" => events.push(quit_command(&args)),
            "prev" => events.push(Event::PrevTab),
            "next" => events.push(Event::NextTab),
            "set" => events.push(set_command(&args, &cursor, &rows)),
            "split" => events.push(Event::SplitDown(*cursor, *cursor)),
            "splice" => events.push(Event::SpliceUp(*cursor, *cursor)),
            "search" => events.push(Event::Search),
            "reload" => events.push(Event::ReloadConfig),
            "cmd" => events.push(Event::Cmd),
            "replace" => events.push(replace_command(&args)),
            // Shell with substitution and no confirm
            "shs" => events.push(Event::Shell(args.join(" "), false, true, root)),
            // Shell with substitution and confirm
            "shcs" => events.push(Event::Shell(args.join(" "), true, true, root)),
            // Shell with confirm and no substitution
            "shc" => events.push(Event::Shell(args.join(" "), true, false, root)),
            // Shell with no confirm nor substitution
            "sh" => events.push(Event::Shell(args.join(" "), false, false, root)),
            "is" => {
                if let Some(set) = is_command(&args) {
                    events.push(set)
                } else {
                    return None;
                }
            }
            "theme" => {
                if let Some(theme) = theme_command(&args) {
                    events.push(theme)
                } else {
                    return None;
                }
            }
            "line" => {
                if let Some(line) = line_command(&args, &cursor) {
                    events.push(line);
                } else {
                    return None;
                }
            }
            _ => {
                let i = match instruction {
                    "save" => save_command(&args),
                    "goto" => goto_command(&args),
                    "move" => move_command(&args),
                    "put" => put_command(&args, &cursor),
                    "delete" => delete_command(&args, &cursor, graphemes, &rows),
                    "load" => load_command(&args),
                    "store" => store_command(&args),
                    "overwrite" => overwrite_command(&args, &rows),
                    _ => return None,
                };
                if let Some(mut command) = i {
                    events.append(&mut command);
                } else {
                    return None;
                }
            }
        }
    }
    Some(events)
}

fn is_command(args: &[&str]) -> Option<Event> {
    Some(Event::Set(
        match &args[0][..] {
            "saved" => Variable::Saved,
            _ => return None,
        },
        true,
    ))
}

fn theme_command(args: &[&str]) -> Option<Event> {
    if args.is_empty() {
        None
    } else {
        Some(Event::Theme(args[0].to_string()))
    }
}

fn replace_command(args: &[&str]) -> Event {
    if !args.is_empty() && args[0] == "*" {
        Event::ReplaceAll
    } else {
        Event::Replace
    }
}

fn open_command(args: &[&str]) -> Event {
    Event::Open(if args.is_empty() {
        None
    } else {
        Some(args[0].to_string())
    })
}

fn quit_command(args: &[&str]) -> Event {
    if args.contains(&"*") {
        Event::QuitAll(args.contains(&"!"))
    } else {
        Event::Quit(args.contains(&"!"))
    }
}

fn line_command(args: &[&str], cursor: &Position) -> Option<Event> {
    if args.is_empty() {
        return None;
    } else if let Some(dir) = args.get(0) {
        return match *dir {
            "below" => Some(Event::InsertLineBelow(*cursor)),
            "above" => Some(Event::InsertLineAbove(*cursor)),
            _ => None,
        };
    }
    None
}

fn set_command(args: &[&str], cursor: &Position, rows: &[Row]) -> Event {
    if args.is_empty() {
        Event::UpdateLine(
            *cursor,
            0,
            Box::new(rows[cursor.y].clone()),
            Box::new(Row::from("")),
        )
    } else {
        Event::UpdateLine(
            *cursor,
            0,
            Box::new(rows[cursor.y].clone()),
            Box::new(Row::from(args.join(" ").as_str())),
        )
    }
}

fn overwrite_command(args: &[&str], rows: &[Row]) -> Option<Vec<Event>> {
    Some(vec![if args.is_empty() {
        Event::Overwrite(rows.to_vec(), vec![Row::from("")])
    } else {
        Event::Overwrite(
            rows.to_vec(),
            args.join(" ")
                .split("\\n")
                .map(Row::from)
                .collect::<Vec<_>>(),
        )
    }])
}

fn save_command(args: &[&str]) -> Option<Vec<Event>> {
    let mut events = vec![];
    if args.is_empty() {
        events.push(Event::Save(None, false))
    } else {
        events.push(if args[0] == "*" {
            Event::SaveAll
        } else if args[0] == "?" {
            Event::Save(None, true)
        } else {
            Event::Save(Some(args[0].to_string()), false)
        })
    }
    Some(events)
}

fn store_command(args: &[&str]) -> Option<Vec<Event>> {
    let mut events = vec![];
    if args.len() == 2 {
        if let Ok(bank) = args[1].parse::<usize>() {
            if let Some(kind) = args.get(0) {
                match *kind {
                    "cursor" => events.push(Event::Store(BankType::Cursor, bank)),
                    "line" => events.push(Event::Store(BankType::Line, bank)),
                    _ => return None,
                }
            }
        } else {
            return None;
        }
    } else {
        return None;
    }
    Some(events)
}

fn load_command(args: &[&str]) -> Option<Vec<Event>> {
    let mut events = vec![];
    if args.len() == 2 {
        if let Ok(bank) = args[1].parse::<usize>() {
            if let Some(kind) = args.get(0) {
                match *kind {
                    "cursor" => events.push(Event::Load(BankType::Cursor, bank)),
                    "line" => events.push(Event::Load(BankType::Line, bank)),
                    _ => return None,
                }
            }
        } else {
            return None;
        }
    } else {
        return None;
    }
    Some(events)
}

fn goto_command(args: &[&str]) -> Option<Vec<Event>> {
    let mut events = vec![];
    match args.len() {
        0 => events.push(Event::GotoCursor(Position { x: 0, y: 0 })),
        1 => {
            if let Ok(y) = args[0].parse::<usize>() {
                events.push(Event::GotoCursor(Position {
                    x: 0,
                    y: y.saturating_sub(1),
                }));
            } else {
                return None;
            }
        }
        2 => {
            if let (Ok(x), Ok(y)) = (args[0].parse::<usize>(), args[1].parse::<usize>()) {
                events.push(Event::GotoCursor(Position {
                    x: x.saturating_sub(1),
                    y: y.saturating_sub(1),
                }));
            } else {
                return None;
            }
        }
        _ => return None,
    }
    Some(events)
}

fn put_command(args: &[&str], cursor: &Position) -> Option<Vec<Event>> {
    let mut events = vec![];
    if args[0] == "\\t" {
        events.push(Event::InsertTab(*cursor));
    } else {
        for (c, ch) in args.join(" ").chars().enumerate() {
            events.push(Event::Insertion(
                Position {
                    x: cursor.x.saturating_add(c),
                    y: cursor.y,
                },
                ch,
            ))
        }
    }
    Some(events)
}

fn move_command(args: &[&str]) -> Option<Vec<Event>> {
    let mut events = vec![];
    if args.len() == 2 {
        if let Ok(magnitude) = args[0].parse::<usize>() {
            let direction = args[1];
            events.push(Event::MoveCursor(
                magnitude as i128,
                match direction {
                    "up" => Direction::Up,
                    "down" => Direction::Down,
                    "left" => Direction::Left,
                    "right" => Direction::Right,
                    _ => return None,
                },
            ));
        } else if args[0] == "word" {
            events.push(Event::MoveWord(match args[1] {
                "left" => Direction::Left,
                "right" => Direction::Right,
                _ => return None,
            }));
        } else {
            return None;
        }
    } else if let Some(direction) = args.get(0) {
        events.push(match *direction {
            "home" => Event::Home,
            "end" => Event::End,
            "pageup" => Event::PageUp,
            "pagedown" => Event::PageDown,
            _ => return None,
        });
    } else {
        return None;
    }
    Some(events)
}

fn delete_command(
    args: &[&str],
    cursor: &Position,
    graphemes: usize,
    rows: &[Row],
) -> Option<Vec<Event>> {
    // Handle the delete command (complicated)
    let mut events = vec![];
    if args.is_empty() {
        if let Some(ch) = rows[cursor.y]
            .string
            .chars()
            .collect::<Vec<_>>()
            .get(graphemes)
        {
            events.push(Event::Deletion(*cursor, *ch));
        }
    } else if args[0] == "word" {
        events.push(Event::DeleteWord(*cursor, "egg".to_string()));
    } else if let Ok(line) = args[0].parse::<i128>() {
        events.push(Event::DeleteLine(
            *cursor,
            line,
            Box::new(rows[line_offset(cursor.y, line, rows.len())].clone()),
        ));
    } else {
        return None;
    }
    Some(events)
}
