//~ ERROR E0597
fn main() {
    let r: &i32;
    let k = 2;
    match k {
        2 => { let x = 5; r = &x; }
        _ => { r = &0; }
    }
    println!("{}", r);
}
