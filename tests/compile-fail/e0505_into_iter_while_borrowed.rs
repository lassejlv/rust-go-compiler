//~ ERROR E0505
fn main() {
    let v = vec![String::from("a")];
    let first = &v[0];
    let all: Vec<String> = v.into_iter().collect();
    println!("{} {}", first, all.len());
}
