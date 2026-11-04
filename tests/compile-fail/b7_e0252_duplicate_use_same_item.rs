//~ ERROR E0252
// Importing the same name twice is E0252 even when both name one item.
use std::io::{self, Read, Write};
use std::io::Write;
fn main() {
    let _ = io::stdout().flush();
}
