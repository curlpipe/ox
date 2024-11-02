use kaolinite::regex;
#[cfg(test)]
use kaolinite::{document::*, event::*, map::*, searching::*, utils::*};
use std::io::Write;
use std::ops::{Range, RangeBounds};
use sugars::hmap;

macro_rules! st {
    ($e:expr) => {
        $e.to_string()
    };
}

#[test]
fn filetypes() {
    assert_eq!(filetype("asm"), Some(st!("Assembly")));
    assert_eq!(filetype("py"), Some(st!("Python")));
    assert_eq!(filetype("php"), Some(st!("PHP")));
    assert_eq!(filetype("txt"), Some(st!("Plain Text")));
    assert_eq!(filetype("vrx"), Some(st!("GLSL")));
    assert_eq!(filetype("zsh"), Some(st!("Zsh")));
    assert_eq!(filetype("abcd"), None);
    assert_eq!(icon("reStructuredText"), st!("󰊄"));
    assert_eq!(icon("abcd"), st!("󰈙 "));
    assert_eq!(modeline("#!/usr/bin/env ruby"), Some("rb"));
    assert_eq!(modeline("#!/usr/bin/python3"), Some("py"));
    assert_eq!(modeline("#! /usr/bin/env python3"), Some("py"));
    assert_eq!(modeline("#!/usr/bin/env foo"), None);
    assert_eq!(modeline("testing"), None);
}

#[test]
fn regex() {
    let reg = regex!("a+b*c");
    assert_eq!(reg.captures("aaac").as_slice().len(), 1);
    let reg = regex!(r"\\{\{\{{}");
    assert_eq!(reg.as_str(), "a^");
    assert_eq!(reg.captures("abd").as_slice().len(), 0);
}

#[test]
fn loc_and_size() {
    let loc1 = Loc { x: 4, y: 2 };
    let loc2 = Loc::at(4, 2);
    assert_eq!(loc1, loc2);
    let size1 = Size { w: 4, h: 2 };
    let size2 = Size::is(4, 2);
    assert_eq!(size1, size2);
}

#[test]
fn trimming() {
    let line = st!("\thi 你n好!");
    assert_eq!(trim(&line, 0, 3, 4), st!("   "));
    assert_eq!(trim(&line, 0, 6, 4), st!("    hi"));
    assert_eq!(trim(&line, 0, 6, 2), st!("  hi  "));
    assert_eq!(trim(&line, 0, 8, 4), st!("    hi  "));
    assert_eq!(trim(&line, 1, 8, 4), st!("   hi 你"));
    assert_eq!(trim(&line, 7, 8, 4), st!("你n好!"));
    assert_eq!(trim(&line, 8, 5, 4), st!(" n好!"));
    assert_eq!(trim(&line, 8, 3, 4), st!(" n "));
    assert_eq!(trim(&line, 100, 3, 4), st!(""));
}

#[test]
fn ranges() {
    let ranges = vec![
        get_range(&(1..8), 0, 100),
        get_range(&(1..=8), 0, 100),
        get_range(&(1..), 0, 100),
        get_range(&(..8), 0, 100),
    ];
    assert_eq!(ranges, vec![(1, 7), (1, 8), (1, 100), (0, 7)],);
}

#[test]
fn widths() {
    assert_eq!(width("", 4), 0);
    assert_eq!(width("hello", 2), 5);
    assert_eq!(width("\t", 3), 3);
    assert_eq!(width("\t\t", 1), 2);
    assert_eq!(width("你", 5), 2);
    assert_eq!(width("\t你", 7), 9);
    assert_eq!(width("你s你s你你你s你你你", 1), 19);
    assert_eq!(width("\trs你t你arsd", 4), 15);
}

#[test]
fn tab_boundaries() {
    // Forward
    assert_eq!(tab_boundaries_forward("hello", 4), vec![]);
    assert_eq!(tab_boundaries_forward("   hello", 3), vec![0]);
    assert_eq!(tab_boundaries_forward("    hello", 2), vec![0, 2]);
    assert_eq!(tab_boundaries_forward("     hello     hello2", 5), vec![0]);
    assert_eq!(
        tab_boundaries_forward("            hello      hello", 6),
        vec![0, 6]
    );
    assert_eq!(tab_boundaries_forward(" 你 ", 1), vec![0]);
    // Backward
    assert_eq!(tab_boundaries_backward("hello", 4), vec![]);
    assert_eq!(tab_boundaries_backward("   hello", 3), vec![3]);
    assert_eq!(tab_boundaries_backward("    hello", 2), vec![2, 4]);
    assert_eq!(tab_boundaries_backward("     hello     hello2", 5), vec![5]);
    assert_eq!(
        tab_boundaries_backward("            hello      hello", 6),
        vec![6, 12]
    );
    assert_eq!(tab_boundaries_backward(" 你 ", 1), vec![1]);
}

#[test]
fn searching() {
    // Basic URL grabber test
    let mut url_grabber = Searcher::new(r"\b(?:https?://|www\.)\S+\b");
    let text = st!("click here: https://github.com/curlpipe/ox to see more information or visit https://curlpipe.github.io");
    assert_eq!(
        url_grabber.lfind(&text),
        Some(Match {
            loc: Loc { x: 12, y: 0 },
            text: st!("https://github.com/curlpipe/ox")
        })
    );
    assert_eq!(
        url_grabber.rfind(&text),
        Some(Match {
            loc: Loc { x: 76, y: 0 },
            text: st!("https://curlpipe.github.io")
        })
    );
    let text = st!("there are no links here!");
    assert_eq!(url_grabber.lfind(&text), None);
    assert_eq!(url_grabber.rfind(&text), None);
    // Unicode handling
    let mut greeting_finder = Searcher::new("你好");
    let text = st!("Hello is 你好 in Mandarin: 你好");
    assert_eq!(Searcher::raw_to_char(0, &text), 0);
    assert_eq!(Searcher::raw_to_char(15, &text), 11);
    assert_eq!(Searcher::raw_to_char(16, &text), 12);
    assert_eq!(
        greeting_finder.lfind(&text),
        Some(Match {
            loc: Loc { x: 9, y: 0 },
            text: st!("你好")
        })
    );
    assert_eq!(
        greeting_finder.rfind(&text),
        Some(Match {
            loc: Loc { x: 25, y: 0 },
            text: st!("你好")
        })
    );
}

