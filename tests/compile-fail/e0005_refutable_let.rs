//~ ERROR E0005
fn main() {
    let o: Option<i32> = Some(1);
    let Some(x) = o;
    println!("{}", x);
}
