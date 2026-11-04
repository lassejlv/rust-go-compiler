//~ ERROR E0277
// Unsafe fn items do not implement `FnMut` (they cannot be called safely).
unsafe fn bump(x: i32) -> i32 { x + 1 }

fn main() {
    let v: Vec<i32> = vec![1, 2, 3].into_iter().map(bump).collect();
    println!("{:?}", v);
}