#[test]
fn char_mapping() {
    let mut test1_map = CharMap::new(hmap! { 0 => vec![]});
    let mut test2_map = CharMap::new(hmap! { 794385 => vec![(1, 1), (5, 4)] });
    let mut test3_map = CharMap::new(hmap! { 2 => vec![(1, 1), (3, 2), (6, 4), (8, 5)] });
    let mut test4_map = CharMap::new(hmap! { 5 => vec![(0, 0)] });
    let results = vec![
        test1_map.count(&Loc::at(0, 0), true).unwrap(),
        test1_map.count(&Loc::at(3, 0), false).unwrap(),
        test2_map.count(&Loc::at(3, 794385), true).unwrap(),
        test2_map.count(&Loc::at(6, 794385), false).unwrap(),
        test3_map.count(&Loc::at(6, 2), true).unwrap(),
        test3_map.count(&Loc::at(6, 2), false).unwrap(),
    ];
    assert_eq!(results, vec![0, 0, 1, 2, 2, 4]);
    // Add
    test1_map.add(1, (4, 7));
    test2_map.add(794385, (9, 7));
    assert_eq!(test1_map.get(1).unwrap()[0], (4, 7));
    assert_eq!(test2_map.get(794385).unwrap()[2], (9, 7));
    // Insert
    test3_map.insert(1, vec![(0, 0)]);
    assert_eq!(test3_map.get(1).unwrap()[0], (0, 0));
    // Delete
    test3_map.delete(1);
    assert!(test3_map.get(1).is_none());
    // Contains
    assert!(!test3_map.contains(1));
    assert!(test3_map.contains(2));
    // Splice
    test3_map.splice(&Loc::at(0, 2), 2, vec![(4, 4), (6, 5)]);
    test1_map.splice(&Loc::at(0, 5), 12, vec![(5, 5), (7, 6), (8, 7)]);
    assert_eq!(
        test3_map.get(2).unwrap(),
        &vec![(1, 1), (3, 2), (4, 4), (6, 5), (6, 4), (8, 5)]
    );
    assert_eq!(test1_map.get(5).unwrap(), &vec![(5, 5), (7, 6), (8, 7)]);
    // Shift_insertion
    assert_eq!(test2_map.shift_insertion(&Loc::at(2, 0), "\to教", 4), 0);
    assert_eq!(
        test2_map.get(794385).unwrap(),
        &vec![(1, 1), (5, 4), (9, 7)]
    );
    assert_eq!(test2_map.get(0), None);
    assert_eq!(
        test2_map.shift_insertion(&Loc::at(2, 794385), "\to教", 4),
        1
    );
    assert_eq!(
        test2_map.get(794385).unwrap(),
        &vec![(1, 1), (12, 7), (16, 10)]
    );
    // Shift_deletion
    test2_map.shift_deletion(&Loc::at(0, 0), (2, 5), "\to教", 4);
    assert_eq!(test2_map.get(0), None);
    assert_eq!(
        test2_map.get(794385).unwrap(),
        &vec![(1, 1), (12, 7), (16, 10)]
    );
    test2_map.shift_deletion(&Loc::at(0, 794385), (2, 5), "\to教", 4);
    assert_eq!(
        test2_map.get(794385).unwrap(),
        &vec![(1, 1), (5, 4), (9, 7)]
    );
    test4_map.shift_deletion(&Loc::at(0, 5), (0, 1), "a", 4);
    assert_eq!(test4_map.get(5), None);
    // Shift_up
    let temp = test2_map.clone();
    test2_map.shift_up(4);
    assert_eq!(temp.get(794386), test2_map.get(794385));
    assert_eq!(temp.get(1), test2_map.get(1));
    // Shift_down
    test2_map.shift_down(4);
    assert_eq!(temp, test2_map);
    // Form_map
    let test_data_string1 = "".to_string();
    let test_data_string2 = "\t\t蔼教\t案 srtin".to_string();
    assert_eq!(form_map(&test_data_string1, 4), (vec![], vec![]));
    assert_eq!(
        form_map(&test_data_string2, 4),
        (
            vec![(8, 2), (10, 3), (16, 5)],
            vec![(0, 0), (4, 1), (12, 4)]
        )
    );
    assert_eq!(form_map(&test_data_string1, 3), (vec![], vec![]));
    assert_eq!(
        form_map(&test_data_string2, 5),
        (
            vec![(10, 2), (12, 3), (19, 5)],
            vec![(0, 0), (5, 1), (14, 4)]
        )
    );
}

#[test]
fn events() {
    let ev = vec![
        Event::Insert(Loc { x: 0, y: 0 }, st!("a")),
        Event::Delete(Loc { x: 5, y: 4 }, st!("b")),
        Event::InsertLine(0, st!("hello")),
        Event::DeleteLine(1, st!("testing")),
        Event::SplitDown(Loc { x: 5, y: 0 }),
        Event::SpliceUp(Loc { x: 0, y: 3 }),
    ];
    let mut locs = vec![];
    ev.iter().for_each(|e| locs.push(e.clone().loc()));
    assert_eq!(
        locs,
        vec![
            Loc { x: 0, y: 0 },
            Loc { x: 5, y: 4 },
            Loc { x: 0, y: 0 },
            Loc { x: 0, y: 1 },
            Loc { x: 5, y: 0 },
            Loc { x: 0, y: 3 },
        ],
    );
    let mut rev = vec![];
    ev.iter().for_each(|e| rev.push(e.clone().reverse()));
    assert_eq!(
        rev,
        vec![
            Event::Delete(Loc { x: 0, y: 0 }, st!("a")),
            Event::Insert(Loc { x: 5, y: 4 }, st!("b")),
            Event::DeleteLine(0, st!("hello")),
            Event::InsertLine(1, st!("testing")),
            Event::SpliceUp(Loc { x: 5, y: 0 }),
            Event::SplitDown(Loc { x: 0, y: 3 }),
        ],
    );
    assert!(Event::Insert(Loc { x: 0, y: 1 }, st!("Test"))
        .same_type(&Event::Insert(Loc { x: 2, y: 3 }, st!("334"))));
    assert!(!Event::Delete(Loc { x: 0, y: 1 }, st!("Test"))
        .same_type(&Event::Insert(Loc { x: 2, y: 3 }, st!("334"))));
}

