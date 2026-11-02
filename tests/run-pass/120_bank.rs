use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
enum TxKind {
    Deposit,
    Withdraw,
    TransferIn(u32),
    TransferOut(u32),
    Interest,
}

#[derive(Debug, Clone)]
struct Tx {
    kind: TxKind,
    amount: i64,
}

#[derive(Debug)]
enum BankError {
    NoSuchAccount(u32),
    Insufficient { id: u32, needed: i64, available: i64 },
    Frozen(u32),
    InvalidAmount,
}

impl fmt::Display for BankError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BankError::NoSuchAccount(id) => write!(f, "account {} not found", id),
            BankError::Insufficient { id, needed, available } => {
                write!(f, "account {} needs {} but has {}", id, cents(*needed), cents(*available))
            }
            BankError::Frozen(id) => write!(f, "account {} is frozen", id),
            BankError::InvalidAmount => write!(f, "invalid amount"),
        }
    }
}

fn cents(v: i64) -> String {
    let sign = if v < 0 { "-" } else { "" };
    format!("{}${}.{:02}", sign, v.abs() / 100, v.abs() % 100)
}

struct Account {
    owner: String,
    balance: i64,
    frozen: bool,
    history: Vec<Tx>,
}

struct Bank {
    accounts: BTreeMap<u32, Account>,
    next_id: u32,
}

impl Bank {
    fn new() -> Self {
        Bank { accounts: BTreeMap::new(), next_id: 100 }
    }

    fn open(&mut self, owner: &str, initial: i64) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        let mut acc = Account { owner: owner.to_string(), balance: 0, frozen: false, history: vec![] };
        if initial > 0 {
            acc.balance = initial;
            acc.history.push(Tx { kind: TxKind::Deposit, amount: initial });
        }
        self.accounts.insert(id, acc);
        id
    }

    fn get_mut(&mut self, id: u32) -> Result<&mut Account, BankError> {
        let acc = self.accounts.get_mut(&id).ok_or(BankError::NoSuchAccount(id))?;
        if acc.frozen {
            return Err(BankError::Frozen(id));
        }
        Ok(acc)
    }

    fn deposit(&mut self, id: u32, amt: i64) -> Result<i64, BankError> {
        if amt <= 0 {
            return Err(BankError::InvalidAmount);
        }
        let acc = self.get_mut(id)?;
        acc.balance += amt;
        acc.history.push(Tx { kind: TxKind::Deposit, amount: amt });
        Ok(acc.balance)
    }

    fn withdraw(&mut self, id: u32, amt: i64) -> Result<i64, BankError> {
        if amt <= 0 {
            return Err(BankError::InvalidAmount);
        }
        let acc = self.get_mut(id)?;
        if acc.balance < amt {
            return Err(BankError::Insufficient { id, needed: amt, available: acc.balance });
        }
        acc.balance -= amt;
        acc.history.push(Tx { kind: TxKind::Withdraw, amount: amt });
        Ok(acc.balance)
    }

    fn transfer(&mut self, from: u32, to: u32, amt: i64) -> Result<(), BankError> {
        self.get_mut(to)?;
        self.withdraw(from, amt)?;
        self.deposit(to, amt)?;
        self.accounts.get_mut(&from).unwrap().history.last_mut().unwrap().kind = TxKind::TransferOut(to);
        self.accounts.get_mut(&to).unwrap().history.last_mut().unwrap().kind = TxKind::TransferIn(from);
        Ok(())
    }

    fn apply_interest(&mut self, basis_points: i64) {
        for acc in self.accounts.values_mut() {
            if acc.frozen || acc.balance <= 0 {
                continue;
            }
            let interest = acc.balance * basis_points / 10000;
            acc.balance += interest;
            acc.history.push(Tx { kind: TxKind::Interest, amount: interest });
        }
    }

    fn statement(&self) {
        for (id, acc) in &self.accounts {
            println!("#{} {:<6} {:>12}{}", id, acc.owner, cents(acc.balance), if acc.frozen { " [FROZEN]" } else { "" });
            for tx in &acc.history {
                let label = match &tx.kind {
                    TxKind::Deposit => "deposit".to_string(),
                    TxKind::Withdraw => "withdraw".to_string(),
                    TxKind::TransferIn(f) => format!("from #{}", f),
                    TxKind::TransferOut(t) => format!("to #{}", t),
                    TxKind::Interest => "interest".to_string(),
                };
                println!("      {:<10} {:>10}", label, cents(tx.amount));
            }
        }
    }
}

fn main() {
    let mut bank = Bank::new();
    let alice = bank.open("alice", 50000);
    let bob = bank.open("bob", 1250);
    let carol = bank.open("carol", 0);
    let report = |label: &str, r: Result<String, BankError>| match r {
        Ok(s) => println!("{:<28} ok {}", label, s),
        Err(e) => println!("{:<28} ERR {}", label, e),
    };
    report("deposit carol 99.99", bank.deposit(carol, 9999).map(cents));
    report("withdraw bob 20.00", bank.withdraw(bob, 2000).map(cents));
    report("withdraw bob 2.50", bank.withdraw(bob, 250).map(cents));
    report("transfer alice->bob 123.45", bank.transfer(alice, bob, 12345).map(|_| String::new()));
    report("transfer bob->999", bank.transfer(bob, 999, 100).map(|_| String::new()));
    report("deposit -5", bank.deposit(alice, -5).map(cents));
    bank.accounts.get_mut(&carol).unwrap().frozen = true;
    report("withdraw frozen carol", bank.withdraw(carol, 100).map(cents));
    report("transfer alice->carol", bank.transfer(alice, carol, 100).map(|_| String::new()));
    bank.apply_interest(125);
    bank.statement();
    let total: i64 = bank.accounts.values().map(|a| a.balance).sum();
    println!("total holdings {}", cents(total));
    let richest = bank.accounts.iter().max_by_key(|(_, a)| a.balance).map(|(id, a)| (*id, a.owner.clone()));
    println!("richest {:?}", richest);
}
