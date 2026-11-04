// Bank/ledger simulation: traits, enums, error types, fixed-point money, reports.
use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::ops::{Add, AddAssign, Neg, Sub, SubAssign, Mul};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
struct Cents(i64);

impl fmt::Display for Cents {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let sign = if self.0 < 0 { "-" } else { "" };
        let a = self.0.abs();
        let s = format!("{}${}.{:02}", sign, group(a / 100), a % 100);
        f.pad(&s)
    }
}

impl fmt::Debug for Cents {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Cents({})", self.0)
    }
}

fn group(n: i64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

impl Add for Cents { type Output = Cents; fn add(self, o: Cents) -> Cents { Cents(self.0 + o.0) } }
impl Sub for Cents { type Output = Cents; fn sub(self, o: Cents) -> Cents { Cents(self.0 - o.0) } }
impl Neg for Cents { type Output = Cents; fn neg(self) -> Cents { Cents(-self.0) } }
impl AddAssign for Cents { fn add_assign(&mut self, o: Cents) { self.0 += o.0; } }
impl SubAssign for Cents { fn sub_assign(&mut self, o: Cents) { self.0 -= o.0; } }
impl Mul<i64> for Cents { type Output = Cents; fn mul(self, k: i64) -> Cents { Cents(self.0 * k) } }
impl std::iter::Sum for Cents {
    fn sum<I: Iterator<Item = Cents>>(it: I) -> Cents { it.fold(Cents(0), |a, b| a + b) }
}

impl Cents {
    // basis points interest with banker's rounding
    fn apply_bps(self, bps: i64) -> Cents {
        let num = self.0 as i128 * bps as i128;
        let q = num / 10_000;
        let r = num % 10_000;
        let q = if r.abs() * 2 > 10_000 || (r.abs() * 2 == 10_000 && q % 2 != 0) {
            q + num.signum()
        } else {
            q
        };
        Cents(q as i64)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct AccountId(u32);

impl fmt::Display for AccountId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ACC-{:04}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq)]
enum LedgerError {
    InsufficientFunds { account: AccountId, needed: Cents, available: Cents },
    AccountNotFound(AccountId),
    AccountFrozen(AccountId),
    InvalidAmount(Cents),
    SameAccount,
    DailyLimitExceeded { account: AccountId, limit: Cents },
}

impl fmt::Display for LedgerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use LedgerError::*;
        match self {
            InsufficientFunds { account, needed, available } => {
                write!(f, "{}: insufficient funds (needed {}, available {})", account, needed, available)
            }
            AccountNotFound(a) => write!(f, "{} not found", a),
            AccountFrozen(a) => write!(f, "{} is frozen", a),
            InvalidAmount(c) => write!(f, "invalid amount {}", c),
            SameAccount => write!(f, "cannot transfer to the same account"),
            DailyLimitExceeded { account, limit } => write!(f, "{}: daily limit {} exceeded", account, limit),
        }
    }
}

impl std::error::Error for LedgerError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Checking,
    Savings { rate_bps: i64 },
    Credit { limit: Cents, apr_bps: i64 },
}