#[test]
fn errors() {
    let test1 = Error::OutOfRange;
    let result = format!("{:?}", test1);
    assert_eq!(result, "OutOfRange".to_string());
}

#[test]
fn document_disks() {
    // Standard test
    let mut doc = Document::open(Size::is(100, 10), "tests/data/saving.txt").unwrap();
    doc.load_to(100);
    doc.delete_line(0);
    doc.insert_line(0, st!("this document is modified"));
    doc.save();
    std::thread::sleep(std::time::Duration::from_millis(100));
    let result = std::fs::read_to_string("tests/data/saving.txt").unwrap();
    // Restore original state of the document
    let mut file = std::fs::File::create("tests/data/saving.txt").unwrap();
    file.write_all(b"this document is original\n").unwrap();
    // Validate
    assert_eq!(result, st!("this document is modified\n"));
    // Error cases
    let mut doc = Document::new(Size::is(100, 10));
    assert!(doc.save().is_err());
    let mut doc = Document::new(Size::is(100, 10));
    doc.info.read_only = true;
    assert!(doc.save().is_err());
    // Save as
    assert!(doc.save_as("tests/data/ghost.txt").is_err());
    doc.info.read_only = false;
    assert!(doc.save_as("tests/data/ghost.txt").is_ok());
    // Clean up and verify ghost exists
    let result = std::fs::read_to_string("tests/data/ghost.txt").unwrap();
    std::fs::remove_file("tests/data/ghost.txt").unwrap();
    assert_eq!(result, st!("\n"));
}

#[test]
fn document_insertion() {
    let mut doc = Document::open(Size::is(100, 10), "tests/data/unicode.txt").unwrap();
    doc.load_to(100);
    doc.exe(Event::Insert(Loc { x: 5, y: 0 }, st!("hello")));
    assert_eq!(doc.line(0), Some(st!("    你hello好")));
    assert_eq!(doc.dbl_map.get(3), Some(&vec![(4, 1), (6, 2)]));
    doc.exe(Event::Insert(Loc { x: 3, y: 3 }, st!("\t你你")));
    assert_eq!(doc.line(3), Some(st!("\t你好\t你你")));
    assert_eq!(
        doc.dbl_map.get(3),
        Some(&vec![(4, 1), (6, 2), (12, 4), (14, 5)])
    );
    doc.exe(Event::Insert(
        Loc { x: 0, y: 6 },
        st!("\thello, world: 你好"),
    ));
    assert_eq!(doc.line(6), None);
    doc.exe(Event::Insert(Loc { x: 10000, y: 0 }, st!(" ")));
    assert_eq!(doc.line(0), Some(st!("    你hello好")));
}

#[test]
fn document_deletion() {
    let mut doc = Document::open(Size::is(100, 10), "tests/data/unicode.txt").unwrap();
    doc.load_to(100);
    doc.exe(Event::Delete(Loc { x: 4, y: 0 }, st!("你")));
    assert_eq!(doc.line(0), Some(st!("    好")));
    assert_eq!(doc.dbl_map.get(3), Some(&vec![(4, 1), (6, 2)]));
    doc.exe(Event::Delete(Loc { x: 1, y: 3 }, st!("你")));
    assert_eq!(doc.line(3), Some(st!("\t好")));
    assert_eq!(doc.dbl_map.get(3), Some(&vec![(4, 1)]));
    doc.exe(Event::Delete(
        Loc { x: 0, y: 6 },
        st!("\thello, world: 你好"),
    ));
    assert_eq!(doc.line(6), None);
    doc.exe(Event::Delete(Loc { x: 3, y: 0 }, st!(" ")));
    assert_eq!(doc.line(0), Some(st!("好")));
    doc.exe(Event::Delete(Loc { x: 10000, y: 0 }, st!(" ")));
    assert_eq!(doc.line(0), Some(st!("好")));
    // Word deleting
    doc.exe(Event::InsertLine(1, st!("    hello -world---")));
    doc.move_to(&Loc { x: 0, y: 1 });
    doc.delete_word();
    assert_eq!(doc.line(1).unwrap(), st!("    hello -world---"));
    doc.move_to(&Loc { x: 4, y: 1 });
    doc.delete_word();
    assert_eq!(doc.line(1).unwrap(), st!("hello -world---"));
    doc.move_to(&Loc { x: 4, y: 1 });
    doc.delete_word();
    assert_eq!(doc.line(1).unwrap(), st!("o -world---"));
    doc.move_to(&Loc { x: 1, y: 1 });
    doc.delete_word();
    assert_eq!(doc.line(1).unwrap(), st!(" -world---"));
    doc.move_to(&Loc { x: 8, y: 1 });
    doc.delete_word();
    assert_eq!(doc.line(1).unwrap(), st!(" -world--"));
    doc.move_to(&Loc { x: 1, y: 1 });
    doc.delete_word();
    assert_eq!(doc.line(1).unwrap(), st!("-world--"));
    doc.move_to(&Loc { x: 1, y: 1 });
    doc.delete_word();
    assert_eq!(doc.line(1).unwrap(), st!("world--"));
    doc.exe(Event::InsertLine(1, st!("    hello -world---")));
    doc.move_to(&Loc { x: 11, y: 1 });
    doc.delete_word();
    assert_eq!(doc.line(1).unwrap(), st!("    world---"));
    doc.exe(Event::InsertLine(1, st!("match => this")));
    doc.move_to(&Loc { x: 8, y: 1 });
    doc.delete_word();
    assert_eq!(doc.line(1).unwrap(), st!(" this"));
}

