//~ ERROR E0515
fn pick(flag: bool) -> &'static str {
    let s = String::from("dynamic");
    match flag { true => &s, false => "static" }
}
fn main() { pick(true); }
