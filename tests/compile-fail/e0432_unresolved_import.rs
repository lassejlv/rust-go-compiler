//~ ERROR E0432
use std::collections::HashMapp;

fn main() {
    let m: HashMapp<i32, i32> = HashMapp::new();
}
