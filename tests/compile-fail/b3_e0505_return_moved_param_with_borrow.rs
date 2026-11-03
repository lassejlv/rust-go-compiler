//~ ERROR E0505
fn pick(v: Vec<String>) -> (Vec<String>, &'static str) {
    let first: &String = &v[0];
    let out = v;
    println!("{}", first);
    (out, "x")
}
fn main() { pick(vec![]); }
