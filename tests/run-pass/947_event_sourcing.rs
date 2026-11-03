// Event-sourced aggregates: generic store/replay over an Aggregate trait with
// associated Command/Event/Error types.
use std::collections::BTreeMap;
use std::fmt::{self, Debug, Display};

trait Aggregate: Default + Debug + Clone {
    type Command: Debug;
    type Event: Debug + Clone;
    type Error: Debug + Display;
    const KIND: &'static str;

    fn handle(&self, cmd: &Self::Command) -> Result<Vec<Self::Event>, Self::Error>;
    fn apply(&mut self, ev: &Self::Event);

    fn replay<'a, I>(events: I) -> Self
    where
        I: IntoIterator<Item = &'a Self::Event>,
        Self::Event: 'a,
    {
        let mut s = Self::default();
        for e in events { s.apply(e); }
        s
    }
}

#[derive(Debug)]
struct Envelope<E> { seq: u64, stream: String, event: E }

struct Store<A: Aggregate> {
    log: Vec<Envelope<A::Event>>,
    snapshots: BTreeMap<String, (u64, A)>,
    next_seq: u64,
}

#[derive(Debug)]
enum StoreError<E> { Domain(E), Conflict { expected: u64, actual: u64 } }
impl<E: Display> Display for StoreError<E> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            StoreError::Domain(e) => write!(f, "domain: {}", e),
            StoreError::Conflict { expected, actual } => write!(f, "conflict: expected v{} got v{}", expected, actual),
        }
    }
}
impl<E> From<E> for StoreError<E> { fn from(e: E) -> Self { StoreError::Domain(e) } }

impl<A: Aggregate> Store<A> {
    fn new() -> Self { Store { log: Vec::new(), snapshots: BTreeMap::new(), next_seq: 1 } }

    fn events_for<'s>(&'s self, stream: &'s str) -> impl Iterator<Item = &'s A::Event> + 's {
        self.log.iter().filter(move |e| e.stream == stream).map(|e| &e.event)
    }

    fn version(&self, stream: &str) -> u64 { self.events_for(stream).count() as u64 }

    fn load(&self, stream: &str) -> A {
        match self.snapshots.get(stream) {
            Some((v, snap)) => {
                let mut a = snap.clone();
                for e in self.events_for(stream).skip(*v as usize) { a.apply(e); }
                a
            }
            None => A::replay(self.events_for(stream)),
        }
    }

    fn execute(&mut self, stream: &str, expected: Option<u64>, cmd: A::Command) -> Result<usize, StoreError<A::Error>> {
        let actual = self.version(stream);
        if let Some(exp) = expected {
            if exp != actual { return Err(StoreError::Conflict { expected: exp, actual }); }
        }
        let state = self.load(stream);
        let evs = state.handle(&cmd)?;
        let n = evs.len();
        for e in evs {
            self.log.push(Envelope { seq: self.next_seq, stream: stream.to_string(), event: e });
            self.next_seq += 1;
        }
        Ok(n)
    }

    fn snapshot(&mut self, stream: &str) {
        let a = self.load(stream);
        let v = self.version(stream);
        self.snapshots.insert(stream.to_string(), (v, a));
    }
}

// ---- Bank account aggregate ----
#[derive(Debug, Default, Clone)]
struct Account { open: bool, balance: i64, owner: String, txns: u32 }

#[derive(Debug)]
enum AccountCmd { Open { owner: String }, Deposit(i64), Withdraw(i64), Close }

#[derive(Debug, Clone)]
enum AccountEvent { Opened(String), Deposited(i64), Withdrew(i64), Overdrawn(i64), Closed }

#[derive(Debug)]
enum AccountError { AlreadyOpen, NotOpen, InvalidAmount(i64), NonZeroBalance(i64) }
impl Display for AccountError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AccountError::AlreadyOpen => write!(f, "account already open"),
            AccountError::NotOpen => write!(f, "account not open"),
            AccountError::InvalidAmount(a) => write!(f, "invalid amount {}", a),
            AccountError::NonZeroBalance(b) => write!(f, "cannot close with balance {}", b),
        }
    }
}

impl Aggregate for Account {
    type Command = AccountCmd;
    type Event = AccountEvent;
    type Error = AccountError;
    const KIND: &'static str = "account";

