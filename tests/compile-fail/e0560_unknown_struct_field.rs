//~ ERROR E0560
struct User {
    name: String,
}

fn main() {
    let u = User { name: "a".into(), email: "e".into() };
    println!("{}", u.name);
}
