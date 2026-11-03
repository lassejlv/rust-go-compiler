//~ ERROR E0107
struct R<'a> { s: &'a str }
fn f<'a, 'b>(r: R<'a, 'b>) {}
fn main() {}