trait Policy {
    fn name(&self) -> String;
    fn can_withdraw(&self, balance: Cents, amount: Cents) -> bool;
    fn month_end(&self, balance: Cents) -> Option<(Cents, &'static str)>;
    fn overdraft_fee(&self) -> Cents { Cents(0) }
}

impl Policy for Kind {
    fn name(&self) -> String {
        match self {
            Kind::Checking => "checking".into(),
            Kind::Savings { rate_bps } => format!("savings@{}.{:02}%", rate_bps / 100, rate_bps % 100),
            Kind::Credit { limit, .. } => format!("credit(limit {})", limit),
        }
    }
    fn can_withdraw(&self, balance: Cents, amount: Cents) -> bool {
        match self {
            Kind::Checking => balance - amount >= Cents(-5000),
            Kind::Savings { .. } => balance >= amount,
            Kind::Credit { limit, .. } => balance - amount >= -*limit,
        }
    }
    fn month_end(&self, balance: Cents) -> Option<(Cents, &'static str)> {
        match *self {
            Kind::Checking if balance < Cents(0) => Some((Cents(-1500), "overdraft fee")),
            Kind::Checking if balance < Cents(10000) => Some((Cents(-500), "maintenance fee")),
            Kind::Checking => None,
            Kind::Savings { rate_bps } if balance > Cents(0) => Some((balance.apply_bps(rate_bps / 12), "interest")),
            Kind::Savings { .. } => None,
            Kind::Credit { apr_bps, .. } if balance < Cents(0) => {
                Some((balance.apply_bps(apr_bps / 12), "finance charge"))
            }
            Kind::Credit { .. } => None,
        }
    }
    fn overdraft_fee(&self) -> Cents {
        if let Kind::Checking = self { Cents(3500) } else { Cents(0) }
    }
}

#[derive(Debug, Clone)]
enum TxKind {
    Deposit,
    Withdrawal,
    TransferIn(AccountId),
    TransferOut(AccountId),
    Adjustment(&'static str),
}

#[derive(Debug, Clone)]
struct Tx {
    id: u64,
    day: u32,
    kind: TxKind,
    amount: Cents,
    balance_after: Cents,
}

struct Account {
    id: AccountId,
    owner: String,
    kind: Kind,
    balance: Cents,
    frozen: bool,
    history: Vec<Tx>,
    withdrawn_today: (u32, Cents),
}

struct Bank {
    accounts: BTreeMap<AccountId, Account>,
    next_id: u32,
    next_tx: u64,
    day: u32,
    errors: Vec<(u32, LedgerError)>,
    daily_limit: Cents,
}

impl Bank {
    fn new() -> Self {
        Bank { accounts: BTreeMap::new(), next_id: 1, next_tx: 1000, day: 1, errors: vec![], daily_limit: Cents(200_000) }
    }

    fn open(&mut self, owner: &str, kind: Kind, initial: Cents) -> AccountId {
        let id = AccountId(self.next_id);
        self.next_id += 1;
        self.accounts.insert(id, Account {
            id,
            owner: owner.to_string(),
            kind,
            balance: Cents(0),
            frozen: false,
            history: Vec::new(),
            withdrawn_today: (0, Cents(0)),
        });
        if initial > Cents(0) {
            self.deposit(id, initial).unwrap();
        }
        id
    }

    fn acct(&mut self, id: AccountId) -> Result<&mut Account, LedgerError> {
        let a = self.accounts.get_mut(&id).ok_or(LedgerError::AccountNotFound(id))?;
        if a.frozen {
            return Err(LedgerError::AccountFrozen(id));
        }
        Ok(a)
    }

    fn record(&mut self, id: AccountId, kind: TxKind, amount: Cents) {
        let day = self.day;
        let txid = self.next_tx;
        self.next_tx += 1;
        let a = self.accounts.get_mut(&id).unwrap();
        a.balance += amount;
        let bal = a.balance;
        a.history.push(Tx { id: txid, day, kind, amount, balance_after: bal });
    }

    fn deposit(&mut self, id: AccountId, amt: Cents) -> Result<Cents, LedgerError> {
        if amt <= Cents(0) {
            return Err(LedgerError::InvalidAmount(amt));
        }
        self.acct(id)?;
        self.record(id, TxKind::Deposit, amt);
        Ok(self.accounts[&id].balance)
    }

    fn check_withdraw(&mut self, id: AccountId, amt: Cents) -> Result<(), LedgerError> {
        if amt <= Cents(0) {
            return Err(LedgerError::InvalidAmount(amt));
        }
        let day = self.day;
        let limit = self.daily_limit;
        let a = self.acct(id)?;
        if a.withdrawn_today.0 != day {
            a.withdrawn_today = (day, Cents(0));
        }
        if a.withdrawn_today.1 + amt > limit {
            return Err(LedgerError::DailyLimitExceeded { account: id, limit });
        }
        if !a.kind.can_withdraw(a.balance, amt) {
            return Err(LedgerError::InsufficientFunds { account: id, needed: amt, available: a.balance });
        }
        Ok(())
    }

