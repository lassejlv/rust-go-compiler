//~ ERROR E0382
fn main() {
    let buf = String::from("data");
    let mut n = 0;
    while n < 2 {
        let owned = buf;
        println!("{}", owned);
        n += 1;
    }
}
