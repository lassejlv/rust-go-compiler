//~ ERROR E0506
fn main() {
    let mut cur: Option<String> = Some("a".into());
    let r = cur.as_deref();
    cur = None;
    println!("{:?}", r);
}
