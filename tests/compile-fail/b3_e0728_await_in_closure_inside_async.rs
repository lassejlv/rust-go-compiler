//~ ERROR E0728
async fn get() -> i32 { 1 }
async fn run() {
    let v = vec![1, 2];
    let _r: Vec<i32> = v.iter().map(|_| get().await).collect();
}
fn main() { let _ = run(); }
