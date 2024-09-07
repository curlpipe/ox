use kaolinite::{Document, Size};

fn main() {
    // Open document with the size of 10 characters by 10 characters
    let mut doc = Document::open(
        Size::is(10, 10),
        "demos/7.txt", 
    ).expect("File couldn't be opened");
    // Load viewport
    doc.load_to(1);
    // Find something out of buffer
    let m = doc.next_match("spine", 0);
    println!("{m:?}");
}
