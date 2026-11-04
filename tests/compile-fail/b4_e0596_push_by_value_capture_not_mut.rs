//~ ERROR E0596
fn main() {
    let v = Vec::new();
    let mut add = move |x: i32| {
        v.push(x);
        v.len()
    };
    println!("{}", add(1));
}
