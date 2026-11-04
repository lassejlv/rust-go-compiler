//~ ERROR E0521
fn first_static<'a>(p: (&'static str, &'a str)) -> &'static str { p.0 }
fn wrap<'a>(x: &'a str) -> &'static str { first_static((x, "y")) }
fn main() { println!("{}", wrap("q")); }
