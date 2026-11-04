//~ ERROR E0732
// Explicit discriminants on an enum with fields need #[repr(inttype)]
// (repr(C) is not enough).
#[repr(C)]
enum Msg {
    Quit = 1,
    Move(i32, i32),
}
fn main() { let _ = Msg::Quit; }
