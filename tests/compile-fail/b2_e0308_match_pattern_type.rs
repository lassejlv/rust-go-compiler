//~ ERROR E0308
fn main() {
    let x: Option<i32> = Some(1);
    match x {
        Ok(v) => println!("{}", v),
        _ => {}
    }
}