    fn withdraw(&mut self, id: AccountId, amt: Cents) -> Result<Cents, LedgerError> {
        self.check_withdraw(id, amt)?;
        self.accounts.get_mut(&id).unwrap().withdrawn_today.1 += amt;
        let before = self.accounts[&id].balance;
        self.record(id, TxKind::Withdrawal, -amt);
        let a = &self.accounts[&id];
        if before >= Cents(0) && a.balance < Cents(0) {
            let fee = a.kind.overdraft_fee();
            if fee > Cents(0) {
                self.record(id, TxKind::Adjustment("NSF fee"), -fee);
            }
        }
        Ok(self.accounts[&id].balance)
    }

    fn transfer(&mut self, from: AccountId, to: AccountId, amt: Cents) -> Result<(), LedgerError> {
        if from == to {
            return Err(LedgerError::SameAccount);
        }
        self.acct(to)?;
        self.check_withdraw(from, amt)?;
        self.accounts.get_mut(&from).unwrap().withdrawn_today.1 += amt;
        self.record(from, TxKind::TransferOut(to), -amt);
        self.record(to, TxKind::TransferIn(from), amt);
        Ok(())
    }

    fn try_op(&mut self, r: Result<(), LedgerError>) {
        if let Err(e) = r {
            self.errors.push((self.day, e));
        }
    }

    fn month_end(&mut self) {
        let ids: Vec<AccountId> = self.accounts.keys().copied().collect();
        for id in ids {
            let a = &self.accounts[&id];
            if a.frozen {
                continue;
            }
            if let Some((amt, why)) = a.kind.month_end(a.balance) {
                if amt != Cents(0) {
                    self.record(id, TxKind::Adjustment(why), amt);
                }
            }
        }
    }

    fn statement(&self, id: AccountId) {
        let a = &self.accounts[&id];
        println!("Statement for {} ({}, {}){}", a.id, a.owner, a.kind.name(), if a.frozen { " [FROZEN]" } else { "" });
        println!("  {:>5} {:>4}  {:<22} {:>12} {:>12}", "tx", "day", "description", "amount", "balance");
        for t in &a.history {
            let desc = match &t.kind {
                TxKind::Deposit => "deposit".to_string(),
                TxKind::Withdrawal => "withdrawal".to_string(),
                TxKind::TransferIn(f) => format!("transfer from {}", f),
                TxKind::TransferOut(to) => format!("transfer to {}", to),
                TxKind::Adjustment(w) => format!("adj: {}", w),
            };
            println!("  {:>5} {:>4}  {:<22} {:>12} {:>12}", t.id, t.day, desc, t.amount, t.balance_after);
        }
        let (ins, outs): (Vec<&Tx>, Vec<&Tx>) = a.history.iter().partition(|t| t.amount > Cents(0));
        println!(
            "  in: {} ({} tx)  out: {} ({} tx)  closing: {}",
            ins.iter().map(|t| t.amount).sum::<Cents>(),
            ins.len(),
            outs.iter().map(|t| t.amount).sum::<Cents>(),
            outs.len(),
            a.balance
        );
    }
}

// deterministic PRNG
struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.0 >> 33
    }
    fn range(&mut self, lo: i64, hi: i64) -> i64 {
        lo + (self.next() % ((hi - lo) as u64)) as i64
    }
}

