//~ ERROR E0597
fn main() {
    let mut out: Vec<&i32> = Vec::new();
    {
        let local = 7;
        out.push(&local);
    }
    println!("{}", out[0]);
}
