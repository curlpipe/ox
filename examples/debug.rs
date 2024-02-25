use kaolinite::{Document, Size, event::Event};

fn main() {
    // Open document with the size of 10 characters by 10 characters
    let mut doc = Document::open(
        Size::is(100, 100),
        "cactus/fresh.txt", 
    ).expect("File couldn't be opened");
    // Load viewport
    doc.load_to(100);
    println!("{:?}", doc);
    doc.exe(Event::InsertLine(doc.loc().y, "".to_string()));
    println!("{:?}", doc);
}
