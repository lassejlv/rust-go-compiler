//~ ERROR E0502
fn main() {
    let mut names = vec![String::from("b"), String::from("a")];
    let pivot = &names[0];
    names.sort_by_key(|s| s.len());
    println!("{}", pivot);
}
