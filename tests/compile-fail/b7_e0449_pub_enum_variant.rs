//~ ERROR E0449
// Enum variants are as visible as their enum.
pub enum Color {
    Red,
    pub Green,
}
fn main() {
    let _ = (Color::Red, Color::Green);
}
