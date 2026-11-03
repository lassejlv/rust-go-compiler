//~ ERROR E0659
mod a {
    pub fn f() -> i32 { 1 }
}
mod b {
    pub fn f() -> i32 { 2 }
}

use a::*;
use b::*;

fn main() {
    println!("{}", f());
}
