// Actor-style bank simulation: worker threads own account shards and talk
// over mpsc channels; a coordinator routes transfers with two-phase commit,
// replies come back via per-request channels. Output is deterministic after join.
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::fmt;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, Sender, SyncSender};
use std::sync::{Arc, Barrier, Condvar, Mutex, RwLock};
use std::thread::{self, JoinHandle};
use std::time::Duration;

type AccountId = u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct TxId(u64);

#[derive(Debug)]
enum Msg {
    Open { id: AccountId, initial: i64 },
    Prepare { tx: TxId, id: AccountId, delta: i64, reply: Sender<Vote> },
    Commit { tx: TxId, id: AccountId, done: Sender<()> },
    Abort { tx: TxId, id: AccountId, done: Sender<()> },
    Balance { id: AccountId, reply: Sender<Option<i64>> },
    Snapshot { reply: SyncSender<Vec<(AccountId, i64)>> },
    Stop,
}

#[derive(Debug, Clone, PartialEq)]
enum Vote {
    Yes(TxId),
    No(TxId, Reason),
}

#[derive(Debug, Clone, PartialEq)]
enum Reason {
    NoSuchAccount(AccountId),
    Insufficient { have: i64, need: i64 },
    Locked(AccountId),
}

impl fmt::Display for Reason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Reason::NoSuchAccount(a) => write!(f, "no such account #{}", a),
            Reason::Insufficient { have, need } => write!(f, "insufficient funds ({} < {})", have, need),
            Reason::Locked(a) => write!(f, "account #{} locked", a),
        }
    }
}

struct Shard {
    idx: usize,
    balances: HashMap<AccountId, i64>,
    pending: HashMap<(TxId, AccountId), i64>,
    locked: HashMap<AccountId, TxId>,
    log: Vec<String>,
    ops: Arc<AtomicU64>,
}

impl Shard {
    fn handle(&mut self, m: Msg) -> bool {
        self.ops.fetch_add(1, Ordering::Relaxed);
        match m {
            Msg::Open { id, initial } => {
                self.balances.insert(id, initial);
            }
            Msg::Prepare { tx, id, delta, reply } => {
                let vote = match self.balances.get(&id) {
                    None => Vote::No(tx, Reason::NoSuchAccount(id)),
                    Some(_) if self.locked.contains_key(&id) => Vote::No(tx, Reason::Locked(id)),
                    Some(&bal) if bal + delta < 0 => Vote::No(tx, Reason::Insufficient { have: bal, need: -delta }),
                    Some(_) => {
                        self.pending.insert((tx, id), delta);
                        self.locked.insert(id, tx);
                        Vote::Yes(tx)
                    }
                };
                let _ = reply.send(vote);
            }
            Msg::Commit { tx, id, done } => {
                let _ = done.send(());
                if let Some(delta) = self.pending.remove(&(tx, id)) {
                    *self.balances.get_mut(&id).unwrap() += delta;
                    self.locked.remove(&id);
                    self.log.push(format!("{:?} {}{:+}", tx, id, delta));
                }
            }
            Msg::Abort { tx, id, done } => {
                if self.pending.remove(&(tx, id)).is_some() {
                    self.locked.remove(&id);
                }
                let _ = done.send(());
            }
            Msg::Balance { id, reply } => {
                let _ = reply.send(self.balances.get(&id).copied());
            }
            Msg::Snapshot { reply } => {
                let mut v: Vec<_> = self.balances.iter().map(|(&k, &v)| (k, v)).collect();
                v.sort_unstable();
                reply.send(v).unwrap();
            }
            Msg::Stop => return false,
        }
        true
    }
}

struct Bank {
    shards: Vec<Sender<Msg>>,
    handles: Vec<JoinHandle<(usize, Vec<String>)>>,
    next_tx: AtomicU64,
    ops: Arc<AtomicU64>,
}

