// include_str!/include_bytes!/include! with relative paths; line!/file! inside included code
const SELF_SRC: &str = include_str!("1459_macro_include_relative.rs");
static SELF_BYTES: &[u8] = include_bytes!("./1459_macro_include_relative.rs");

macro_rules! first_line_of {
    ($f:literal) => { include_str!($f).lines().next().unwrap_or("") };
}

fn main() {
    let first = SELF_SRC.lines().next().unwrap();
    println!("{}", first);
    println!("{}", SELF_BYTES.len() == SELF_SRC.len());
    println!("{}", first_line_of!("1459_macro_include_relative.rs"));
    println!("{}", SELF_SRC.lines().filter(|l| l.contains("include_str!")).count());
    let n = SELF_SRC.matches("println").count();
    println!("{}", n);
    println!("{}", &SELF_BYTES[..2] == b"//");
    println!("{}", line!());
}
