//~ ERROR E0308
async fn get() -> i32 { 1 }
async fn run() -> i32 {
    let x: i32 = get();
    x
}
fn main() { let _ = run(); }
