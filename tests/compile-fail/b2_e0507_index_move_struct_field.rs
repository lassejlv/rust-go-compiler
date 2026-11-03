//~ ERROR E0507
struct Rec { name: String }
fn main() {
    let v = vec![Rec { name: "a".into() }];
    let n = v[0].name;
    println!("{}", n);
}
