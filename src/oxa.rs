/*
    Oxa.rs - Tools for parsing and lexing the Ox Assembly format

    Oxa is an interpreted specific purpose language inspired by x86 assembly
    It is used to write macros for the editor that make editing text painless
    It is also used for writing commands in the "macro mode" when editing

    An example usage could be writing a macro to delete the current line
*/
use crate::{Event, Position, Row};

pub fn interpret_line(line: &str, cursor: &Position, rows: &[Row]) -> Option<Vec<Event>> {
    // Take an instruction of Oxa and interpret it
    let mut events = vec![];
    let mut line = line.split(' ');
    if let Some(instruction) = line.next() {
        let args: Vec<&str> = line.collect();
        match instruction {
            "goto" => {
                if args.len() == 1 {
                    if let Ok(y) = args[0].parse() {
                        events.push(Event::GotoCursor(Position { x: 0, y }));
                    } else {
                        return None;
                    }
                } else if args.len() == 2 {
                    if let (Ok(x), Ok(y)) = (args[0].parse(), args[1].parse()) {
                        events.push(Event::GotoCursor(Position { x, y }));
                    } else {
                        return None;
                    }
                } else {
                    return None;
                }
            }
            "move" => {
                if args.len() == 1 {
                    if let Ok(y) = args[0].parse() {
                        events.push(Event::MoveCursor(0, y));
                    } else {
                        return None;
                    }
                } else if let (Ok(x), Ok(y)) = (args[0].parse(), args[1].parse()) {
                    events.push(Event::MoveCursor(x, y))
                } else {
                    return None;
                }
            }
            "put" => {
                if args[0] == "\n" {
                    events.push(Event::ReturnEnd(*cursor));
                } else {
                    for (c, ch) in args.join(" ").chars().enumerate() {
                        events.push(Event::InsertMid(
                            Position {
                                x: cursor.x.saturating_add(c),
                                y: cursor.y,
                            },
                            ch,
                        ))
                    }
                }
            }
            "delete" => {
                if let Some(mut evts) = delete_command(&args, cursor, rows) {
                    events.append(&mut evts);
                } else {
                    return None;
                }
            }
            "new" => events.push(Event::New),
            "open" => events.push(Event::Open(if args.is_empty() {
                None
            } else {
                Some(args[0].to_string())
            })),
            "save" => {
                if !args.is_empty() {
                    events.push(if args[0] == "*" {
                        Event::SaveAll
                    } else {
                        Event::Save(Some(args[0].to_string()), false)
                    })
                }
            }
            "undo" => events.push(Event::Undo),
            "commit" => events.push(Event::Commit),
            "redo" => events.push(Event::Redo),
            "quit" => events.push(Event::Quit(args.len() != 0 && args[0] == "!")),
            "overwrite" => events.push(if args.is_empty() {
                Event::Overwrite(rows.to_vec(), vec![Row::from("")])
            } else {
                Event::Overwrite(
                    rows.to_vec(),
                    args[0].split('\n').map(Row::from).collect::<Vec<_>>(),
                )
            }),
            _ => return None,
        }
    }
    Some(events)
}

fn delete_command(args: &[&str], cursor: &Position, rows: &[Row]) -> Option<Vec<Event>> {
    // Handle the delete command (complicated)
    let mut events = vec![];
    if args.is_empty() {
        return None;
    } else if let Ok(line) = args[0].parse::<i128>() {
        let ind;
        if line.is_negative() {
            if cursor.y as i128 + line >= 0 {
                ind = (cursor.y as i128 + line) as usize;
            } else {
                ind = 0;
            }
        } else if cursor.y as i128 + line < rows.len() as i128 {
            ind = (cursor.y as i128 + line) as usize;
        } else {
            ind = rows.len().saturating_sub(1);
        }
        events.push(Event::DeleteLine(ind, Box::new(rows[ind].clone())));
    } else {
        match *args.get(0).unwrap_or(&"") {
            "~" => {
                let mut c = cursor.x as i128;
                let chars: Vec<char> = rows[cursor.y].string.chars().collect();
                while c >= 0 && chars[c as usize] != ' ' {
                    events.push(Event::BackspaceMid(
                        Position {
                            x: c as usize,
                            y: cursor.y,
                        },
                        chars[c as usize],
                    ));
                    c -= 1;
                }
            }
            "$" => events.push(Event::UpdateLine(
                cursor.y,
                Box::new(rows[cursor.y].clone()),
                Box::new(Row::from("")),
            )),
            _ => return None,
        }
    }
    Some(events)
}
