//~ ERROR E0015
const ANSWER: i32 = (|| 42)();
fn main() { println!("{}", ANSWER); }
