#[cfg(test)]

use kaolinite::{document::*, event::*, map::*, searching::*, utils::*};
use kaolinite::regex;
use sugars::hmap;
use std::ops::{Range, RangeBounds};
use std::io::Write;

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
}

#[test]
fn regex() {
    let reg = regex!("a+b*c");
    println!("{:?}", reg.captures("aaac"));
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
    assert_eq!(
        ranges,
        vec![(1, 7), (1, 8), (1, 100), (0, 7)],
    );
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
    assert_eq!(tab_boundaries_forward("            hello      hello", 6), vec![0, 6]);
    assert_eq!(tab_boundaries_forward(" 你 ", 1), vec![0]);
    // Backward
    assert_eq!(tab_boundaries_backward("hello", 4), vec![]);
    assert_eq!(tab_boundaries_backward("   hello", 3), vec![3]);
    assert_eq!(tab_boundaries_backward("    hello", 2), vec![2, 4]);
    assert_eq!(tab_boundaries_backward("     hello     hello2", 5), vec![5]);
    assert_eq!(tab_boundaries_backward("            hello      hello", 6), vec![6, 12]);
    assert_eq!(tab_boundaries_backward(" 你 ", 1), vec![1]);
}

#[test]
fn searching() {
    // Basic URL grabber test
    let mut url_grabber = Searcher::new(r"\b(?:https?://|www\.)\S+\b");
    let text = st!("click here: https://github.com/curlpipe/ox to see more information or visit https://curlpipe.github.io");
    assert_eq!(url_grabber.lfind(&text), Some(Match { loc: Loc { x: 12, y: 0 }, text: st!("https://github.com/curlpipe/ox") }));
    assert_eq!(url_grabber.rfind(&text), Some(Match { loc: Loc { x: 76, y: 0 }, text: st!("https://curlpipe.github.io") }));
    let text = st!("there are no links here!");
    assert_eq!(url_grabber.lfind(&text), None);
    assert_eq!(url_grabber.rfind(&text), None);
    // Unicode handling
    let mut greeting_finder = Searcher::new("你好");
    let text = st!("Hello is 你好 in Mandarin: 你好");
    assert_eq!(Searcher::raw_to_char(0, &text), 0);
    assert_eq!(Searcher::raw_to_char(15, &text), 11);
    assert_eq!(Searcher::raw_to_char(16, &text), 12);
    assert_eq!(greeting_finder.lfind(&text), Some(Match { loc: Loc { x: 9, y: 0 }, text: st!("你好") }));
    assert_eq!(greeting_finder.rfind(&text), Some(Match { loc: Loc { x: 25, y: 0 }, text: st!("你好") }));
}

#[test]
fn char_mapping() {
    let mut test1_map = CharMap::new(hmap!{ 0 => vec![]});
    let mut test2_map = CharMap::new(hmap!{ 794385 => vec![(1, 1), (5, 4)] });
    let mut test3_map = CharMap::new(hmap!{ 2 => vec![(1, 1), (3, 2), (6, 4), (8, 5)] });
    let mut test4_map = CharMap::new(hmap!{ 5 => vec![(0, 0)] });
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
    assert_eq!(test3_map.get(2).unwrap(), &vec![(1, 1), (3, 2), (4, 4), (6, 5), (6, 4), (8, 5)]);
    assert_eq!(test1_map.get(5).unwrap(), &vec![(5, 5), (7, 6), (8, 7)]);
    // Shift_insertion
    assert_eq!(test2_map.shift_insertion(&Loc::at(2, 0), "\to教", 4), 0);
    assert_eq!(test2_map.get(794385).unwrap(), &vec![(1, 1), (5, 4), (9, 7)]);
    assert_eq!(test2_map.get(0), None);
    assert_eq!(test2_map.shift_insertion(&Loc::at(2, 794385), "\to教", 4), 1);
    assert_eq!(test2_map.get(794385).unwrap(), &vec![(1, 1), (12, 7), (16, 10)]);
    // Shift_deletion
    test2_map.shift_deletion(&Loc::at(0, 0), (2, 5), "\to教", 4);
    assert_eq!(test2_map.get(0), None);
    assert_eq!(test2_map.get(794385).unwrap(), &vec![(1, 1), (12, 7), (16, 10)]);
    test2_map.shift_deletion(&Loc::at(0, 794385), (2, 5), "\to教", 4);
    assert_eq!(test2_map.get(794385).unwrap(), &vec![(1, 1), (5, 4), (9, 7)]);
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
    assert_eq!(form_map(&test_data_string2, 4), 
               (vec![(8, 2), (10, 3), (16, 5)], vec![(0, 0), (4, 1), (12, 4)]));
    assert_eq!(form_map(&test_data_string1, 3), (vec![], vec![]));
    assert_eq!(form_map(&test_data_string2, 5),
               (vec![(10, 2), (12, 3), (19, 5)], vec![(0, 0), (5, 1), (14, 4)]));
}

