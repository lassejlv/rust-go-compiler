//~ ERROR E0597
fn keep(_s: &'static str) {}

fn main() {
    let s = String::from("local");
    keep(&s);
}
