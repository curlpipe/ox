#[cfg(test)]
use kaolinite::{document::*, event::*, utils::*, map::*, searching::*};
use sugars::hmap;

#[test]
fn char_mapping() {
    // Test data
    let mut test1_map = CharMap::new(hmap!{ 0 => vec![]});
    let mut test2_map = CharMap::new(hmap!{ 794385 => vec![(1, 1), (5, 4)] });
    let mut test3_map = CharMap::new(hmap!{ 2 => vec![(1, 1), (3, 2), (6, 4), (8, 5)] });
    // Output & Verification
    // Count
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
fn line_trimming() {
    // Test data
    let test1 = "".to_string();
    let test2 = "hello world".to_string();
    let test3 = "hello wor蔼t教案ld".to_string();
    let test4 = "蔼教案蔼教案教蔼".to_string();
    let test5 = "\t\t蔼教\t案 srtin".to_string();
    // Output
    let results = vec![
        trim(&test1, 0, 0, 4),
        trim(&test1, 128, 128, 4),
        trim(&test2, 6, 5, 4),
        trim(&test2, 6, 7, 4),
        trim(&test3, 0, 13, 4),
        trim(&test3, 13, 4, 4),
        trim(&test4, 1, 2, 4),
        trim(&test4, 1, 4, 4),
        trim(&test5, 1, 6, 4),
        trim(&test5, 5, 9, 2),
    ];
    // Verification
    assert_eq!(results, vec![
        "".to_string(),
        "".to_string(),
        "world".to_string(),
        "world".to_string(),
        "hello wor蔼t ".to_string(),
        " 案l".to_string(),
        "  ".to_string(),
        " 教 ".to_string(),
        "      ".to_string(),
        " 教  案 s".to_string(),
    ]);
}

#[test]
fn filetype_detection() {
    // Test data
    let test1 = "rs";
    let test2 = "txt";
    let test3 = "jsb";
    let test4 = "frag";
    // Output
    let results = vec![
        filetype(test1).unwrap_or("Unknown".to_string()),
        filetype(test2).unwrap_or("Unknown".to_string()),
        filetype(test3).unwrap_or("Unknown".to_string()),
        filetype(test4).unwrap_or("Unknown".to_string()),
    ];
    // Verification
    assert_eq!(
        results, 
        vec![
            "Rust".to_string(),
            "Plain Text".to_string(), 
            "Unknown".to_string(),
            "GLSL".to_string(),
        ]
    );
}

#[test]
fn errors() {
    // Test data
    let test1 = Error::OutOfRange;
    // Output
    let result = format!("{:?}", test1);
    // Verification
    assert_eq!(result, "OutOfRange".to_string());
}

#[test]
fn document_opening() {
    // Test data
    let size = Size { w: 10, h: 10 };
    let doc1 = Document::open(size, "demos/2.txt");
    let doc2 = Document::open(size, "demos/foo.txt");
    // Verification
    assert!(doc1.is_ok());
    let mut doc1 = doc1.unwrap();
    doc1.load_to(1);
    assert!(doc1.dbl_map.map.is_empty());
    assert_eq!(doc1.line_trim(0, 1, 3), Some("ell".to_string()));
    assert!(doc2.is_err());
}

#[test]
fn document_moving() {
    // Test data
    let size = Size::is(10, 10);
    let mut doc1 = Document::open(size, "demos/5.txt").unwrap();
    doc1.load_to(100);
    let mut doc2 = Document::open(size, "demos/6.txt").unwrap();
    doc2.load_to(100);
    let mut doc3 = Document::open(size, "demos/7short.txt").unwrap();
    doc2.load_to(100);
    // Output & Verification
    doc1.move_up();
    assert_eq!(doc1.loc(), Loc { x: 0, y: 0 });
    for _ in 0..12 {
        doc1.move_down();
    }
    assert_eq!(doc1.loc(), Loc { x: 0, y: 12 });
    assert_eq!(doc1.offset.y, 3);
    for _ in 0..11 {
        doc1.move_up();
    }
    assert_eq!(doc1.loc(), Loc { x: 0, y: 1 });
    assert_eq!(doc1.offset.y, 1);
    doc1.goto_y(8);
    assert_eq!(doc1.loc(), Loc { x: 0, y: 8 });
    assert_eq!(doc1.offset.y, 0);
    doc1.move_right();
    assert_eq!(doc1.loc(), Loc { x: 1, y: 8 });
    assert_eq!(doc1.offset.x, 0);
    for _ in 0..14 {
        doc2.move_right();
    }
    assert_eq!(doc2.loc(), Loc { x: 20, y: 0 });
    assert_eq!(doc2.char_loc(), Loc { x: 17, y: 0 });
    assert_eq!(doc2.offset.x, 11);
    for _ in 0..5 {
        doc2.move_left();
    }
    assert_eq!(doc2.loc(), Loc { x: 14, y: 0 });
    assert_eq!(doc2.char_loc(), Loc { x: 12, y: 0 });
    assert_eq!(doc2.offset.x, 11);
    doc2.move_home();
    assert_eq!(doc2.loc(), Loc { x: 0, y: 0 });
    assert_eq!(doc2.char_loc(), Loc { x: 0, y: 0 });
    assert_eq!(doc2.offset.x, 0);
    assert_eq!(doc2.move_left(), Status::StartOfLine);
    doc2.move_end();
    assert_eq!(doc2.loc(), Loc { x: 25, y: 0 });
    assert_eq!(doc2.char_loc(), Loc { x: 21, y: 0 });
    assert_eq!(doc2.offset.x, 25);
    assert_eq!(doc2.move_up(), Status::StartOfFile);
    doc2.move_down();
    assert_eq!(doc2.loc(), Loc { x: 22, y: 1 });
    assert_eq!(doc2.char_loc(), Loc { x: 18, y: 1 });
    doc2.goto(&Loc { x: 6, y: 1 });
    assert_eq!(doc2.loc(), Loc { x: 7, y: 1 });
    assert_eq!(doc2.char_loc(), Loc { x: 6, y: 1 });
    assert_eq!(doc2.offset.x, 0);
    assert_eq!(doc2.offset.y, 0);
    doc2.goto(&Loc { x: 6, y: 0 });
    doc2.old_cursor = 5;
    doc2.move_down();
    assert_eq!(doc2.loc(), Loc { x: 5, y: 1 });
    assert_eq!(doc2.char_loc(), Loc { x: 5, y: 1 });
    assert_eq!(doc2.offset.x, 0);
    assert_eq!(doc2.offset.y, 0);
    doc2.move_up();
    doc2.offset.x = 6;
    doc2.cursor.x = 0;
    doc2.char_ptr = 6;
    doc2.old_cursor = 5;
    doc2.move_down();
    assert_eq!(doc2.loc(), Loc { x: 5, y: 1 });
    assert_eq!(doc2.char_loc(), Loc { x: 5, y: 1 });
    assert_eq!(doc2.offset.x, 5);
    doc2.old_cursor = 0;
    doc2.move_down();
    doc2.move_down();
    doc2.move_end();
    assert_eq!(doc2.loc(), Loc { x: 0, y: 3 });
    assert_eq!(doc2.char_loc(), Loc { x: 0, y: 3 });
    assert_eq!(doc2.move_down(), Status::EndOfFile);
    assert_eq!(doc2.move_right(), Status::EndOfLine);
    doc2.move_up();
    doc2.move_end();
    assert_eq!(doc2.move_right(), Status::EndOfLine);
    doc2.move_left();
    doc2.move_left();
    doc2.move_right();
    doc2.goto_x(10);
    assert_eq!(doc2.loc(), Loc { x: 10, y: 2 });
    assert_eq!(doc2.char_loc(), Loc { x: 10, y: 2 });
    assert_eq!(doc2.offset.x, 10);
    doc1.goto(&Loc { x: 3, y: 5 });
    assert_eq!(doc1.char_loc(), Loc { x: 3, y: 5 });
    doc1.goto_y(15);
    doc1.goto_y(14);
    assert_eq!(doc1.loc(), Loc { x: 1, y: 14 });
    assert_eq!(doc1.offset.y, 6);
    doc1.move_top();
    assert_eq!(doc1.loc(), Loc { x: 0, y: 0 });
    assert_eq!(doc1.offset.y, 0);
    doc1.move_bottom();
    assert_eq!(doc1.loc(), Loc { x: 0, y: 17 });
    assert_eq!(doc1.offset.y, 8);
    doc3.goto_y(34);
    doc3.move_page_down();
    assert_eq!(doc3.cursor, Loc { x: 0, y: 0 });
    assert_eq!(doc3.offset.y, 35);
    doc3.move_page_down();
    assert_eq!(doc3.cursor, Loc { x: 0, y: 0 });
    assert_eq!(doc3.offset.y, 45);
    doc3.move_page_down();
    assert_eq!(doc3.cursor, Loc { x: 0, y: 0 });
    assert_eq!(doc3.offset.y, 55);
    doc3.move_page_up();
    assert_eq!(doc3.cursor, Loc { x: 0, y: 0 });
    assert_eq!(doc3.offset.y, 45);
    doc1.goto_y(4);
    doc1.move_page_down();
    doc1.move_page_down();
    assert_eq!(doc1.loc(), Loc { x: 0, y: 17 });
    assert_eq!(doc1.offset.y, 17);
    doc1.move_page_down();
    assert_eq!(doc1.loc(), Loc { x: 0, y: 17 });
    assert_eq!(doc1.offset.y, 17);
    doc2.move_bottom();
    doc2.move_end();
    assert_eq!(doc2.loc(), Loc { x: 0, y: 3 });
    assert_eq!(doc2.offset.y, 0);
}

#[test]
#[allow(unused_must_use)]
fn document_tab() {
    // Test data
    let size = Size::is(10, 10);
    let mut doc1 = Document::open(size, "demos/6tab.txt").unwrap();
    doc1.set_tab_width(6);
    doc1.load_to(100);
    // Output
    doc1.insert(&Loc { x: 20, y: 1 }, "\t");
    doc1.insert(&Loc { x: 0, y: 2 }, "\t");
    doc1.insert(&Loc { x: 5, y: 2 }, "hello\tworld");
    // Verification
    // Check tab map for each line
    assert_eq!(doc1.tab_map.get(0).unwrap(), &vec![(0, 0)]);
    assert_eq!(doc1.tab_map.get(1).unwrap(), &vec![(24, 20)]);
    assert_eq!(doc1.tab_map.get(2).unwrap(), &vec![(0, 0), (15, 10)]);
    // Check moving and cursor position
    doc1.goto(&Loc::at(0, 0));
    doc1.move_right();
    assert_eq!(doc1.loc(), Loc::at(6, 0));
    assert_eq!(doc1.char_ptr, 1);
    doc1.move_right();
    assert_eq!(doc1.loc(), Loc::at(7, 0));
    assert_eq!(doc1.char_ptr, 2);
    doc1.move_down();
    doc1.move_up();
    assert_eq!(doc1.loc(), Loc::at(7, 0));
    assert_eq!(doc1.char_ptr, 2);
    // Check tab cursor split
    doc1.goto(&Loc::at(1, 0));
    doc1.old_cursor = 6;
    doc1.move_down();
    doc1.move_left();
    doc1.move_left();
    assert_eq!(doc1.loc(), Loc::at(4, 1));
    assert_eq!(doc1.char_ptr, 4);
    doc1.old_cursor = 0;
    doc1.move_up();
    assert_eq!(doc1.loc(), Loc::at(0, 0));
    assert_eq!(doc1.char_ptr, 0);
    // Check width_of retrieval
    assert_eq!(doc1.width_of(1, 20), 6);
    assert_eq!(doc1.width_of(0, 1), 1);
}

#[test]
#[allow(unused_must_use)]
fn insertion() {
    // Test data
    let size = Size { w: 10, h: 10 };
    let mut doc1 = Document::open(size, "demos/8.txt").unwrap();
    doc1.load_to(100);
    let mut doc2 = Document::open(size, "demos/3.txt").unwrap();
    doc2.load_to(100);
    // Output
    doc1.insert(&Loc { x: 11, y: 16 }, "::read_to_string");
    doc2.insert(&Loc { x: 7, y: 10 }, "123");
    doc1.insert(&Loc { x: 1, y: 66 }, " // st蔼ld");
    doc1.insert(&Loc { x: 4, y: 66 }, "蔼");
    // Verification
    assert_eq!(doc1.line(16).unwrap(), "use std::fs::read_to_string;".to_string());
    assert_eq!(doc2.line(10).unwrap(), "offst的e123tting".to_string());
    assert_eq!(doc1.line(66).unwrap(), "} //蔼 st蔼ld".to_string());
    assert_eq!(doc1.dbl_map.get(66), Some(&vec![(4, 4), (9, 8)]));
    assert!(doc1.insert(&Loc { x: 1000, y: 100000 }, "hello").is_err());
}

#[test]
#[allow(unused_must_use)]
fn deletion() {
    // Test data
    let size = Size { w: 10, h: 10 };
    let mut doc1 = Document::open(size, "demos/8.txt").unwrap();
    doc1.load_to(200);
    let mut doc2 = Document::open(size, "demos/6.txt").unwrap();
    doc2.load_to(100);
    // Output
    doc1.delete(16..24, 20);
    doc1.delete(7.., 152);
    doc1.delete(..5, 102);
    doc1.delete(..=4, 22);
    doc1.delete(.., 2);
    doc2.delete(4..12, 0);
    doc2.delete(4..=5, 0);
    doc2.delete(6..=7, 0);
    doc2.delete(8..=9, 0);
    // Verification
    assert!(doc1.delete(5..2, 0).is_err());
    assert_eq!(doc1.line(20).unwrap(), "#[derive(Debug, PartialEq)]".to_string());
    assert_eq!(doc1.line(152).unwrap(), "    ///".to_string());
    assert_eq!(doc1.line(102).unwrap(), "    // Read in information".to_string());
    assert_eq!(doc1.line(22).unwrap(), "/// The file name of the document".to_string());
    assert_eq!(doc1.line(2).unwrap(), "".to_string());
    assert_eq!(doc2.line(0).unwrap(), "    stststs".to_string());
}

#[test]
#[allow(unused_must_use)]
fn line_tweaking() {
    // Test data
    let size = Size { w: 10, h: 10 };
    let mut doc1 = Document::open(size, "demos/6.txt").unwrap();
    doc1.load_to(100);
    // Output
    doc1.insert_line(0, "ewuln在hd".to_string());
    doc1.delete_line(3);
    doc1.delete_line(1);
    // Verification
    assert_eq!(
        vec![doc1.line(0).unwrap(), doc1.line(1).unwrap()],
        vec![
            "ewuln在hd".to_string(), 
            "  art的st了st在st为sts".to_string()
        ],
    );
}

#[test]
#[allow(unused_must_use)]
fn line_splicing() {
    // Test data
    let size = Size { w: 10, h: 10 };
    let mut doc1 = Document::open(size, "demos/6.txt").unwrap();
    doc1.load_to(100);
    // Output
    doc1.splice_up(1);
    // Verification
    assert!(doc1.splice_up(3).is_err());
    assert_eq!(doc1.line(0).unwrap(), "    arst的st了st在st为sts".to_string());
    assert_eq!(doc1.line(1).unwrap(), "  art的st了st在st为stshello world!".to_string());
}

#[test]
#[allow(unused_must_use)]
fn line_splitting() {
    // Test data
    let size = Size { w: 10, h: 10 };
    let mut doc1 = Document::open(size, "demos/6.txt").unwrap();
    doc1.load_to(100);
    // Output
    doc1.split_down(&Loc { x: 6, y: 2 });
    // Verification
    assert!(doc1.splice_up(3).is_err());
    assert_eq!(doc1.line(0).unwrap(), "    arst的st了st在st为sts".to_string());
    assert_eq!(doc1.line(1).unwrap(), "  art的st了st在st为sts".to_string());
    assert_eq!(doc1.line(2).unwrap(), "hello ".to_string());
    assert_eq!(doc1.line(3).unwrap(), "world!".to_string());
}

#[test]
fn line_numbering() {
    // Test data
    let size = Size { w: 10, h: 10 };
    let doc1 = Document::open(size, "demos/8.txt").unwrap();
    // Output & Verification
    assert_eq!(doc1.line_number(2), "    3".to_string());
    assert_eq!(doc1.line_number(2532), " 2533".to_string());
    assert_eq!(doc1.line_number(125323), "    ~".to_string());
}

#[test]
#[allow(unused_must_use)]
fn disk_writing() {
    // Test data
    let size = Size { w: 10, h: 10 };
    let mut doc1 = Document::open(size, "demos/6.txt").unwrap();
    doc1.load_to(100);
    // Output & Verification
    doc1.insert_line(2, "123".to_string());
    doc1.delete_line(1);
    doc1.save();
    assert_eq!(
        std::fs::read_to_string("demos/6.txt").unwrap(), 
        "    arst的st了st在st为sts\n123\nhello world!\n".to_string()
    );
    doc1.save_as("demos/6test.txt");
    assert_eq!(
        std::fs::read_to_string("demos/6test.txt").unwrap(), 
        "    arst的st了st在st为sts\n123\nhello world!\n".to_string()
    );
    doc1.delete_line(1);
    doc1.insert_line(1, "  art的st了st在st为sts".to_string());
    doc1.save();
    assert_eq!(
        std::fs::read_to_string("demos/6.txt").unwrap(), 
        "    arst的st了st在st为sts\n  art的st了st在st为sts\nhello world!\n".to_string()
    );
    assert_eq!(
        std::fs::read_to_string("demos/6test.txt").unwrap(), 
        "    arst的st了st在st为sts\n123\nhello world!\n".to_string()
    );
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
}

#[test]
#[allow(unused_must_use)]
fn undo() {
    // Test data
    let size = Size { w: 10, h: 10 };
    let mut doc1 = Document::open(size, "demos/6.txt").unwrap();
    doc1.load_to(100);
    let orig1 = doc1.lines.clone();
    // Output
    doc1.exe(Event::Insert(Loc { x: 0, y: 0 }, 'h'.to_string()));
    doc1.exe(Event::DeleteLine(1, "  art的st了st在st为sts".to_string()));
    doc1.exe(Event::Delete(Loc { x: 2, y: 1 }, 'l'.to_string()));
    doc1.event_mgmt.commit();
    let orig2 = doc1.lines.clone();
    doc1.exe(Event::InsertLine(2, "1984".to_string()));
    doc1.exe(Event::SplitDown(Loc { x: 2, y: 2 }));
    doc1.exe(Event::SpliceUp(Loc { x: 0, y: 2 }));
    let orig3 = doc1.lines.clone();
    // Verification
    doc1.undo();
    assert_eq!(doc1.lines, orig2);
    doc1.undo();
    assert_eq!(doc1.lines, orig1);
    doc1.redo();
    assert_eq!(doc1.lines, orig2);
    doc1.redo();
    assert_eq!(doc1.lines, orig3);
}

#[test]
fn word_jumping() {
    // Test data
    let size = Size { w: 10, h: 10 };
    let mut doc1 = Document::open(size, "demos/4.txt").unwrap();
    doc1.load_to(100);
    // Verification
    doc1.move_next_word();
    assert_eq!(doc1.char_ptr, 16);
    doc1.move_next_word();
    assert_eq!(doc1.char_ptr, 21);
    doc1.move_prev_word();
    assert_eq!(doc1.char_ptr, 16);
    doc1.move_prev_word();
    assert_eq!(doc1.char_ptr, 0);
    doc1.move_prev_word();
    assert_eq!(doc1.char_ptr, 0);
    doc1.move_down();
    assert_eq!(doc1.move_next_word(), Status::None);
    assert_eq!(doc1.char_ptr, 0);
    assert_eq!(doc1.move_prev_word(), Status::StartOfLine);
    assert_eq!(doc1.char_ptr, 0);
    doc1.move_up();
    doc1.goto_x(210);
    assert_eq!(doc1.move_next_word(), Status::EndOfLine);
    assert_eq!(doc1.char_ptr, 210);
}

#[test]
fn searching() {
    // Test data
    let size = Size { w: 10, h: 10 };
    let mut doc1 = Document::open(size, "demos/3.txt").unwrap();
    doc1.load_to(5);
    // Output & Verification
    assert_eq!(doc1.next_match("hi", 1), Some(Match { loc: Loc::at(1, 0), text: "hi".to_string() }));
    assert_eq!(doc1.next_match("k?ng", 1), Some(Match { loc: Loc::at(2, 3), text: "ng".to_string() }));
    assert_eq!(doc1.next_match("offst的(ett)", 1), Some(Match { 
        loc: Loc::at(6, 10), 
        text: "ett".to_string()
    }));
    assert_eq!(doc1.next_match("oesf", 1), None);
    doc1.move_right();
    assert_eq!(doc1.next_match("hi", 0), Some(Match { loc: Loc::at(1, 0), text: "hi".to_string() }));
    assert_eq!(doc1.next_match("hi", 1), Some(Match { loc: Loc::at(1, 6), text: "hi".to_string() }));
    doc1.goto(&Loc::at(4, 5));
    assert_eq!(doc1.prev_match("ex"), Some(Match { loc: Loc::at(0, 5), text: "ex".to_string() }));
    assert_eq!(doc1.prev_match("^a"), Some(Match { loc: Loc::at(0, 2), text: "a".to_string() }));
    assert_eq!(doc1.prev_match("f(i+)"), Some(Match { loc: Loc::at(1, 4), text: "i".to_string() }));
    assert_eq!(doc1.prev_match("eggbar"), None);
}

#[test]
#[allow(unused_must_use)]
fn replacing() {
    // Test data
    let size = Size { w: 10, h: 10 };
    let mut doc1 = Document::open(size, "demos/3.txt").unwrap();
    doc1.load_to(5);
    // Output & Verification
    doc1.replace_all("a", "b");
    assert_eq!(doc1.line(2), Some("b".to_string()));
    assert_eq!(doc1.line(5), Some("exbmple".to_string()));
    assert_eq!(doc1.line(14), Some("bxit的s".to_string()));
    doc1.replace(Loc::at(6, 10), "etting", "axit的s");
    assert_eq!(doc1.line(10), Some("offst的axit的s".to_string()));
    doc1.replace_all("s", "t");
    assert_eq!(doc1.line(10), Some("offtt的axit的t".to_string()));
}

#[test]
#[allow(unused_must_use)]
fn fuzz() {
    for _ in 0..500 {
        println!("--");
        let size = Size { w: 10, h: 8 };
        let mut doc = Document::open(size, "demos/2.txt").unwrap();
        doc.load_to(100);
        println!("{} | {}", doc.loc().x, doc.char_ptr);
        for _ in 0..200 {
            let e = rand::random::<u8>() % 25;
            println!("{}", e);
            match e {
                0 => doc.forth(Event::Insert(doc.char_loc(), 'a'.to_string())),
                1 => doc.forth(Event::Insert(doc.char_loc(), 'b'.to_string())),
                2 => doc.forth(Event::Insert(doc.char_loc(), '在'.to_string())),
                3 => doc.forth(Event::Delete(
                    Loc { x: doc.char_ptr.saturating_sub(1), y: doc.char_loc().y }, ' '.to_string())
                ),
                4 => doc.forth(Event::InsertLine(doc.loc().y, "surpri在se".to_string())),
                5 => doc.forth(Event::DeleteLine(doc.loc().y, "".to_string())),
                6 => doc.forth(Event::SplitDown(doc.char_loc())),
                7 => doc.forth(Event::SpliceUp(Loc { x: 0, y: doc.loc().y })),
                8 => { doc.move_left(); Ok(()) },
                9 => { doc.move_right(); Ok(()) },
                10 => { doc.move_up(); Ok(()) },
                11 => { doc.move_down(); Ok(()) },
                12 => { doc.move_end(); Ok(()) },
                13 => { doc.move_home(); Ok(()) },
                14 => { doc.move_top(); Ok(()) },
                15 => { doc.move_bottom(); Ok(()) },
                16 => { doc.move_page_up(); Ok(()) },
                17 => { doc.move_page_down(); Ok(()) },
                18 => { doc.move_prev_word(); Ok(()) },
                19 => { doc.move_next_word(); Ok(()) },
                20 => { doc.replace_all("a", "c"); Ok(()) },
                21 => { doc.event_mgmt.commit(); Ok(()) },
                22 => { doc.event_mgmt.commit(); Ok(()) },
                23 => { doc.undo() },
                24 => { doc.redo() },
                _ => Ok(()),
            };
            println!("{} | {}", doc.loc().x, doc.char_ptr);
        }
    }
}

#[test]
#[allow(unused_must_use)]
fn blank_document() {
    // Test data
    let mut document = Document::new(Size { w: 10, h: 10 });
    document.exe(Event::Insert(Loc { x: 0, y: 0 }, "hello, world!".to_string())).unwrap();
    // Output & Verification
    assert!(document.save().is_err());
    assert!(document.save_as("demos/dump.txt").is_ok());
    assert_eq!(
        std::fs::read_to_string("demos/dump.txt").unwrap(),
        "hello, world!\n".to_string()
    );
}

#[test]
#[allow(unused_must_use)]
fn read_only() {
    // Test data
    let mut document = Document::new(Size { w: 10, h: 10 });
    document.read_only = true;
    document.exe(Event::Insert(Loc { x: 0, y: 0 }, "hello, world!".to_string())).unwrap();
    // Output & Verification
    assert_eq!(document.lines, vec!["".to_string()]);
    assert!(document.save().is_err());
    assert!(document.save_as("demos/nonexist.txt").is_err());
    assert!(std::fs::read_to_string("demos/nonexist.txt").is_err());
}

/*
Template:

#[test]
#[allow(unused_must_use)]
fn name() {
    // Test data
    // Output
    // Verification
}

*/