#[test]
fn document_undo_redo() {
    let mut doc = Document::open(Size::is(100, 10), "tests/data/unicode.txt").unwrap();
    doc.load_to(100);
    assert!(doc.event_mgmt.undo(doc.take_snapshot()).is_none());
    assert!(doc.event_mgmt.with_disk(&doc.take_snapshot()));
    doc.event_mgmt.force_not_with_disk = true;
    assert!(!doc.event_mgmt.with_disk(&doc.take_snapshot()));
    doc.event_mgmt.force_not_with_disk = false;
    assert!(doc.event_mgmt.with_disk(&doc.take_snapshot()));
    assert!(doc.event_mgmt.undo(doc.take_snapshot()).is_none());
    assert!(doc.redo().is_ok());
    assert!(doc.event_mgmt.with_disk(&doc.take_snapshot()));
    doc.exe(Event::InsertLine(0, st!("hello你bye好hello")));
    doc.exe(Event::Delete(Loc { x: 0, y: 2 }, st!("\t")));
    doc.exe(Event::Insert(Loc { x: 3, y: 2 }, st!("a")));
    assert!(!doc.event_mgmt.with_disk(&doc.take_snapshot()));
    doc.commit();
    assert!(!doc.event_mgmt.with_disk(&doc.take_snapshot()));
    assert!(doc.undo().is_ok());
    assert!(doc.event_mgmt.with_disk(&doc.take_snapshot()));
    assert_eq!(doc.line(0), Some(st!("    你好")));
    assert_eq!(doc.line(1), Some(st!("\thello")));
    assert_eq!(doc.line(2), Some(st!("    hello")));
    assert!(doc.redo().is_ok());
    assert!(!doc.event_mgmt.with_disk(&doc.take_snapshot()));
    assert_eq!(doc.line(0), Some(st!("hello你bye好hello")));
    assert_eq!(doc.line(2), Some(st!("helalo")));
    assert!(!doc.event_mgmt.with_disk(&doc.take_snapshot()));
    doc.event_mgmt.disk_write(&doc.take_snapshot());
    assert!(doc.event_mgmt.with_disk(&doc.take_snapshot()));
    let mut doc = Document::open(Size::is(100, 10), "tests/data/unicode.txt").unwrap();
    doc.load_to(100);
    assert!(doc.event_mgmt.with_disk(&doc.take_snapshot()));
    doc.exe(Event::InsertLine(0, st!("hello你bye好hello")));
    assert!(doc.event_mgmt.with_disk(&doc.take_snapshot()));
}

