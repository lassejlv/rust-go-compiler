//~ ERROR E0597
fn main() {
    let mut refs: Vec<&String> = Vec::new();
    for i in 0..3 {
        let s = i.to_string();
        refs.push(&s);
    }
    println!("{:?}", refs);
}
