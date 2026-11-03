//~ ERROR E0382
fn handle(_s: String) {}
fn main() {
    let s = String::from("x");
    for i in 0..3 {
        match i {
            0 => handle(s),
            _ => {}
        }
    }
}