#[test]
fn document_moving() {
    let mut doc = Document::open(Size::is(10, 10), "tests/data/big.txt").unwrap();
    doc.load_to(10);
    // Check moving down
    for loaded in 0..100 {
        assert_eq!(doc.move_down(), Status::None);
        assert_eq!(doc.cursor.loc.y, 1 + loaded);
        assert_eq!(
            doc.offset.y,
            if loaded < 9 {
                0
            } else {
                (1 + loaded).saturating_sub(9)
            }
        );
        assert!(doc.info.loaded_to >= loaded);
    }
    assert_eq!(doc.move_down(), Status::EndOfFile);
    // Check moving up
    for loaded in 0..100 {
        assert_eq!(doc.move_up(), Status::None);
        assert_eq!(doc.cursor.loc.y, 99 - loaded);
        assert_eq!(doc.offset.y, if loaded < 9 { 91 } else { 99 - loaded });
    }
    assert_eq!(doc.move_up(), Status::StartOfFile);
    // Check cursor "stickiness" & goto & double width straddling attempts
    let mut doc = Document::open(Size::is(100, 10), "tests/data/unicode.txt").unwrap();
    doc.load_to(10);
    doc.exe(Event::InsertLine(5, st!("hello你bye")));
    doc.move_to(&Loc { x: 4, y: 1 });
    doc.old_cursor = 7;
    assert_eq!(doc.char_loc(), Loc { x: 4, y: 1 });
    assert_eq!(doc.loc(), Loc { x: 7, y: 1 });
    doc.move_down();
    doc.move_down();
    assert_eq!(doc.char_loc(), Loc { x: 2, y: 3 });
    assert_eq!(doc.loc(), Loc { x: 6, y: 3 });
    doc.move_to(&Loc { x: 1000, y: 4 });
    doc.old_cursor = 19;
    assert_eq!(doc.char_loc(), Loc { x: 17, y: 4 });
    assert_eq!(doc.loc(), Loc { x: 19, y: 4 });
    doc.move_down();
    assert_eq!(doc.char_loc(), Loc { x: 9, y: 5 });
    assert_eq!(doc.loc(), Loc { x: 10, y: 5 });
    doc.move_up();
    assert_eq!(doc.char_loc(), Loc { x: 17, y: 4 });
    assert_eq!(doc.loc(), Loc { x: 19, y: 4 });
    // Moving left and right & cursor "stickiness"
    let mut doc = Document::open(Size::is(10, 10), "tests/data/big.txt").unwrap();
    doc.load_to(10);
    for _ in 0..9 {
        doc.move_right();
    }
    assert_eq!(doc.offset.x, 0);
    doc.move_right();
    assert_eq!(doc.offset.x, 1);
    assert_eq!(doc.cursor.loc.x, 10);
    for _ in 0..21 {
        doc.move_right();
    }
    assert_eq!(doc.move_right(), Status::EndOfLine);
    for _ in 0..21 {
        doc.move_left();
    }
    doc.move_left();
    assert_eq!(doc.offset.x, 9);
    assert_eq!(doc.cursor.loc.x, 9);
    for _ in 0..9 {
        doc.move_left();
    }
    assert_eq!(doc.move_left(), Status::StartOfLine);
    doc.exe(Event::InsertLine(2, st!("        tab line")));
    doc.move_to(&Loc { x: 0, y: 2 });
    doc.move_right();
    assert_eq!(doc.loc(), Loc { x: 4, y: 2 });
    assert_eq!(doc.char_loc(), Loc { x: 4, y: 2 });
    doc.move_right();
    assert_eq!(doc.loc(), Loc { x: 8, y: 2 });
    assert_eq!(doc.char_loc(), Loc { x: 8, y: 2 });
    doc.move_left();
    assert_eq!(doc.loc(), Loc { x: 4, y: 2 });
    assert_eq!(doc.char_loc(), Loc { x: 4, y: 2 });
    doc.move_left();
    assert_eq!(doc.loc(), Loc { x: 0, y: 2 });
    assert_eq!(doc.char_loc(), Loc { x: 0, y: 2 });
    // Test unicode
    let mut doc = Document::open(Size::is(100, 10), "tests/data/unicode.txt").unwrap();
    doc.load_to(10);
    doc.move_right();
    assert_eq!(doc.loc(), Loc { x: 4, y: 0 });
    assert_eq!(doc.char_loc(), Loc { x: 4, y: 0 });
    doc.move_right();
    assert_eq!(doc.loc(), Loc { x: 6, y: 0 });
    assert_eq!(doc.char_loc(), Loc { x: 5, y: 0 });
    doc.move_left();
    assert_eq!(doc.loc(), Loc { x: 4, y: 0 });
    assert_eq!(doc.char_loc(), Loc { x: 4, y: 0 });
    // Specialist moving methods
    doc.move_to(&Loc { x: 0, y: 0 });
    doc.move_end();
    assert_eq!(doc.loc(), Loc { x: 8, y: 0 });
    assert_eq!(doc.char_loc(), Loc { x: 6, y: 0 });
    assert_eq!(doc.old_cursor, 8);
    doc.move_home();
    assert_eq!(doc.loc(), Loc { x: 0, y: 0 });
    assert_eq!(doc.char_loc(), Loc { x: 0, y: 0 });
    assert_eq!(doc.old_cursor, 0);
    let mut doc = Document::open(Size::is(10, 10), "tests/data/big.txt").unwrap();
    doc.load_to(10);
    doc.move_right();
    assert_eq!(doc.char_loc(), Loc { x: 1, y: 0 });
    assert_eq!(doc.old_cursor, 1);
    doc.move_bottom();
    assert_eq!(
        doc.char_loc(),
        Loc {
            x: 0,
            y: doc.len_lines()
        }
    );
    assert_eq!(doc.old_cursor, 0);
    assert_eq!(doc.info.loaded_to, doc.len_lines() + 1);
    doc.move_top();
    assert_eq!(doc.char_loc(), Loc { x: 0, y: 0 });
    assert_eq!(doc.old_cursor, 0);
    doc.select_bottom();
    assert_eq!(
        doc.char_loc(),
        Loc {
            x: 0,
            y: doc.len_lines()
        }
    );
    assert_eq!(doc.old_cursor, 0);
    assert_eq!(doc.info.loaded_to, doc.len_lines() + 1);
    doc.select_top();
    assert_eq!(doc.char_loc(), Loc { x: 0, y: 0 });
    assert_eq!(doc.old_cursor, 0);
    doc.move_page_down();
    assert_eq!(doc.loc(), Loc { x: 0, y: 10 });
    doc.move_page_down();
    assert_eq!(doc.loc(), Loc { x: 0, y: 20 });
    doc.move_page_up();
    assert_eq!(doc.loc(), Loc { x: 0, y: 10 });
    doc.move_page_up();
    assert_eq!(doc.loc(), Loc { x: 0, y: 0 });
    // Test word moving
    doc.exe(Event::InsertLine(10, st!("these are words this.is.code()")));
    doc.move_to(&Loc { x: 0, y: 10 });
    doc.move_next_word();
    assert_eq!(doc.loc(), Loc { x: 6, y: 10 });
    doc.move_next_word();
    assert_eq!(doc.loc(), Loc { x: 10, y: 10 });
    doc.move_next_word();
    assert_eq!(doc.loc(), Loc { x: 16, y: 10 });
    doc.move_next_word();
    assert_eq!(doc.loc(), Loc { x: 20, y: 10 });
    doc.move_next_word();
    assert_eq!(doc.loc(), Loc { x: 21, y: 10 });
    doc.move_next_word();
    assert_eq!(doc.loc(), Loc { x: 23, y: 10 });
    doc.move_next_word();
    assert_eq!(doc.loc(), Loc { x: 24, y: 10 });
    doc.move_next_word();
    assert_eq!(doc.loc(), Loc { x: 28, y: 10 });
    doc.move_next_word();
    assert_eq!(doc.loc(), Loc { x: 30, y: 10 });
    assert_eq!(doc.move_next_word(), Status::EndOfLine);
    doc.move_prev_word();
    assert_eq!(doc.loc(), Loc { x: 28, y: 10 });
    doc.move_prev_word();
    assert_eq!(doc.loc(), Loc { x: 24, y: 10 });
    doc.move_prev_word();
    assert_eq!(doc.loc(), Loc { x: 23, y: 10 });
    doc.move_prev_word();
    assert_eq!(doc.loc(), Loc { x: 21, y: 10 });
    doc.move_prev_word();
    assert_eq!(doc.loc(), Loc { x: 20, y: 10 });
    doc.move_prev_word();
    assert_eq!(doc.loc(), Loc { x: 15, y: 10 });
    doc.move_prev_word();
    assert_eq!(doc.loc(), Loc { x: 9, y: 10 });
    doc.move_prev_word();
    assert_eq!(doc.loc(), Loc { x: 5, y: 10 });
    doc.move_prev_word();
    assert_eq!(doc.loc(), Loc { x: 0, y: 10 });
    assert_eq!(doc.move_prev_word(), Status::StartOfLine);
    doc.exe(Event::InsertLine(11, st!("----test hello there----")));
    doc.move_to(&Loc { x: 7, y: 11 });
    doc.move_next_word();
    assert_eq!(doc.loc(), Loc { x: 14, y: 11 });
    doc.move_to(&Loc { x: 0, y: 11 });
    assert_eq!(doc.prev_word_close(Loc { x: 0, y: 11 }), 0);
    assert_eq!(doc.prev_word_close(Loc { x: 2, y: 11 }), 0);
    assert_eq!(doc.prev_word_close(Loc { x: 9, y: 11 }), 4);
    assert_eq!(doc.prev_word_close(Loc { x: 8, y: 11 }), 0);
    assert_eq!(doc.prev_word_close(Loc { x: 14, y: 11 }), 8);
    assert_eq!(doc.prev_word_close(Loc { x: 20, y: 11 }), 14);
    assert_eq!(doc.prev_word_close(Loc { x: 24, y: 11 }), 15);
    assert_eq!(doc.prev_word_close(Loc { x: 22, y: 11 }), 15);
    assert_eq!(doc.next_word_close(Loc { x: 4, y: 11 }), 4);
    assert_eq!(doc.next_word_close(Loc { x: 1, y: 11 }), 4);
    assert_eq!(doc.next_word_close(Loc { x: 8, y: 11 }), 8);
    assert_eq!(doc.next_word_close(Loc { x: 9, y: 11 }), 9);
    assert_eq!(doc.next_word_close(Loc { x: 14, y: 11 }), 14);
    assert_eq!(doc.next_word_close(Loc { x: 20, y: 11 }), 20);
    assert_eq!(doc.next_word_close(Loc { x: 24, y: 11 }), 24);
    assert_eq!(doc.next_word_close(Loc { x: 22, y: 11 }), 24);
    doc.move_to(&Loc {
        x: 0,
        y: 10000000000,
    });
    assert_eq!(
        doc.loc(),
        Loc {
            x: 0,
            y: doc.len_lines()
        }
    );
}

