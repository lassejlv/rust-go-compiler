//~ ERROR E0252
mod a { pub struct Config; }
mod b { pub struct Config; }
use a::Config;
use b::Config;
fn main() {}
