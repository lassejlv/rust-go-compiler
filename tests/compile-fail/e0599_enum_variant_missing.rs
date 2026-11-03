//~ ERROR E0599
enum Color {
    Red,
    Green,
}

fn main() {
    let c = Color::Blue;
    match c {
        Color::Red | Color::Green => {}
    }
}
