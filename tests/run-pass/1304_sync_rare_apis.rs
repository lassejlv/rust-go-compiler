// Less common stable sync/thread APIs: Builder::spawn_scoped, park_timeout, OnceLock::wait,
// Mutex/RwLock with unsized data, RwLock guards across scoped threads, Condvar::notify_all,
// Arc<Mutex<dyn Trait>>, Weak::new/ptr_eq, Arc::from Vec/String/Box, mpsc::Sender in structs.
use std::fmt::Debug;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Condvar, Mutex, OnceLock, RwLock, Weak};
use std::thread;
use std::time::{Duration, Instant};

trait Counter: Send {
    fn bump(&mut self) -> u32;
}
#[derive(Debug)]
struct Simple(u32);
impl Counter for Simple {
    fn bump(&mut self) -> u32 {
        self.0 += 1;
        self.0
    }
}

struct Logger {
    tx: mpsc::Sender<String>,
    prefix: &'static str,
}
impl Logger {
    fn log(&self, m: impl Debug) {
        self.tx.send(format!("{}{:?}", self.prefix, m)).unwrap();
    }
}

fn main() {
    let mut data = vec![1, 2, 3, 4];
    let total = AtomicUsize::new(0);
    thread::scope(|s| {
        let (a, b) = data.split_at_mut(2);
        let h1 = thread::Builder::new().name("left".into()).spawn_scoped(s, || {
            a.iter_mut().for_each(|x| *x *= 10);
            total.fetch_add(a.iter().sum::<i32>() as usize, Ordering::SeqCst);
            thread::current().name().unwrap().to_string()
        }).unwrap();
        let h2 = s.spawn(|| {
            b.reverse();
            total.fetch_add(b.len(), Ordering::SeqCst);
        });
        println!("h1 name {}", h1.join().unwrap());
        h2.join().unwrap();
    });
    println!("{:?} total {}", data, total.load(Ordering::SeqCst));

    let start = Instant::now();
    thread::park_timeout(Duration::from_millis(5));
    println!("park_timeout returned (elapsed>=0: {})", start.elapsed() >= Duration::ZERO);

    let cell = Arc::new(OnceLock::<String>::new());
    let c2 = cell.clone();
    let waiter = thread::spawn(move || c2.wait().clone());
    thread::sleep(Duration::from_millis(5));
    cell.set("ready".to_string()).unwrap();
    println!("waited: {}", waiter.join().unwrap());

    let m: Arc<Mutex<[u8]>> = Arc::new(Mutex::new([1u8, 2, 3]));
    m.lock().unwrap()[1] = 20;
    println!("unsized mutex {:?}", &*m.lock().unwrap());
    let rw: Box<RwLock<[i32]>> = Box::new(RwLock::new([5, 6]));
    rw.write().unwrap().swap(0, 1);
    println!("unsized rwlock {:?}", &*rw.read().unwrap());

    let dynm: Arc<Mutex<dyn Counter>> = Arc::new(Mutex::new(Simple(0)));
    let hs: Vec<_> = (0..4).map(|_| {
        let d = dynm.clone();
        thread::spawn(move || {
            for _ in 0..25 {
                d.lock().unwrap().bump();
            }
        })
    }).collect();
    for h in hs {
        h.join().unwrap();
    }
    println!("dyn counter {}", dynm.lock().unwrap().bump());

    let shared = RwLock::new(vec![0u32; 4]);
    thread::scope(|s| {
        for i in 0..4 {
            let shared = &shared;
            s.spawn(move || {
                let r = shared.read().unwrap();
                let _len = r.len();
                drop(r);
                shared.write().unwrap()[i] = i as u32 * 3;
            });
        }
    });
    println!("{:?}", shared.into_inner().unwrap());

    let state = Arc::new((Mutex::new(0u32), Condvar::new()));
    let hs: Vec<_> = (0..3).map(|i| {
        let st = state.clone();
        thread::spawn(move || {
            let (m, cv) = &*st;
            let g = cv.wait_while(m.lock().unwrap(), |v| *v == 0).unwrap();
            *g + i
        })
    }).collect();
    thread::sleep(Duration::from_millis(5));
    *state.0.lock().unwrap() = 100;
    state.1.notify_all();
    let mut r: Vec<u32> = hs.into_iter().map(|h| h.join().unwrap()).collect();
    r.sort();
    println!("notify_all {:?}", r);

    let w: Weak<u8> = Weak::new();
    println!("weak new upgrade {:?} strong {}", w.upgrade(), w.strong_count());
    let a = Arc::new(5u8);
    let w1 = Arc::downgrade(&a);
    let w2 = w1.clone();
    println!("weak ptr_eq {} {}", w1.ptr_eq(&w2), w1.ptr_eq(&Weak::new()));
    let av: Arc<[i32]> = Arc::from(vec![1, 2, 3]);
    let as_: Arc<str> = Arc::from(String::from("arc string"));
    let ab: Arc<Vec<u8>> = Arc::from(Box::new(vec![9u8]));
    println!("{:?} {} {:?}", av, as_, ab);

    let (tx, rx) = mpsc::channel();
    let loggers: Vec<Logger> = ["a:", "b:"].iter().map(|p| Logger { tx: tx.clone(), prefix: p }).collect();
    drop(tx);
    thread::scope(|s| {
        for (i, l) in loggers.iter().enumerate() {
            s.spawn(move || l.log((i, "msg")));
        }
    });
    drop(loggers);
    let mut msgs: Vec<String> = rx.into_iter().collect();
    msgs.sort();
    println!("{:?}", msgs);
}