#[test]
fn document_selection() {
    let mut doc = Document::open(Size::is(10, 10), "tests/data/big.txt").unwrap();
    doc.load_to(10);
    assert!(doc.is_selection_empty());
    doc.select_to(&Loc { x: 1, y: 1 });
    assert!(!doc.is_selection_empty());
    assert_eq!(
        doc.selection_loc_bound(),
        (Loc { x: 0, y: 0 }, Loc { x: 1, y: 1 })
    );
    assert!(doc.is_loc_selected(Loc { x: 0, y: 1 }));
    assert!(doc.is_loc_selected(Loc { x: 0, y: 0 }));
    assert!(doc.is_loc_selected(Loc { x: 2, y: 0 }));
    assert!(doc.is_loc_selected(Loc { x: 3, y: 0 }));
    assert!(!doc.is_loc_selected(Loc { x: 0, y: 2 }));
    assert!(!doc.is_loc_selected(Loc { x: 1, y: 1 }));
    assert!(!doc.is_loc_selected(Loc { x: 0, y: 3 }));
    assert!(!doc.is_loc_selected(Loc { x: 2, y: 1 }));
    assert!(!doc.is_loc_selected(Loc { x: 3, y: 3 }));
    assert_eq!(doc.selection_range(), 0..33);
    assert_eq!(
        doc.selection_text(),
        st!("5748248337351130204990967092462\n8")
    );
    doc.remove_selection();
    assert!(doc.is_selection_empty());
    assert!(!doc.is_loc_selected(Loc { x: 0, y: 1 }));
    assert!(!doc.is_loc_selected(Loc { x: 0, y: 0 }));
    assert!(!doc.is_loc_selected(Loc { x: 2, y: 0 }));
    assert!(!doc.is_loc_selected(Loc { x: 3, y: 0 }));
    doc.select_line_at(1);
    assert_eq!(
        doc.selection_loc_bound(),
        (Loc { x: 0, y: 1 }, Loc { x: 31, y: 1 })
    );
    doc.remove_selection();
    doc.exe(Event::InsertLine(1, "hello there world".to_string()));
    doc.exe(Event::InsertLine(2, "hello".to_string()));
    doc.move_to(&Loc { x: 8, y: 1 });
    doc.select_word_at(&Loc { x: 8, y: 1 });
    assert_eq!(
        doc.selection_loc_bound(),
        (Loc { x: 6, y: 1 }, Loc { x: 11, y: 1 })
    );
    doc.remove_selection();
    doc.move_to(&Loc { x: 0, y: 2 });
    doc.select_word_at(&Loc { x: 0, y: 2 });
    assert_eq!(
        doc.selection_loc_bound(),
        (Loc { x: 0, y: 2 }, Loc { x: 5, y: 2 })
    );
    doc.select_word_at(&Loc { x: 5, y: 2 });
    assert_eq!(
        doc.selection_loc_bound(),
        (Loc { x: 0, y: 2 }, Loc { x: 5, y: 2 })
    );
}

#[test]
fn document_scrolling() {
    let mut doc = Document::open(Size::is(10, 10), "tests/data/big.txt").unwrap();
    doc.load_to(10);
    // Scrolling down
    assert_eq!(doc.offset.y, 0);
    doc.scroll_down();
    assert_eq!(doc.offset.y, 1);
    assert_eq!(doc.info.loaded_to, 11);
    // Scrolling up
    assert_eq!(doc.offset.y, 1);
    doc.scroll_up();
    assert_eq!(doc.offset.y, 0);
    assert_eq!(doc.info.loaded_to, 11);
}

