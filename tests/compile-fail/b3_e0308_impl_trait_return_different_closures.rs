//~ ERROR E0308
fn pick(b: bool) -> impl Fn() -> i32 {
    let a = 1;
    if b { move || a } else { move || a + 1 }
}
fn main() { pick(true)(); }
