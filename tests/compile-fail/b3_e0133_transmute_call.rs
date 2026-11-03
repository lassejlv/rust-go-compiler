//~ ERROR E0133
fn main() {
    let f: f32 = std::mem::transmute(1u32);
    println!("{}", f);
}
