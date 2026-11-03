//~ ERROR E0023
enum Msg { Move(i32, i32) }
fn main() {
    let m = Msg::Move(1, 2);
    match m { Msg::Move(x, y, z) => println!("{} {} {}", x, y, z) }
}