#[test]
fn document_utilities() {
    let mut doc = Document::open(Size::is(100, 2), "tests/data/big.txt").unwrap();
    doc.load_to(1000);
    // File type
    assert_eq!(doc.get_file_type(), Some("txt"));
    // Cursor location
    doc.move_to(&Loc { x: 5, y: 5 });
    assert_eq!(doc.loc(), Loc { x: 5, y: 5 });
    assert_eq!(doc.char_loc(), Loc { x: 5, y: 5 });
    // Tab width
    doc.set_tab_width(2);
    assert_eq!(doc.tab_width, 2);
    // Line retrieval
    assert_eq!(doc.line(3), Some(st!("4081246106821888240886212802811")));
    assert_eq!(doc.line_trim(3, 3, 5), Some(st!("12461")));
    assert_eq!(doc.line(1000), None);
    // Loc to file position
    assert_eq!(doc.loc_to_file_pos(&Loc { x: 2, y: 1 }), 34);
    assert_eq!(doc.loc_to_file_pos(&Loc { x: 5, y: 2 }), 69);
    // Valid range
    assert!(doc.valid_range(6, 3, 0).is_err());
    // Line numbering
    assert_eq!(doc.line_number(3), st!("  4"));
    assert_eq!(doc.line_number(1000), st!("  ~"));
    assert_eq!(doc.line_number(21), st!(" 22"));
    assert_eq!(doc.line_number(100), st!("  ~"));
    assert_eq!(doc.line_number(99), st!("100"));
    // Tab detection
    doc.exe(Event::InsertLine(0, st!("\thello")));
    assert!(doc.is_tab(0, 0));
    assert!(!doc.is_tab(0, 1));
    // Width of
    assert_eq!(doc.width_of(0, 0), 2);
    // Cursor within viewport
    doc.move_to(&Loc { x: 5, y: 5 });
    assert_eq!(doc.cursor_loc_in_screen(), Some(Loc { x: 5, y: 1 }));
    doc.scroll_up();
    doc.scroll_up();
    assert_eq!(doc.cursor_loc_in_screen(), None);
    doc.scroll_down();
    doc.scroll_down();
    doc.scroll_down();
    doc.scroll_down();
    assert_eq!(doc.cursor_loc_in_screen(), None);
    doc.move_to(&Loc { x: 0, y: 5 });
    doc.offset.x = 5;
    assert_eq!(doc.cursor_loc_in_screen(), None);
}

#[test]
fn document_line_editing() {
    let mut doc = Document::open(Size::is(100, 10), "tests/data/unicode.txt").unwrap();
    doc.load_to(1000);
    // Basics
    doc.exe(Event::InsertLine(2, st!("hello你bye好hello")));
    assert_eq!(doc.line(2), Some(st!("hello你bye好hello")));
    assert_eq!(doc.len_lines(), 6);
    assert_eq!(doc.dbl_map.get(2), Some(&vec![(5, 5), (10, 9)]));
    doc.exe(Event::DeleteLine(4, st!("hello你world好hello")));
    assert_ne!(doc.line(4), Some(st!("hello你bye好hello")));
    assert_eq!(doc.len_lines(), 5);
    // Bounds checking
    doc.exe(Event::InsertLine(0, st!("hello你bye好hello")));
    assert_eq!(doc.line(0), Some(st!("hello你bye好hello")));
    doc.exe(Event::DeleteLine(0, st!("hello你bye好hello")));
    assert_ne!(doc.line(0), Some(st!("hello你bye好hello")));
    assert_eq!(doc.line(5), Some(st!("")));
    doc.exe(Event::InsertLine(5, st!("forever")));
    assert_eq!(doc.line(5), Some(st!("forever")));
    doc.exe(Event::DeleteLine(5, st!("forever")));
    assert_eq!(doc.line(5), Some(st!("")));
    // Line swapping
    let mut doc = Document::open(Size::is(100, 10), "tests/data/unicode.txt").unwrap();
    doc.load_to(1000);
    doc.swap_line_down().unwrap();
    assert_eq!(doc.line(0), Some(st!("\thello")));
    assert_eq!(doc.line(1), Some(st!("    你好")));
    doc.swap_line_up().unwrap();
    assert_eq!(doc.line(0), Some(st!("    你好")));
    assert_eq!(doc.line(1), Some(st!("\thello")));
}

#[test]
fn document_splitting_splicing() {
    let mut doc = Document::open(Size::is(100, 10), "tests/data/unicode.txt").unwrap();
    doc.load_to(1000);
    // Splitting
    assert_eq!(doc.dbl_map.get(4), Some(&vec![(5, 5), (12, 11)]));
    assert_eq!(doc.dbl_map.get(5), None);
    doc.exe(Event::SplitDown(Loc { x: 9, y: 4 }));
    assert_eq!(doc.dbl_map.get(4), Some(&vec![(5, 5)]));
    assert_eq!(doc.dbl_map.get(5), Some(&vec![(2, 2)]));
    assert_eq!(doc.line(4), Some(st!("hello你wor")));
    assert_eq!(doc.line(5), Some(st!("ld好hello")));
    assert_eq!(doc.len_lines(), 6);
    // Splicing
    doc.exe(Event::SpliceUp(Loc { x: 9, y: 4 }));
    assert_eq!(doc.line(4), Some(st!("hello你world好hello")));
    assert_eq!(doc.len_lines(), 5);
    assert_eq!(doc.dbl_map.get(4), Some(&vec![(5, 5), (12, 11)]));
    assert_eq!(doc.dbl_map.get(5), None);
}

