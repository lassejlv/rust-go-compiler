//~ ERROR E0728
async fn fetch() -> u8 { 1 }
fn main() { let x = fetch().await; println!("{}", x); }
