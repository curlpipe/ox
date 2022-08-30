use kaolinite::utils;

fn main() {
    let string = "\tblack你milk好attack好".to_string();
    for i in 0..26 {
        // With the string, cut it so that it starts at i and is a length of 5
        // it will render tabs as 4 spaces
        println!("{:?}", utils::trim(&string, i, 5, 4));
    }
}
