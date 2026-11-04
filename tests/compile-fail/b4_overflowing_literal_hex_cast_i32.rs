//~ ERROR literal out of range
// An unsuffixed literal in `lit as T` gets the type T: 0xFFFF_FFFF does
// not fit i32 (the lint also fires for hex literals).
fn main() {
    let mask = 0xFFFF_FFFF as i32;
    println!("{}", mask);
}