fn main() {
    let mut bank = Bank::new();
    let alice = bank.open("Alice", Kind::Checking, Cents(150_000));
    let alice_s = bank.open("Alice", Kind::Savings { rate_bps: 425 }, Cents(1_000_000));
    let bob = bank.open("Bob", Kind::Checking, Cents(2_000));
    let carol = bank.open("Carol", Kind::Credit { limit: Cents(300_000), apr_bps: 1999 }, Cents(0));
    let dave = bank.open("Dave", Kind::Savings { rate_bps: 100 }, Cents(50_00));

    println!("{:?}", bank.withdraw(bob, Cents(6_000)));
    println!("{:?}", bank.withdraw(bob, Cents(2_000)).map_err(|e| e.to_string()));
    println!("{:?}", bank.deposit(bob, Cents(-5)));
    println!("{:?}", bank.transfer(alice, alice, Cents(1)));
    println!("{:?}", bank.transfer(alice, AccountId(99), Cents(1)));
    println!("{}", bank.withdraw(alice_s, Cents(250_000)).unwrap_err());
    bank.day = 2;
    println!("{:?}", bank.withdraw(alice_s, Cents(150_000)));

    let ids = [alice, alice_s, bob, carol, dave];
    let mut rng = Lcg(42);
    for day in 3..=30u32 {
        bank.day = day;
        for _ in 0..4 {
            let op = rng.range(0, 10);
            let a = ids[rng.range(0, ids.len() as i64) as usize];
            let amt = Cents(rng.range(1, 900) * 100 + rng.range(0, 100));
            let r = match op {
                0..=3 => bank.deposit(a, amt).map(|_| ()),
                4..=6 => bank.withdraw(a, amt).map(|_| ()),
                _ => {
                    let b = ids[rng.range(0, ids.len() as i64) as usize];
                    bank.transfer(a, b, amt)
                }
            };
            bank.try_op(r);
        }
        if day == 15 {
            bank.accounts.get_mut(&dave).unwrap().frozen = true;
        }
        if day == 20 {
            bank.accounts.get_mut(&dave).unwrap().frozen = false;
        }
    }
    bank.month_end();

    for id in ids {
        bank.statement(id);
        println!();
    }

    println!("Errors ({}):", bank.errors.len());
    let mut by_kind: HashMap<&'static str, u32> = HashMap::new();
    for (day, e) in &bank.errors {
        let k = match e {
            LedgerError::InsufficientFunds { .. } => "insufficient",
            LedgerError::AccountNotFound(_) => "notfound",
            LedgerError::AccountFrozen(_) => "frozen",
            LedgerError::InvalidAmount(_) => "invalid",
            LedgerError::SameAccount => "same",
            LedgerError::DailyLimitExceeded { .. } => "limit",
        };
        *by_kind.entry(k).or_default() += 1;
        println!("  day {:>2}: {}", day, e);
    }
    let mut kinds: Vec<_> = by_kind.into_iter().collect();
    kinds.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    println!("error kinds: {:?}", kinds);

    // owner summary
    let mut per_owner: BTreeMap<&str, (usize, Cents)> = BTreeMap::new();
    for a in bank.accounts.values() {
        let e = per_owner.entry(a.owner.as_str()).or_insert((0, Cents(0)));
        e.0 += 1;
        e.1 += a.balance;
    }
    println!("{:<8}|{:>5}|{:>14}", "owner", "accts", "net");
    println!("{}", "-".repeat(29));
    for (o, (n, net)) in &per_owner {
        println!("{:<8}|{:>5}|{:>14}", o, n, net);
    }
    let total: Cents = bank.accounts.values().map(|a| a.balance).sum();
    println!("{:<8}|{:>5}|{:>14}", "TOTAL", bank.accounts.len(), total);
    let richest = bank.accounts.values().max_by_key(|a| a.balance).unwrap();
    let poorest = bank.accounts.values().min_by_key(|a| a.balance).unwrap();
    println!("richest {} {}  poorest {} {}", richest.id, richest.balance, poorest.id, poorest.balance);
    println!("rounding: {} {} {} {}", Cents(250).apply_bps(5000), Cents(350).apply_bps(5000), Cents(-250).apply_bps(5000), Cents(12345).apply_bps(333));
    println!("[{:<14}] [{:^14}] [{:>14}]", Cents(-123456789), Cents(5), Cents(100000));
}
