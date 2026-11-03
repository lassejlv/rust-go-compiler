//~ ERROR E0616
mod bank { pub struct Acct { balance: u64 } impl Acct { pub fn new() -> Acct { Acct { balance: 0 } } } }
fn main() { let a = bank::Acct::new(); println!("{}", a.balance); }
