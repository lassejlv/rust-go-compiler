//~ ERROR E0277
// An unsafe fn item cannot become a `Box<dyn Fn>`.
unsafe fn halve(x: u32) -> u32 { x / 2 }

fn main() {
    let b: Box<dyn Fn(u32) -> u32> = Box::new(halve);
    println!("{}", b(8));
}
