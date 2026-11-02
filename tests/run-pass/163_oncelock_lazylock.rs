use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{LazyLock, Mutex, OnceLock};
use std::thread;

static INIT_CALLS: AtomicU32 = AtomicU32::new(0);

static TABLE: LazyLock<HashMap<&'static str, u32>> = LazyLock::new(|| {
    INIT_CALLS.fetch_add(1, Ordering::SeqCst);
    let mut m = HashMap::new();
    m.insert("one", 1);
    m.insert("two", 2);
    m.insert("three", 3);
    m
});

static CONFIG: OnceLock<String> = OnceLock::new();

static REGISTRY: Mutex<Vec<String>> = Mutex::new(Vec::new());

fn config() -> &'static str {
    CONFIG.get_or_init(|| {
        INIT_CALLS.fetch_add(100, Ordering::SeqCst);
        "production".to_string()
    })
}

fn register(name: &str) -> usize {
    let mut r = REGISTRY.lock().unwrap();
    r.push(name.to_string());
    r.len()
}

fn main() {
    println!("calls before {}", INIT_CALLS.load(Ordering::SeqCst));
    println!("CONFIG set? {}", CONFIG.get().is_some());
    let hs: Vec<_> = (0..6)
        .map(|i| {
            thread::spawn(move || {
                let v = TABLE.get("two").copied().unwrap_or(0) * i;
                let c = config().len();
                (v, c)
            })
        })
        .collect();
    let mut results: Vec<(u32, usize)> = hs.into_iter().map(|h| h.join().unwrap()).collect();
    results.sort();
    println!("{:?}", results);
    println!("calls after {}", INIT_CALLS.load(Ordering::SeqCst));
    println!("config {}", config());
    println!("set again: {:?}", CONFIG.set("dev".to_string()));
    let mut keys: Vec<_> = TABLE.keys().collect();
    keys.sort();
    println!("{:?} len {}", keys, TABLE.len());
    println!("calls final {}", INIT_CALLS.load(Ordering::SeqCst));

    let cell: OnceLock<Vec<i32>> = OnceLock::new();
    println!("{:?}", cell.get());
    println!("{:?}", cell.set(vec![1, 2]));
    println!("{:?}", cell.set(vec![3]));
    println!("{:?}", cell.get());
    let mut cell2 = OnceLock::new();
    cell2.get_or_init(|| 5);
    if let Some(v) = cell2.get_mut() {
        *v += 1;
    }
    println!("{:?}", cell2.get());
    println!("{:?}", cell2.take());
    println!("{:?}", cell2.into_inner());

    let hs: Vec<_> = ["a", "b", "c", "d"].into_iter().map(|n| thread::spawn(move || register(n))).collect();
    let mut sizes: Vec<usize> = hs.into_iter().map(|h| h.join().unwrap()).collect();
    sizes.sort();
    println!("sizes {:?}", sizes);
    let mut names = REGISTRY.lock().unwrap().clone();
    names.sort();
    println!("{:?}", names);

    let lazy_local = LazyLock::new(|| (1..=5).map(|x| x * x).collect::<Vec<u32>>());
    println!("{:?} sum {}", *lazy_local, lazy_local.iter().sum::<u32>());
    let once = std::sync::Once::new();
    let mut n = 0;
    for _ in 0..3 {
        once.call_once(|| n += 1);
    }
    println!("once ran {} times, completed {}", n, once.is_completed());
}
