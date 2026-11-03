//~ ERROR E0373
fn counter() -> Box<dyn FnMut() -> u32> {
    let mut c = 0;
    Box::new(|| { c += 1; c })
}
fn main() { let mut f = counter(); f(); }
