use kaolinite::{Document, Size};
use std::time::Instant;

fn main() {
    let start = Instant::now();
    // Open document with the size of 10 characters by 10 characters
    let mut doc = Document::open(Size::is(10, 10), "demos/7.txt").expect("File couldn't be opened");
    // Load viewport
    doc.load_to(10);
    // Display the first 100 characters from the first 10 lines
    let mut len = 0;
    for i in 0..10 {
        println!(
            "{}",
            doc.line(i).unwrap().chars().take(100).collect::<String>()
        );
        len += doc.line(i).unwrap().chars().count();
    }
    println!("{}", len);
    let end = Instant::now();
    println!("ran in {:?}", end - start);
}
