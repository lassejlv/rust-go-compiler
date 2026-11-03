use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct AccountId(u32);
impl fmt::Display for AccountId { fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "ACC-{:04}", self.0) } }

#[derive(Debug, Clone, PartialEq)]
enum Txn { Deposit { to: AccountId, cents: u64 }, Withdraw { from: AccountId, cents: u64 }, Transfer { from: AccountId, to: AccountId, cents: u64 }, Interest { bps: u32 } }

#[derive(Debug, PartialEq)]
enum TxnError { NoSuchAccount(AccountId), Insufficient { id: AccountId, short_by: u64 }, Frozen(AccountId), SameAccount }
impl fmt::Display for TxnError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::NoSuchAccount(id) => write!(f, "no such account {}", id),
            Self::Insufficient { id, short_by } => write!(f, "{} short by {}", id, Cents(*short_by)),
            Self::Frozen(id) => write!(f, "{} is frozen", id),
            Self::SameAccount => write!(f, "cannot transfer to same account"),
        }
    }
}

struct Cents(u64);
impl fmt::Display for Cents {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let whole = self.0 / 100;
        let s = whole.to_string();
        let mut grouped = String::new();
        for (i, c) in s.chars().enumerate() { if i > 0 && (s.len() - i) % 3 == 0 { grouped.push(','); } grouped.push(c); }
        f.pad(&format!("${}.{:02}", grouped, self.0 % 100))
    }
}

trait Policy { fn overdraft_limit(&self) -> u64 { 0 } fn fee(&self, _t: &Txn) -> u64 { 0 } fn label(&self) -> String; }
struct Basic; struct Premium { limit: u64 }
impl Policy for Basic { fn fee(&self, t: &Txn) -> u64 { if matches!(t, Txn::Withdraw { .. }) { 150 } else { 0 } } fn label(&self) -> String { "basic".into() } }
impl Policy for Premium { fn overdraft_limit(&self) -> u64 { self.limit } fn label(&self) -> String { format!("premium(od={})", Cents(self.limit)) } }

struct Account { owner: String, balance: i64, frozen: bool, policy: Box<dyn Policy>, history: Vec<i64> }

#[derive(Default)]
struct Bank { accounts: BTreeMap<AccountId, Account>, next: u32, journal: Vec<(usize, Result<(), String>)> }

impl Bank {
    fn open(&mut self, owner: &str, policy: Box<dyn Policy>) -> AccountId {
        self.next += 1;
        let id = AccountId(self.next);
        self.accounts.insert(id, Account { owner: owner.into(), balance: 0, frozen: false, policy, history: vec![] });
        id
    }
    fn acct(&mut self, id: AccountId) -> Result<&mut Account, TxnError> {
        let a = self.accounts.get_mut(&id).ok_or(TxnError::NoSuchAccount(id))?;
        if a.frozen { Err(TxnError::Frozen(id)) } else { Ok(a) }
    }
    fn debit(&mut self, id: AccountId, cents: u64, t: &Txn) -> Result<(), TxnError> {
        let a = self.acct(id)?;
        let total = cents + a.policy.fee(t);
        let avail = a.balance + a.policy.overdraft_limit() as i64;
        if (total as i64) > avail { return Err(TxnError::Insufficient { id, short_by: total - avail.max(0) as u64 }); }
        a.balance -= total as i64;
        a.history.push(-(total as i64));
        Ok(())
    }
    fn apply(&mut self, t: &Txn) -> Result<(), TxnError> {
        match *t {
            Txn::Deposit { to, cents } => { let a = self.acct(to)?; a.balance += cents as i64; a.history.push(cents as i64); Ok(()) }
            Txn::Withdraw { from, cents } => self.debit(from, cents, t),
            Txn::Transfer { from, to, cents } => {
                if from == to { return Err(TxnError::SameAccount); }
                self.acct(to)?;
                self.debit(from, cents, t)?;
                let a = self.acct(to)?; a.balance += cents as i64; a.history.push(cents as i64);
                Ok(())
            }
            Txn::Interest { bps } => {
                for a in self.accounts.values_mut().filter(|a| !a.frozen && a.balance > 0) {
                    let i = a.balance * bps as i64 / 10_000;
                    a.balance += i; a.history.push(i);
                }
                Ok(())
            }
        }
    }
    fn run(&mut self, txns: &[Txn]) {
        for (i, t) in txns.iter().enumerate() {
            let r = self.apply(t).map_err(|e| e.to_string());
            self.journal.push((i, r));
        }
    }
}

fn main() {
    let mut bank = Bank::default();
    let alice = bank.open("alice", Box::new(Basic));
    let bob = bank.open("bob", Box::new(Premium { limit: 50_000 }));
    let carol = bank.open("carol", Box::new(Basic));
    let ghost = AccountId(99);
    let txns = vec![
        Txn::Deposit { to: alice, cents: 1_234_567 },
        Txn::Deposit { to: bob, cents: 10_000 },
        Txn::Withdraw { from: bob, cents: 45_000 },
        Txn::Withdraw { from: bob, cents: 20_000 },
        Txn::Transfer { from: alice, to: carol, cents: 300_050 },
        Txn::Transfer { from: carol, to: carol, cents: 1 },
        Txn::Transfer { from: carol, to: ghost, cents: 1 },
        Txn::Withdraw { from: carol, cents: 299_950 },
        Txn::Withdraw { from: carol, cents: 1 },
        Txn::Interest { bps: 125 },
        Txn::Deposit { to: ghost, cents: 5 },
    ];
    bank.run(&txns);
    bank.accounts.get_mut(&alice).unwrap().frozen = true;
    bank.run(&[Txn::Withdraw { from: alice, cents: 1 }, Txn::Transfer { from: bob, to: alice, cents: 1 }]);
    for (i, r) in &bank.journal {
        match r { Ok(()) => println!("#{:<2} ok", i), Err(e) => println!("#{:<2} FAILED: {}", i, e) }
    }
    for (id, a) in &bank.accounts {
        println!("{} {:<6} {:>16} {:<22} frozen={} hist={:?}", id, a.owner, Cents(a.balance.max(0) as u64), a.policy.label(), a.frozen, a.history);
    }
    let total: i64 = bank.accounts.values().map(|a| a.balance).sum();
    println!("total {} raw={}", Cents(total as u64), total);
    println!("[{:<14}] [{:^14}] [{}]", Cents(5), Cents(100_000_000_001), Cents(0));
}
