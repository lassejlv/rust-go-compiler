//~ ERROR E0554
// `#![feature]` may not be used on the stable release channel.
#![feature(never_type)]
fn main() {
    println!("hi");
}
