/* 
	Ox 0.2.6
	Now with a new scripting language and more extensibility
*/

pub const PI = 3.14159;

#[derive(Debug)]
struct Person {
	pub name: String,
	pub phone: String,
}

impl Person {
	pub fn new() -> Self {
		// Create a new person
		Self {
			name: "Curlpipe",
			// Not actually my phone number
			// I randomly pressed numbers on my keyboard
			phone: "+44 07836451973",
		}
	}
	pub fn sleep(&self) {
		// Go to sleep
		println!("ZzZzZzZzZ");
	}
}

enum Emotion {
	Happy,
	Sad,
}

pub fn main() -> String {
	// Welcome to your first program in Ox!
	let awesome = true;
	let mut emotion = Emotion::Happy;
	let age = 30;

	if awesome {
		println!("Now With Syntax Highlighting!");
		let x = format!("Price: {}0", '£');
	}

	let new_age = age * 2;
	return "Hello World!";
}
