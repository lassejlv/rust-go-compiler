//~ ERROR E0308
fn main() {
    let o = Some(2);
    let v = if let Some(n) = o { n } else { "none" };
}
