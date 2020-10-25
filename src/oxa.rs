/* 
    Oxa.rs - Tools for parsing the Ox Assembly format

    Oxa is an interpreted specific purpose language inspired by x86 assembly
    It is used to write macros for the editor that make editing text painless
    It is also used for writing commands in the "macro mode" when editing
    
    An example usage could be writing a macro to delete the current line
*/
use crate::undo::Event;
use crate::Position;

pub fn interpret_line(line: &str, cursor: &Position) -> Option<Vec<Event>> {
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
            },
            "move" => {
                if args.len() == 1 {
                    if let Ok(y) = args[0].parse() {
                        events.push(Event::MoveCursor(0, y));
                    } else {
                        return None;
                    }
                } else {
                    if let (Ok(x), Ok(y)) = (args[0].parse(), args[1].parse()) {
                        events.push(Event::MoveCursor(x, y))
                    } else {
                        return None;
                    }
                }
            },
            "put" => {
                if args[0] == "\n" {
                    events.push(Event::ReturnEnd(*cursor));
                } else {
                    for (c, ch) in args.join(" ").chars().enumerate() {
                        events.push(Event::InsertMid(Position { 
                            x: cursor.x.saturating_add(c), 
                            y: cursor.y 
                        }, ch))
                    }
                }
            },
            _ => return None,
        }
    }
    Some(events)
}

