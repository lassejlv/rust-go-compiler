//~ ERROR E0616
mod bank {
    pub struct Account {
        balance: i64,
    }
    impl Account {
        pub fn new() -> Account {
            Account { balance: 0 }
        }
    }
}

fn main() {
    let a = bank::Account::new();
    println!("{}", a.balance);
}