    fn handle(&self, cmd: &AccountCmd) -> Result<Vec<AccountEvent>, AccountError> {
        use AccountCmd::*;
        match cmd {
            Open { owner } => if self.open { Err(AccountError::AlreadyOpen) } else { Ok(vec![AccountEvent::Opened(owner.clone())]) },
            _ if !self.open => Err(AccountError::NotOpen),
            Deposit(a) | Withdraw(a) if *a <= 0 => Err(AccountError::InvalidAmount(*a)),
            Deposit(a) => Ok(vec![AccountEvent::Deposited(*a)]),
            Withdraw(a) => {
                let mut v = vec![AccountEvent::Withdrew(*a)];
                if self.balance - a < 0 { v.push(AccountEvent::Overdrawn(self.balance - a)); }
                Ok(v)
            }
            Close => if self.balance != 0 { Err(AccountError::NonZeroBalance(self.balance)) } else { Ok(vec![AccountEvent::Closed]) },
        }
    }

    fn apply(&mut self, ev: &AccountEvent) {
        match ev {
            AccountEvent::Opened(o) => { self.open = true; self.owner = o.clone(); }
            AccountEvent::Deposited(a) => { self.balance += a; self.txns += 1; }
            AccountEvent::Withdrew(a) => { self.balance -= a; self.txns += 1; }
            AccountEvent::Overdrawn(_) => {}
            AccountEvent::Closed => self.open = false,
        }
    }
}

// ---- Counter aggregate (to show the store is generic) ----
#[derive(Debug, Default, Clone)]
struct Counter { value: u32, max_seen: u32 }
#[derive(Debug, Clone)] enum CounterEv { Inc(u32), Reset }
#[derive(Debug)] struct CounterErr(String);
impl Display for CounterErr { fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "counter: {}", self.0) } }
impl Aggregate for Counter {
    type Command = i32;
    type Event = CounterEv;
    type Error = CounterErr;
    const KIND: &'static str = "counter";
    fn handle(&self, cmd: &i32) -> Result<Vec<CounterEv>, CounterErr> {
        match *cmd {
            0 => Ok(vec![CounterEv::Reset]),
            n if n > 0 => Ok((0..n).map(|_| CounterEv::Inc(1)).collect()),
            n => Err(CounterErr(format!("negative {}", n))),
        }
    }
    fn apply(&mut self, ev: &CounterEv) {
        match ev { CounterEv::Inc(k) => { self.value += k; self.max_seen = self.max_seen.max(self.value); } CounterEv::Reset => self.value = 0 }
    }
}

fn run<A: Aggregate>(store: &mut Store<A>, stream: &str, cmds: Vec<(Option<u64>, A::Command)>) {
    for (exp, c) in cmds {
        let desc = format!("{:?}", c);
        match store.execute(stream, exp, c) {
            Ok(n) => println!("[{}:{}] {} -> {} event(s), v{}", A::KIND, stream, desc, n, store.version(stream)),
            Err(e) => println!("[{}:{}] {} -> ERROR {}", A::KIND, stream, desc, e),
        }
    }
}

fn main() {
    let mut bank: Store<Account> = Store::new();
    run(&mut bank, "acc-1", vec![
        (None, AccountCmd::Deposit(5)),
        (Some(0), AccountCmd::Open { owner: "ann".into() }),
        (None, AccountCmd::Open { owner: "bob".into() }),
        (None, AccountCmd::Deposit(100)),
        (None, AccountCmd::Withdraw(30)),
        (Some(1), AccountCmd::Deposit(1)),
        (None, AccountCmd::Withdraw(-4)),
    ]);
    bank.snapshot("acc-1");
    run(&mut bank, "acc-1", vec![(None, AccountCmd::Withdraw(100)), (None, AccountCmd::Close), (None, AccountCmd::Deposit(30)), (Some(6), AccountCmd::Close)]);
    run(&mut bank, "acc-2", vec![(None, AccountCmd::Open { owner: "cy".into() }), (None, AccountCmd::Deposit(7))]);
    println!("{:?}", bank.load("acc-1"));
    println!("{:?}", bank.load("acc-2"));
    for e in bank.log.iter().filter(|e| e.seq % 3 == 0) { println!("  #{} {} {:?}", e.seq, e.stream, e.event); }
    let from_scratch = Account::replay(bank.events_for("acc-1"));
    println!("replay balance {} txns {} open {}", from_scratch.balance, from_scratch.txns, from_scratch.open);

    let mut counters: Store<Counter> = Store::new();
    run(&mut counters, "c", vec![(None, 3), (None, -2), (None, 0), (Some(4), 2)]);
    println!("{:?} log={}", counters.load("c"), counters.log.len());
}
