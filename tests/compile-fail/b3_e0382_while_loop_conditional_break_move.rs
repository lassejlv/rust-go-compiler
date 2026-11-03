//~ ERROR E0382
fn main() {
    let s = String::from("x");
    let mut n = 0;
    while n < 3 {
        let t = s;
        n += t.len();
    }
}
