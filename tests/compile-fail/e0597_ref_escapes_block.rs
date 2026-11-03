//~ ERROR E0597
fn main() {
    let r;
    {
        let x = 5;
        r = &x;
    }
    println!("{}", r);
}
