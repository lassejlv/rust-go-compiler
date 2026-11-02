use std::sync::{Arc, Barrier, Mutex, RwLock};
use std::thread;

fn main() {
    // Barrier: each thread records a phase-1 value, then after the barrier reads all of them.
    let n = 4;
    let barrier = Arc::new(Barrier::new(n));
    let slots = Arc::new(Mutex::new(vec![0u32; n]));
    let hs: Vec<_> = (0..n)
        .map(|i| {
            let b = Arc::clone(&barrier);
            let s = Arc::clone(&slots);
            thread::spawn(move || {
                s.lock().unwrap()[i] = (i as u32 + 1) * 10;
                let leader = b.wait().is_leader();
                let total: u32 = s.lock().unwrap().iter().sum();
                (total, leader)
            })
        })
        .collect();
    let mut leaders = 0;
    for h in hs {
        let (total, leader) = h.join().unwrap();
        println!("after barrier total {}", total);
        if leader {
            leaders += 1;
        }
    }
    println!("leaders {}", leaders);

    // Dining philosophers with ordered lock acquisition (no deadlock).
    let forks: Arc<Vec<Mutex<u32>>> = Arc::new((0..5).map(|_| Mutex::new(0)).collect());
    let meals = Arc::new(Mutex::new(vec![0u32; 5]));
    let hs: Vec<_> = (0..5)
        .map(|p| {
            let forks = Arc::clone(&forks);
            let meals = Arc::clone(&meals);
            thread::spawn(move || {
                let (a, b) = (p, (p + 1) % 5);
                let (first, second) = if a < b { (a, b) } else { (b, a) };
                for _ in 0..20 {
                    let mut f1 = forks[first].lock().unwrap();
                    let mut f2 = forks[second].lock().unwrap();
                    *f1 += 1;
                    *f2 += 1;
                    meals.lock().unwrap()[p] += 1;
                }
            })
        })
        .collect();
    for h in hs {
        h.join().unwrap();
    }
    println!("meals {:?}", meals.lock().unwrap());
    let uses: Vec<u32> = forks.iter().map(|f| *f.lock().unwrap()).collect();
    println!("fork uses {:?}", uses);

    // RwLock-protected cache with readers and a writer, final state is deterministic.
    let cache = Arc::new(RwLock::new(Vec::<u64>::new()));
    let writer = {
        let c = Arc::clone(&cache);
        thread::spawn(move || {
            for i in 0..10 {
                c.write().unwrap().push(i * i);
            }
        })
    };
    writer.join().unwrap();
    let readers: Vec<_> = (0..3)
        .map(|k| {
            let c = Arc::clone(&cache);
            thread::spawn(move || c.read().unwrap().iter().skip(k).step_by(3).sum::<u64>())
        })
        .collect();
    let sums: Vec<u64> = readers.into_iter().map(|h| h.join().unwrap()).collect();
    println!("reader sums {:?}", sums);
    println!("available_parallelism >= 1: {}", thread::available_parallelism().map(|n| n.get() >= 1).unwrap_or(true));
}
