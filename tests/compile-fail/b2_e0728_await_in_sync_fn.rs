//~ ERROR E0728
async fn get() -> u8 { 1 }
fn main() { let x = get().await; }
