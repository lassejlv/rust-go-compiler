//~ ERROR lifetime may not live long enough
fn to_static<'a>(s: &'a str) -> &'static str {
    s
}

fn main() {
    println!("{}", to_static("hi"));
}