#[test]
#[allow(unused_must_use)]
fn event_management() {
    // Test data
    let mut mgmt = EventMgmt::default();
    mgmt.register(Event::Insert(Loc { x: 0, y: 0 }, 't'.to_string()));
    mgmt.register(Event::Insert(Loc { x: 1, y: 0 }, 'e'.to_string()));
    mgmt.commit();
    mgmt.register(Event::Insert(Loc { x: 2, y: 0 }, 's'.to_string()));
    mgmt.register(Event::Insert(Loc { x: 3, y: 0 }, 't'.to_string()));
    mgmt.commit();
    // Output & Verification
    assert_eq!(
        mgmt.undo(), 
        Some(vec![
             Event::Insert(Loc { x: 3, y: 0 }, 't'.to_string()),
             Event::Insert(Loc { x: 2, y: 0 }, 's'.to_string()),
        ])
    );
    mgmt.register(Event::Insert(Loc { x: 0, y: 0 }, 'x'.to_string()));
    assert!(!mgmt.is_patch_empty());
    assert!(mgmt.is_redo_empty());
    assert!(!mgmt.is_undo_empty());
    assert_eq!(
        mgmt.last(),
        Some(&Event::Insert(Loc { x: 0, y: 0 }, 'x'.to_string()))
    );
    mgmt.commit();
    assert_eq!(
        mgmt.undo(),
        Some(vec![
             Event::Insert(Loc { x: 0, y: 0 }, 'x'.to_string()),
        ])
    );
    assert_eq!(
        mgmt.undo(),
        Some(vec![
             Event::Insert(Loc { x: 1, y: 0 }, 'e'.to_string()),
             Event::Insert(Loc { x: 0, y: 0 }, 't'.to_string()),
        ])
    );
    assert_eq!(
        mgmt.redo(), 
        Some(vec![
             Event::Insert(Loc { x: 0, y: 0 }, 't'.to_string()),
             Event::Insert(Loc { x: 1, y: 0 }, 'e'.to_string()),
        ])
    );
    assert_eq!(
        mgmt.redo(),
        Some(vec![
             Event::Insert(Loc { x: 0, y: 0 }, 'x'.to_string()),
        ])
    );
    assert_eq!(
        mgmt.last(),
        Some(&Event::Insert(Loc { x: 0, y: 0 }, 'x'.to_string()))
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
    doc.read_only = true;
    assert!(doc.save().is_err());
    // Save as
    assert!(doc.save_as("tests/data/ghost.txt").is_err());
    doc.read_only = false;
    assert!(doc.save_as("tests/data/ghost.txt").is_ok());
    let result = std::fs::read_to_string("tests/data/ghost.txt").unwrap();
    std::fs::remove_file("tests/data/ghost.txt").unwrap();
    assert_eq!(result, st!("\n"));
}

#[test]
fn document_insertion() {
}

#[test]
fn document_deletion() {
}

#[test]
fn document_undo_redo() {
}

#[test]
fn document_moving() {
}

#[test]
fn document_scrolling() {
}

#[test]
fn document_utilities() {
    let mut doc = Document::open(Size::is(100, 10), "tests/data/big.txt").unwrap();
    doc.load_to(1000);
    // Cursor location
    doc.move_to(&Loc { x: 5, y: 5 });
    assert_eq!(doc.loc(), Loc { x: 5, y: 5 });
    assert_eq!(doc.char_loc(), Loc { x: 5, y: 5 });
    // Tab width
    doc.set_tab_width(2);
    assert_eq!(doc.tab_width, 2);
}

#[test]
fn document_line_editing() {
}

#[test]
fn document_splitting_splicing() {
}

#[test]
fn document_selection() {
}

#[test]
fn document_searching() {
}

#[test]
fn document_replacing() {
}

#[test]
fn document_validation() {
}

#[test]
fn document_indices() {
}

#[test]
fn document_buffering() {
}

/*
Template:

#[test]
fn name() {
}

*/
