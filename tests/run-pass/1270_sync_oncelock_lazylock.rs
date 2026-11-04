// OnceLock, LazyLock, OnceCell, LazyCell, std::sync::Once, initialized racing from threads.
use std::cell::{LazyCell, OnceCell};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{LazyLock, Once, OnceLock};
use std::thread;

static INIT_COUNT: AtomicUsize = AtomicUsize::new(0);
static CONFIG: OnceLock<HashMap<&'static str, u32>> = OnceLock::new();
static SQUARES: LazyLock<Vec<u64>> = LazyLock::new(|| {
    INIT_COUNT.fetch_add(1, Ordering::SeqCst);
    (0..10).map(|x| x * x).collect()
});
static START: Once = Once::new();
static mut STARTED_VALUE: u32 = 0;

fn config() -> &'static HashMap<&'static str, u32> {
    CONFIG.get_or_init(|| {
        INIT_COUNT.fetch_add(100, Ordering::SeqCst);
        let mut m = HashMap::new();
        m.insert("threads", 4);
        m.insert("retries", 3);
        m
    })
}

fn main() {
    println!("before: {:?}", CONFIG.get().is_some());
    let hs: Vec<_> = (0..8)
        .map(|i| {
            thread::spawn(move || {
                let c = config();
                let s = SQUARES[i];
                START.call_once(|| unsafe { STARTED_VALUE += 1 });
                c["threads"] as u64 + s
            })
        })
        .collect();
    let mut out: Vec<u64> = hs.into_iter().map(|h| h.join().unwrap()).collect();
    out.sort();
    println!("{:?}", out);
    println!("init count {}", INIT_COUNT.load(Ordering::SeqCst));
    println!("once done {} value {}", START.is_completed(), unsafe { STARTED_VALUE });
    println!("retries {}", config()["retries"]);
    println!("sum squares {}", SQUARES.iter().sum::<u64>());

    let ol: OnceLock<String> = OnceLock::new();
    println!("set1 {:?}", ol.set("first".into()));
    println!("set2 {:?}", ol.set("second".into()));
    println!("get {:?}", ol.get());
    let mut ol2 = ol;
    ol2.get_mut().unwrap().push_str("-mut");
    println!("take {:?} then {:?}", ol2.take(), ol2.get());
    println!("into_inner {:?}", OnceLock::from(5).into_inner());

    let cell: OnceCell<Vec<i32>> = OnceCell::new();
    let v = cell.get_or_init(|| vec![1, 2, 3]);
    println!("cell {:?} again {:?}", v, cell.get_or_init(|| vec![9]));
    let calls = std::cell::Cell::new(0);
    let lazy = LazyCell::new(|| {
        calls.set(calls.get() + 1);
        "computed".len()
    });
    println!("lazy {} {} calls={}", *lazy, *lazy + 1, calls.get());
    let ll = LazyLock::new(|| String::from("abc"));
    println!("lazylock deref len {}", ll.len());
    println!("force {}", LazyLock::force(&ll));
}
