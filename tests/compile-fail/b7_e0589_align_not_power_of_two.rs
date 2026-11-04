//~ ERROR E0589
// Alignments are powers of two.
#[repr(align(3))]
struct S;
fn main() {
    let _ = S;
}
