//~ ERROR E0308
fn main() {
    let r: Result<i32, ()> = Ok(1);
    if let Some(x) = r { println!("{}", x); }
}
