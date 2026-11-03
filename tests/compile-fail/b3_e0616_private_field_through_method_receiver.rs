//~ ERROR E0616
mod bank {
    pub struct Account { balance: u64 }
    impl Account { pub fn new() -> Self { Account { balance: 0 } } }
}
fn main() {
    let mut a = bank::Account::new();
    a.balance += 100;
}
