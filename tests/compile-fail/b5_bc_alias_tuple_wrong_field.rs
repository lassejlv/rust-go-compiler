//~ ERROR lifetime
type Row<'a> = Vec<(&'a str, &'static str)>;
type Out = &'static str;
fn first<'a>(r: &Row<'a>) -> Out {
    r[0].0
}
fn main() {
    let s = String::from("v");
    println!("{}", first(&vec![(s.as_str(), "b")]));
}