impl Bank {
    fn new(n: usize) -> Bank {
        let ops = Arc::new(AtomicU64::new(0));
        let mut shards = Vec::new();
        let mut handles = Vec::new();
        for idx in 0..n {
            let (tx, rx): (Sender<Msg>, Receiver<Msg>) = mpsc::channel();
            let ops = Arc::clone(&ops);
            let h = thread::Builder::new()
                .name(format!("shard-{}", idx))
                .spawn(move || {
                    let mut s = Shard { idx, balances: HashMap::new(), pending: HashMap::new(), locked: HashMap::new(), log: Vec::new(), ops };
                    for m in rx.iter() {
                        if !s.handle(m) {
                            break;
                        }
                    }
                    assert_eq!(thread::current().name(), Some(format!("shard-{}", idx).as_str()));
                    (s.idx, s.log)
                })
                .expect("spawn");
            shards.push(tx);
            handles.push(h);
        }
        Bank { shards, handles, next_tx: AtomicU64::new(1), ops }
    }
    fn shard(&self, id: AccountId) -> &Sender<Msg> {
        &self.shards[id as usize % self.shards.len()]
    }
    fn open(&self, id: AccountId, initial: i64) {
        self.shard(id).send(Msg::Open { id, initial }).unwrap();
    }
    fn balance(&self, id: AccountId) -> Option<i64> {
        let (tx, rx) = mpsc::channel();
        self.shard(id).send(Msg::Balance { id, reply: tx }).unwrap();
        rx.recv().unwrap()
    }
    fn transfer(&self, from: AccountId, to: AccountId, amount: i64) -> Result<TxId, Reason> {
        let tx = TxId(self.next_tx.fetch_add(1, Ordering::SeqCst));
        let (vtx, vrx) = mpsc::channel();
        let legs = [(from, -amount), (to, amount)];
        for &(id, delta) in &legs {
            self.shard(id).send(Msg::Prepare { tx, id, delta, reply: vtx.clone() }).unwrap();
        }
        drop(vtx);
        let votes: Vec<Vote> = vrx.iter().collect();
        assert_eq!(votes.len(), 2);
        let failure = votes.iter().find_map(|v| match v {
            Vote::No(_, r) => Some(r.clone()),
            Vote::Yes(_) => None,
        });
        let (dtx, drx) = mpsc::channel();
        for &(id, _) in &legs {
            let done = dtx.clone();
            let m = if failure.is_some() { Msg::Abort { tx, id, done } } else { Msg::Commit { tx, id, done } };
            self.shard(id).send(m).unwrap();
        }
        drop(dtx);
        assert_eq!(drx.iter().count(), 2);
        match failure {
            Some(r) => Err(r),
            None => Ok(tx),
        }
    }
    fn snapshot(&self) -> BTreeMap<AccountId, i64> {
        let mut all = BTreeMap::new();
        for s in &self.shards {
            let (tx, rx) = mpsc::sync_channel(1);
            s.send(Msg::Snapshot { reply: tx }).unwrap();
            all.extend(rx.recv().unwrap());
        }
        all
    }
    fn shutdown(self) -> Vec<(usize, Vec<String>)> {
        for s in &self.shards {
            s.send(Msg::Stop).unwrap();
        }
        let mut out: Vec<_> = self.handles.into_iter().map(|h| h.join().unwrap()).collect();
        out.sort_by_key(|(i, _)| *i);
        out
    }
}

// A bounded work queue built on Mutex + Condvar, used by a pool of "clients".
struct WorkQueue<T> {
    inner: Mutex<(VecDeque<T>, bool)>,
    not_empty: Condvar,
    not_full: Condvar,
    cap: usize,
}

impl<T> WorkQueue<T> {
    fn new(cap: usize) -> Self {
        WorkQueue { inner: Mutex::new((VecDeque::new(), false)), not_empty: Condvar::new(), not_full: Condvar::new(), cap }
    }
    fn push(&self, t: T) {
        let mut g = self.inner.lock().unwrap();
        while g.0.len() >= self.cap {
            g = self.not_full.wait(g).unwrap();
        }
        g.0.push_back(t);
        self.not_empty.notify_one();
    }
    fn close(&self) {
        self.inner.lock().unwrap().1 = true;
        self.not_empty.notify_all();
    }
    fn pop(&self) -> Option<T> {
        let mut g = self.inner.lock().unwrap();
        loop {
            if let Some(t) = g.0.pop_front() {
                self.not_full.notify_one();
                return Some(t);
            }
            if g.1 {
                return None;
            }
            g = self.not_empty.wait_timeout(g, Duration::from_millis(50)).unwrap().0;
        }
    }
}

struct Lcg(u64);
impl Lcg {
    fn next(&mut self, n: u64) -> u64 {
        self.0 = self.0.wrapping_mul(2862933555777941757).wrapping_add(3037000493);
        (self.0 >> 32) % n
    }
}

