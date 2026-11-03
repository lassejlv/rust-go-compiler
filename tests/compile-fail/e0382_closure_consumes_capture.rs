//~ ERROR E0382
fn main() {
    let s = String::from("abc");
    let f = || {
        let t = s;
        t.len()
    };
    let n = f();
    println!("{} {}", n, s);
}
