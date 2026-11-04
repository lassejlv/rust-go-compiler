//~ ERROR null characters in C string literals are not supported
// C string literals cannot contain NUL.
fn main() {
    let s = c"a\0b";
    println!("{:?}", s);
}