fn main() {
    let bank = Arc::new(Bank::new(4));
    const N: u32 = 24;
    for id in 0..N {
        bank.open(id, 1000 + id as i64 * 10);
    }
    let initial_total: i64 = (0..N).map(|id| bank.balance(id).unwrap()).sum();
    println!("initial total {}", initial_total);
    println!("missing account: {:?}", bank.balance(999));

    // sequential, deterministic transfers
    let mut rng = Lcg(7);
    let mut outcomes: BTreeMap<String, usize> = BTreeMap::new();
    for _ in 0..300 {
        let from = rng.next(N as u64 + 1) as u32;
        let to = rng.next(N as u64) as u32;
        let amt = rng.next(700) as i64;
        let key = match bank.transfer(from, to, amt) {
            Ok(_) => "ok".to_string(),
            Err(Reason::Insufficient { .. }) => "insufficient".to_string(),
            Err(r) => r.to_string(),
        };
        *outcomes.entry(key).or_insert(0) += 1;
    }
    println!("{:?}", outcomes);
    let snap = bank.snapshot();
    let total: i64 = snap.values().sum();
    println!("after sequential: total {} min {:?} max {:?}", total, snap.iter().min_by_key(|(_, v)| **v), snap.iter().max_by_key(|(k, v)| (**v, std::cmp::Reverse(**k))));

    // concurrent clients: each client moves money around a ring; the ring sums are conserved
    let queue: Arc<WorkQueue<(u32, u32, i64)>> = Arc::new(WorkQueue::new(8));
    let ok = Arc::new(AtomicUsize::new(0));
    let failed = Arc::new(AtomicUsize::new(0));
    let barrier = Arc::new(Barrier::new(5));
    let seen_threads = Arc::new(RwLock::new(Vec::<String>::new()));
    let mut clients = vec![];
    for c in 0..4 {
        let (q, b, ok, failed, bank, seen) = (queue.clone(), barrier.clone(), ok.clone(), failed.clone(), bank.clone(), seen_threads.clone());
        clients.push(thread::spawn(move || {
            b.wait();
            seen.write().unwrap().push(format!("client-{}", c));
            let mut mine = 0;
            while let Some((f, t, a)) = q.pop() {
                // retry on lock conflicts
                let mut attempts = 0;
                loop {
                    attempts += 1;
                    match bank.transfer(f, t, a) {
                        Ok(_) => {
                            ok.fetch_add(1, Ordering::SeqCst);
                            break;
                        }
                        Err(Reason::Locked(_)) if attempts < 1000 => thread::yield_now(),
                        Err(_) => {
                            failed.fetch_add(1, Ordering::SeqCst);
                            break;
                        }
                    }
                }
                mine += 1;
            }
            mine
        }));
    }
    barrier.wait();
    for i in 0..400u32 {
        let from = i % N;
        let to = (i * 7 + 3) % N;
        if from != to {
            queue.push((from, to, (i % 50) as i64));
        }
    }
    queue.close();
    let per_client: Vec<usize> = clients.into_iter().map(|h| h.join().unwrap()).collect();
    let processed: usize = per_client.iter().sum();
    let snap2 = bank.snapshot();
    let total2: i64 = snap2.values().sum();
    println!("concurrent processed {} (ok+failed = {}), total {}", processed, ok.load(Ordering::SeqCst) + failed.load(Ordering::SeqCst), total2);
    let mut seen = seen_threads.read().unwrap().clone();
    seen.sort();
    println!("clients seen: {:?}", seen);

    // scoped threads computing statistics over the snapshot
    let values: Vec<i64> = snap2.values().copied().collect();
    let (sum, sq) = thread::scope(|s| {
        let h1 = s.spawn(|| values.iter().sum::<i64>());
        let h2 = s.spawn(|| values.iter().map(|v| v * v).sum::<i64>());
        (h1.join().unwrap(), h2.join().unwrap())
    });
    let n = values.len() as f64;
    let mean = sum as f64 / n;
    let var = sq as f64 / n - mean * mean;
    println!("mean {:.2} var>=0 {}", mean, var >= 0.0);

    let bank = Arc::try_unwrap(bank).ok().expect("bank still shared");
    let total_ops = bank.ops.clone();
    let logs = bank.shutdown();
    for (i, log) in &logs {
        println!("shard {} committed {} legs, first {:?}", i, log.len(), log.first());
    }
    println!("total shard ops > 1000: {}", total_ops.load(Ordering::Relaxed) > 1000);
    let (tx, rx) = mpsc::channel::<i32>();
    drop(tx);
    println!("recv on closed: {:?}, try_recv: {:?}", rx.recv().is_err(), rx.try_recv());
    let (stx, srx) = mpsc::sync_channel::<u8>(0);
    let h = thread::spawn(move || {
        for i in 0..5 {
            stx.send(i).unwrap();
        }
    });
    let got: Vec<u8> = srx.iter().collect();
    h.join().unwrap();
    println!("rendezvous got {:?}; recv_timeout {:?}", got, srx.recv_timeout(Duration::from_millis(5)));
}
