/*
    Oxa.rs - Tools for parsing and lexing the Ox Assembly format

    Oxa is an interpreted specific purpose language inspired by x86 assembly
    It is used to write macros for the editor that make editing text painless
    It is also used for writing commands in the "macro mode" when editing

    An example usage could be writing a macro to delete the current line
*/
use crate::config::Reader;
use crate::undo::BankType;
use crate::{Direction, Event, Position, Row};

pub fn interpret_line(
    line: &str,
    cursor: &Position,
    graphemes: usize,
    rows: &[Row],
    config: &Reader,
) -> Option<Vec<Event>> {
    // Take an instruction of Oxa and interpret it
    let mut events = vec![];
    let mut line = line.trim_start_matches(' ').split(' ');
    let mut start = line.next();
    let times;
    if let Ok(repeat) = start.unwrap_or_default().parse::<usize>() {
        times = repeat;
        start = line.next();
    } else {
        times = 1;
    }
    if let Some(instruction) = start {
        let args: Vec<&str> = line.collect();
        for _ in 0..times {
            match instruction {
                "goto" => match args.len() {
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
                        if let (Ok(x), Ok(y)) = (args[0].parse::<usize>(), args[1].parse::<usize>())
                        {
                            events.push(Event::GotoCursor(Position {
                                x: x.saturating_sub(1),
                                y: y.saturating_sub(1),
                            }));
                        } else {
                            return None;
                        }
                    }
                    _ => return None,
                },
                "move" => {
                    if args.len() == 2 {
                        let magnitude: usize = args[0].parse().unwrap_or_default();
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
                    } else {
                        return None;
                    }
                }
                "put" => {
                    if args[0] == "\\t" {
                        for _ in 0..config.general.tab_width {
                            events.push(Event::Insertion(*cursor, ' '));
                        }
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
                }
                "delete" => {
                    if let Some(mut evts) = delete_command(&args, cursor, graphemes, rows) {
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
                    } else {
                        events.push(Event::Save(None, false))
                    }
                }
                "undo" => events.push(Event::Undo),
                "commit" => events.push(Event::Commit),
                "redo" => events.push(Event::Redo),
                "quit" => {
                    events.push(if args.contains(&"*") {
                        Event::QuitAll(args.contains(&"!"))
                    } else {
                        Event::Quit(args.contains(&"!"))
                    });
                }
                "overwrite" => events.push(if args.is_empty() {
                    Event::Overwrite(rows.to_vec(), vec![Row::from("")])
                } else {
                    Event::Overwrite(
                        rows.to_vec(),
                        args.join(" ")
                            .split("\\n")
                            .map(Row::from)
                            .collect::<Vec<_>>(),
                    )
                }),
                "prev" => events.push(Event::PrevTab),
                "next" => events.push(Event::NextTab),
                "set" => {
                    if args.is_empty() {
                        events.push(Event::UpdateLine(
                            cursor.y.saturating_sub(1),
                            Box::new(rows[cursor.y.saturating_sub(1)].clone()),
                            Box::new(Row::from("")),
                        ));
                    } else {
                        events.push(Event::UpdateLine(
                            cursor.y.saturating_sub(1),
                            Box::new(rows[cursor.y.saturating_sub(1)].clone()),
                            Box::new(Row::from(args.join(" ").as_str())),
                        ));
                    }
                }
                "line" => {
                    if !args.is_empty() {
                        match args[0] {
                            "below" => events.push(Event::InsertLineBelow(cursor.y)),
                            "above" => events.push(Event::InsertLineAbove(cursor.y)),
                            _ => return None,
                        }
                    } else {
                        return None;
                    }
                }
                "split" => events.push(Event::SplitDown(*cursor)),
                "splice" => events.push(Event::SpliceUp(*cursor)),
                "store" => {
                    if args.len() == 2 {
                        if let Ok(bank) = args[1].parse::<usize>() {
                            match args[0] {
                                "cursor" => events.push(Event::Store(BankType::Cursor, bank)),
                                "line" => events.push(Event::Store(BankType::Line, bank)),
                                _ => return None,
                            }
                        } else {
                            return None;
                        }
                    } else {
                        return None;
                    }
                }
                "load" => {
                    if args.len() == 2 {
                        if let Ok(bank) = args[1].parse::<usize>() {
                            match args[0] {
                                "cursor" => events.push(Event::Load(BankType::Cursor, bank)),
                                "line" => events.push(Event::Load(BankType::Line, bank)),
                                _ => return None,
                            }
                        } else {
                            return None;
                        }
                    } else {
                        return None;
                    }
                }
                _ => return None,
            }
        }
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
        events.push(Event::DeleteLine(
            ind.saturating_sub(1),
            Box::new(rows[ind.saturating_sub(1)].clone()),
        ));
    } else {
        return None;
    }
    Some(events)
}
