//~ ERROR E0507
fn main() {
    let v = vec![String::from("a")];
    let mut out = Vec::new();
    for s in &v {
        out.push(*s);
    }
}
