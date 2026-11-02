// Arc<Mutex<T>> shared across spawned threads joined deterministically; results gathered via
// join handles in spawn order; thread::scope borrowing local data; atomic counters; Mutex poisoning
// observed after a panicking thread; into_inner/get_mut; Arc::strong_count after joins.
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let total = Arc::new(Mutex::new(0u64));
    let hist = Arc::new(Mutex::new(BTreeMap::new()));
    let mut handles = Vec::new();
    for t in 0..4u64 {
        let total = Arc::clone(&total);
        let hist = Arc::clone(&hist);
        handles.push(thread::spawn(move || {
            let mut local = 0;
            for i in 0..1000u64 {
                if i % (t + 2) == 0 {
                    local += i;
                }
            }
            *total.lock().unwrap() += local;
            hist.lock().unwrap().insert(t, local);
            local * 2
        }));
    }
    let rets: Vec<u64> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    println!("returns {:?}", rets);
    println!("total {}", *total.lock().unwrap());
    println!("hist {:?}", hist.lock().unwrap());
    println!("strong {}", Arc::strong_count(&total));

    // scoped threads borrowing local data
    let data: Vec<u32> = (1..=100).collect();
    let chunks: Vec<&[u32]> = data.chunks(25).collect();
    let sums: Vec<u32> = thread::scope(|s| {
        let hs: Vec<_> = chunks.iter().map(|c| s.spawn(move || c.iter().sum::<u32>())).collect();
        hs.into_iter().map(|h| h.join().unwrap()).collect()
    });
    println!("chunk sums {:?} = {}", sums, sums.iter().sum::<u32>());

    let mut grid = vec![0u32; 8];
    thread::scope(|s| {
        for (i, cell) in grid.iter_mut().enumerate() {
            s.spawn(move || {
                *cell = (i as u32 + 1).pow(3);
            });
        }
    });
    println!("grid {:?}", grid);

    let counter = Arc::new(AtomicUsize::new(0));
    let hs: Vec<_> = (0..8)
        .map(|_| {
            let c = Arc::clone(&counter);
            thread::spawn(move || {
                for _ in 0..500 {
                    c.fetch_add(1, Ordering::Relaxed);
                }
            })
        })
        .collect();
    for h in hs {
        h.join().unwrap();
    }
    println!("atomic {}", counter.load(Ordering::SeqCst));
    let prev = counter.swap(7, Ordering::SeqCst);
    let cas = counter.compare_exchange(7, 9, Ordering::SeqCst, Ordering::SeqCst);
    let cas2 = counter.compare_exchange(7, 11, Ordering::SeqCst, Ordering::SeqCst);
    println!("{} {:?} {:?} {}", prev, cas, cas2, counter.load(Ordering::SeqCst));
    println!("fetch_max {} now {}", counter.fetch_max(20, Ordering::SeqCst), counter.load(Ordering::SeqCst));

    // poisoning
    std::panic::set_hook(Box::new(|_| {}));
    let m = Arc::new(Mutex::new(vec![1, 2, 3]));
    let m2 = Arc::clone(&m);
    let r = thread::spawn(move || {
        let mut g = m2.lock().unwrap();
        g.push(4);
        panic!("thread failed while holding lock");
    })
    .join();
    println!("join err {}", r.is_err());
    println!("poisoned {}", m.is_poisoned());
    match m.lock() {
        Ok(_) => println!("unexpected ok"),
        Err(p) => {
            let g = p.into_inner();
            println!("recovered {:?}", *g);
        }
    }
    m.clear_poison();
    println!("poisoned after clear {}", m.is_poisoned());

    let mut owned = Mutex::new(String::from("abc"));
    owned.get_mut().unwrap().push('d');
    println!("{}", owned.into_inner().unwrap());

    let named = thread::Builder::new().name("worker-7".into()).spawn(|| thread::current().name().map(|s| s.to_string())).unwrap();
    println!("{:?}", named.join().unwrap());
    let r = thread::spawn(|| -> Result<u8, String> { Err("nope".into()) }).join().unwrap();
    println!("{:?}", r);
    let try_lock = Mutex::new(1);
    let g = try_lock.lock().unwrap();
    println!("try_lock while held: {}", try_lock.try_lock().is_err());
    drop(g);
    println!("try_lock after: {:?}", try_lock.try_lock().map(|g| *g).ok());
}
