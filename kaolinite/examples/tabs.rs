use kaolinite::{Document, Size, Loc};

fn main() {
    // Open document with the size of 10 characters by 10 characters
    let mut doc = Document::open(
        Size::is(10, 10),
        "demos/10.txt", 
    ).expect("File couldn't be opened");
    // Load viewport
    doc.load_to(10);
    doc.goto(&Loc { x: 0, y: 0 });
    // Find something out of buffer
    println!("{:?}", doc.line(0));
    println!("{:?}", doc.line(1));
    println!();
    println!("{:?}", doc.loc());
    doc.move_right();
    println!("{:?}", doc.loc());
}
