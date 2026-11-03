//~ ERROR E0369
struct T(i32);

fn main() {
    let a = T(1);
    let b = T(1);
    if a == b {
        println!("eq");
    }
}
