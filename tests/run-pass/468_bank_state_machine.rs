// Bank account state machine: enum states with data (Pending/Active/Frozen/Closed), transitions
// consuming self and returning Result<Self, (Self, Error)>, an event log, transfers between accounts
// in a HashMap with two-phase borrow (remove/insert), overdraft rules, and an audit summary.
use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
enum State {
    Pending { kyc_docs: u8 },
    Active { balance: i64, overdraft: i64 },
    Frozen { balance: i64, reason: String },
    Closed { final_balance: i64 },
}

#[derive(Debug, Clone, PartialEq)]
enum Error {
    InvalidTransition(&'static str, String),
    Insufficient { need: i64, available: i64 },
    NotFound(u32),
    SameAccount,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::InvalidTransition(op, st) => write!(f, "cannot {} while {}", op, st),
            Error::Insufficient { need, available } => write!(f, "insufficient funds: need {} have {}", need, available),
            Error::NotFound(id) => write!(f, "account {} not found", id),
            Error::SameAccount => write!(f, "cannot transfer to same account"),
        }
    }
}

impl State {
    fn label(&self) -> String {
        match self {
            State::Pending { .. } => "pending".into(),
            State::Active { .. } => "active".into(),
            State::Frozen { reason, .. } => format!("frozen({})", reason),
            State::Closed { .. } => "closed".into(),
        }
    }
    fn submit_doc(self) -> Result<State, Error> {
        match self {
            State::Pending { kyc_docs } if kyc_docs + 1 >= 2 => Ok(State::Active { balance: 0, overdraft: 100 }),
            State::Pending { kyc_docs } => Ok(State::Pending { kyc_docs: kyc_docs + 1 }),
            other => Err(Error::InvalidTransition("submit doc", other.label())),
        }
    }
    fn deposit(&mut self, amt: i64) -> Result<i64, Error> {
        match self {
            State::Active { balance, .. } | State::Frozen { balance, .. } => {
                *balance += amt;
                Ok(*balance)
            }
            other => Err(Error::InvalidTransition("deposit", other.label())),
        }
    }
    fn withdraw(&mut self, amt: i64) -> Result<i64, Error> {
        match self {
            State::Active { balance, overdraft } => {
                if *balance - amt < -*overdraft {
                    return Err(Error::Insufficient { need: amt, available: *balance + *overdraft });
                }
                *balance -= amt;
                Ok(*balance)
            }
            other => Err(Error::InvalidTransition("withdraw", other.label())),
        }
    }
    fn freeze(self, reason: &str) -> Result<State, Error> {
        match self {
            State::Active { balance, .. } => Ok(State::Frozen { balance, reason: reason.into() }),
            other => Err(Error::InvalidTransition("freeze", other.label())),
        }
    }
    fn unfreeze(self) -> Result<State, Error> {
        match self {
            State::Frozen { balance, .. } => Ok(State::Active { balance, overdraft: 0 }),
            other => Err(Error::InvalidTransition("unfreeze", other.label())),
        }
    }
    fn close(self) -> Result<State, Error> {
        match self {
            State::Active { balance, .. } if balance >= 0 => Ok(State::Closed { final_balance: balance }),
            State::Active { balance, .. } => Err(Error::Insufficient { need: -balance, available: 0 }),
            State::Pending { .. } => Ok(State::Closed { final_balance: 0 }),
            other => Err(Error::InvalidTransition("close", other.label())),
        }
    }
}

struct Bank {
    accounts: BTreeMap<u32, State>,
    log: Vec<String>,
}

impl Bank {
    fn apply(&mut self, id: u32, op: &str, f: impl FnOnce(State) -> Result<State, Error>) {
        let Some(st) = self.accounts.remove(&id) else {
            self.log.push(format!("{} #{}: {}", op, id, Error::NotFound(id)));
            return;
        };
        let backup = st.clone();
        match f(st) {
            Ok(new) => {
                self.log.push(format!("{} #{}: {} -> {}", op, id, backup.label(), new.label()));
                self.accounts.insert(id, new);
            }
            Err(e) => {
                self.log.push(format!("{} #{}: ERROR {}", op, id, e));
                self.accounts.insert(id, backup);
            }
        }
    }
    fn transfer(&mut self, from: u32, to: u32, amt: i64) -> Result<(), Error> {
        if from == to {
            return Err(Error::SameAccount);
        }
        let mut src = self.accounts.remove(&from).ok_or(Error::NotFound(from))?;
        let result: Result<(), Error> = (|| {
            let dst = self.accounts.get_mut(&to).ok_or(Error::NotFound(to))?;
            let mut probe = dst.clone();
            probe.deposit(amt)?;
            src.withdraw(amt)?;
            dst.deposit(amt)?;
            Ok(())
        })();
        self.accounts.insert(from, src);
        self.log.push(format!("transfer {} {}->{}: {}", amt, from, to, match &result { Ok(()) => "ok".to_string(), Err(e) => e.to_string() }));
        result
    }
}

fn main() {
    let mut bank = Bank { accounts: BTreeMap::new(), log: vec![] };
    for id in 1..=3 {
        bank.accounts.insert(id, State::Pending { kyc_docs: 0 });
    }
    bank.apply(1, "doc", State::submit_doc);
    bank.apply(1, "doc", State::submit_doc);
    bank.apply(2, "doc", State::submit_doc);
    bank.apply(2, "doc", State::submit_doc);
    bank.apply(1, "doc", State::submit_doc);
    bank.apply(9, "doc", State::submit_doc);
    bank.apply(1, "deposit", |mut s| s.deposit(500).map(|_| s));
    bank.apply(3, "deposit", |mut s| s.deposit(5).map(|_| s));
    bank.apply(2, "withdraw", |mut s| s.withdraw(80).map(|_| s));
    bank.apply(2, "withdraw", |mut s| s.withdraw(30).map(|_| s));
    let _ = bank.transfer(1, 2, 200);
    let _ = bank.transfer(2, 1, 1000);
    let _ = bank.transfer(1, 3, 10);
    let _ = bank.transfer(1, 1, 10);
    let _ = bank.transfer(1, 7, 10);
    bank.apply(1, "freeze", |s| s.freeze("audit"));
    bank.apply(1, "withdraw", |mut s| s.withdraw(1).map(|_| s));
    bank.apply(1, "deposit", |mut s| s.deposit(1).map(|_| s));
    bank.apply(1, "close", State::close);
    bank.apply(1, "unfreeze", State::unfreeze);
    bank.apply(2, "close", State::close);
    bank.apply(3, "close", State::close);
    bank.apply(3, "close", State::close);
    for l in &bank.log {
        println!("{}", l);
    }
    for (id, st) in &bank.accounts {
        println!("#{} {:?}", id, st);
    }
    let total: i64 = bank.accounts.values().map(|s| match s {
        State::Active { balance, .. } | State::Frozen { balance, .. } => *balance,
        State::Closed { final_balance } => *final_balance,
        State::Pending { .. } => 0,
    }).sum();
    println!("total holdings {}", total);
}
