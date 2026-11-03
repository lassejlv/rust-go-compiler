//~ ERROR E0507
struct User { name: String }
fn main() {
    let users = vec![User { name: "a".into() }];
    let names: Vec<String> = users.iter().map(|u| u.name).collect();
    println!("{:?}", names);
}