#[test]
fn document_searching() {
    let mut doc = Document::open(Size::is(100, 1), "tests/data/unicode.txt").unwrap();
    doc.load_to(1);
    assert_eq!(
        doc.next_match("hello", 0),
        Some(Match {
            loc: Loc { x: 1, y: 1 },
            text: st!("hello")
        })
    );
    assert_eq!(
        doc.next_match("world", 0),
        Some(Match {
            loc: Loc { x: 6, y: 4 },
            text: st!("world")
        })
    );
    assert_eq!(doc.info.loaded_to, 5);
    doc.move_to(&Loc { x: 2, y: 2 });
    assert_eq!(
        doc.next_match("hello", 0),
        Some(Match {
            loc: Loc { x: 4, y: 2 },
            text: st!("hello")
        })
    );
    assert_eq!(doc.next_match("random", 0), None);
    doc.move_to(&Loc { x: 9, y: 4 });
    assert_eq!(
        doc.prev_match("你"),
        Some(Match {
            loc: Loc { x: 5, y: 4 },
            text: st!("你")
        })
    );
    assert_eq!(doc.prev_match("random"), None);
    assert_eq!(
        doc.prev_match("\\s+hello"),
        Some(Match {
            loc: Loc { x: 0, y: 2 },
            text: st!("    hello")
        })
    );
    // General searching stuff
    let mut searcher = Searcher::new("[0-9]+");
    assert_eq!(
        searcher.rfinds("hello098hello765hello"),
        vec![
            Match {
                loc: Loc { x: 13, y: 0 },
                text: "765".to_string()
            },
            Match {
                loc: Loc { x: 5, y: 0 },
                text: "098".to_string()
            },
        ],
    );
}

#[test]
fn document_replacing() {
    let mut doc = Document::open(Size::is(100, 10), "tests/data/unicode.txt").unwrap();
    doc.load_to(1000);
    doc.replace_all("hello", "你好");
    assert_eq!(doc.line(0), Some(st!("    你好")));
    assert_eq!(doc.line(1), Some(st!("\t你好")));
    assert_eq!(doc.line(2), Some(st!("    你好")));
    assert_eq!(doc.line(3), Some(st!("\t你好")));
    assert_eq!(doc.line(4), Some(st!("你好你world好你好")));
}

#[test]
fn document_validation() {
    let mut doc = Document::open(Size::is(100, 10), "tests/data/unicode.txt").unwrap();
    doc.load_to(1000);
    doc.move_to(&Loc { x: 2, y: 1 });
    doc.old_cursor = 5;
    doc.move_up();
    assert_eq!(doc.char_ptr, 4);
    assert_eq!(doc.loc(), Loc { x: 4, y: 0 });
}

#[test]
fn document_indices() {
    let mut doc = Document::open(Size::is(100, 10), "tests/data/unicode.txt").unwrap();
    doc.load_to(1000);
    assert_eq!(doc.character_idx(&Loc { x: 6, y: 0 }), 5);
    assert_eq!(doc.character_idx(&Loc { x: 5, y: 1 }), 2);
}

#[test]
fn file_paths() {
    assert!(get_absolute_path("tests/data/unicode.txt")
        .unwrap()
        .starts_with("/home/"));
    assert!(get_absolute_path("tests/data/unicode.txt")
        .unwrap()
        .starts_with("/home/"));
    assert_eq!(
        get_file_name("tests/data/unicode.txt"),
        Some(st!("unicode.txt"))
    );
    assert_eq!(
        get_file_name("tests/data/unicode.txt"),
        Some(st!("unicode.txt"))
    );
    assert_eq!(get_file_name("src/document.rs"), Some(st!("document.rs")));
    assert_eq!(get_file_ext("tests/data/unicode.txt"), Some(st!("txt")));
    assert_eq!(get_file_ext("src/document.rs"), Some(st!("rs")));
}

#[test]
fn fuzz() {
    for _ in 0..10 {
        println!("--");
        let size = Size { w: 10, h: 8 };
        let mut doc = Document::open(size, "tests/data/unicode.txt").unwrap();
        doc.load_to(100);
        println!("{} | {}", doc.loc().x, doc.char_ptr);
        for _ in 0..300 {
            let e = rand::random::<u8>() % 25;
            println!("{}", e);
            match e {
                0 => doc.forth(Event::Insert(doc.char_loc(), 'a'.to_string())),
                1 => doc.forth(Event::Insert(doc.char_loc(), 'b'.to_string())),
                2 => doc.forth(Event::Insert(doc.char_loc(), '在'.to_string())),
                3 => doc.forth(Event::Delete(
                    Loc {
                        x: doc.char_ptr.saturating_sub(1),
                        y: doc.char_loc().y,
                    },
                    ' '.to_string(),
                )),
                4 => doc.forth(Event::InsertLine(doc.loc().y, "surpri在se".to_string())),
                5 => doc.forth(Event::DeleteLine(doc.loc().y, "".to_string())),
                6 => doc.forth(Event::SplitDown(doc.char_loc())),
                7 => doc.forth(Event::SpliceUp(Loc {
                    x: 0,
                    y: doc.loc().y,
                })),
                8 => {
                    doc.move_left();
                    Ok(())
                }
                9 => {
                    doc.move_right();
                    Ok(())
                }
                10 => {
                    doc.move_up();
                    Ok(())
                }
                11 => {
                    doc.move_down();
                    Ok(())
                }
                12 => {
                    doc.move_end();
                    Ok(())
                }
                13 => {
                    doc.move_home();
                    Ok(())
                }
                14 => {
                    doc.move_top();
                    Ok(())
                }
                15 => {
                    doc.move_bottom();
                    Ok(())
                }
                16 => {
                    doc.move_page_up();
                    Ok(())
                }
                17 => {
                    doc.move_page_down();
                    Ok(())
                }
                18 => {
                    doc.move_prev_word();
                    Ok(())
                }
                19 => {
                    doc.move_next_word();
                    Ok(())
                }
                20 => {
                    doc.replace_all("a", "c");
                    Ok(())
                }
                21 => {
                    doc.commit();
                    Ok(())
                }
                22 => {
                    doc.commit();
                    Ok(())
                }
                23 => doc.undo(),
                24 => doc.redo(),
                _ => Ok(()),
            };
            println!("{} | {}", doc.loc().x, doc.char_ptr);
            doc.load_to(doc.len_lines() + 10);
        }
    }
}

/*
Template:

#[test]
fn name() {
}

*/
