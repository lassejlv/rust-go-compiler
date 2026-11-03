//~ ERROR E0503
struct Pt { x: i32 }
fn main() {
    let mut p = Pt { x: 1 };
    let r = &mut p;
    let y = p.x + 1;
    r.x = y;
}
