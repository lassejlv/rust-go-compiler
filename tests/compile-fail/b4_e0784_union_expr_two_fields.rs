//~ ERROR E0784
union Bits { i: u32, f: f32 }
fn main() {
    let b = Bits { i: 1, f: 2.0 };
    println!("{}", unsafe { b.i });
}
