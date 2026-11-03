//~ ERROR E0063
struct User {
    name: String,
    age: u32,
}

fn main() {
    let u = User { name: "a".into() };
    println!("{}", u.name);
}
