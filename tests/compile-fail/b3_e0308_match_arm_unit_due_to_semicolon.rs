//~ ERROR E0308
fn sign(x: i32) -> &'static str {
    match x {
        0 => "zero",
        _ => { "nonzero"; }
    }
}
fn main() { sign(1); }
