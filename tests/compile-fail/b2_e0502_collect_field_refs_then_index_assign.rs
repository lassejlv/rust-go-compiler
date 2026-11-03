//~ ERROR E0502
struct P { name: String }
fn main() {
    let mut ps = vec![P { name: "x".into() }];
    let names: Vec<&String> = ps.iter().map(|p| &p.name).collect();
    ps[0].name = String::from("y");
    println!("{:?}", names);
}
