// a panic in a nested call after output: exit code 101, Drop runs during unwinding
use std::collections::BTreeMap;

struct Guard {
    name: String,
}
impl Drop for Guard {
    fn drop(&mut self) {
        println!("unwinding drops {}", self.name);
    }
}

#[derive(Debug)]
struct Account {
    id: u32,
    balance: i64,
}

fn withdraw(acc: &mut Account, amt: i64) -> i64 {
    let _g = Guard { name: format!("withdraw-{}", acc.id) };
    if amt > acc.balance {
        panic!("insufficient funds in account {}: have {}, need {}", acc.id, acc.balance, amt);
    }
    acc.balance -= amt;
    acc.balance
}

fn process(ledger: &mut BTreeMap<u32, Account>, ops: &[(u32, i64)]) {
    let _g = Guard { name: "process".to_string() };
    for (id, amt) in ops {
        let acc = ledger.get_mut(id).expect("account exists");
        let left = withdraw(acc, *amt);
        println!("account {} withdrew {} -> {}", id, amt, left);
    }
}

fn audit(ledger: &BTreeMap<u32, Account>) -> Vec<String> {
    ledger
        .values()
        .map(|a| {
            let status = match a.balance {
                b if b < 0 => "overdrawn",
                0 => "empty",
                1..=20 => "low",
                _ => "ok",
            };
            format!("{}:{}:{}", a.id, a.balance, status)
        })
        .collect()
}

fn main() {
    let mut ledger = BTreeMap::new();
    for (id, bal) in [(1, 100), (2, 50), (3, 10)] {
        ledger.insert(id, Account { id, balance: bal });
    }
    println!("{:?}", ledger.values().map(|a| a.balance).collect::<Vec<_>>());
    let _outer = Guard { name: "main".into() };
    let ok = std::panic::catch_unwind(|| {
        let v: Vec<i32> = Vec::new();
        v.iter().sum::<i32>()
    });
    println!("catch ok: {:?}", ok.is_ok());
    let totals: i64 = ledger.values().map(|a| a.balance).sum();
    println!("total balance {}", totals);
    let res = std::panic::catch_unwind(|| {
        let mut a = Account { id: 9, balance: 5 };
        withdraw(&mut a, 100)
    });
    println!("caught inner panic: {}", res.is_err());
    let names: Vec<String> = ledger.keys().map(|k| format!("acct-{:03}", k)).collect();
    println!("{:?}", names);
    for (id, acc) in ledger.iter_mut() {
        acc.balance += *id as i64;
    }
    println!("{:?}", ledger);
    println!("{:?}", audit(&ledger));
    let _scoped = Guard { name: "before-process".into() };
    process(&mut ledger, &[(1, 30), (2, 50), (1, 60), (3, 111), (1, 1)]);
    println!("never printed");
}
