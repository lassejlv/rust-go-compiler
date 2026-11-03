//~ ERROR E0502
fn main() {
    let mut slot: Option<String> = Some("a".into());
    let r = slot.as_ref().unwrap();
    slot.replace(String::from("b"));
    println!("{}", r);
}
