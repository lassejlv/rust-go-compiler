use std::thread;
use std::sync::atomic::{AtomicUsize, Ordering};

fn parallel_sum(v: &[u64], parts: usize) -> u64 {
    let chunk = (v.len() + parts - 1) / parts;
    thread::scope(|s| {
        let hs: Vec<_> = v.chunks(chunk).map(|c| s.spawn(move || c.iter().sum::<u64>())).collect();
        hs.into_iter().map(|h| h.join().unwrap()).sum()
    })
}

fn main() {
    let v: Vec<u64> = (1..=1000).collect();
    println!("sum {}", parallel_sum(&v, 4));
    let mut a = vec![1, 2, 3];
    let mut b = vec![10, 20];
    let counter = AtomicUsize::new(0);
    thread::scope(|s| {
        s.spawn(|| {
            a.push(4);
            counter.fetch_add(1, Ordering::SeqCst);
        });
        s.spawn(|| {
            b.iter_mut().for_each(|x| *x *= 3);
            counter.fetch_add(1, Ordering::SeqCst);
        });
    });
    println!("{:?} {:?} {}", a, b, counter.load(Ordering::SeqCst));
    let mut grid = vec![vec![0u32; 4]; 4];
    thread::scope(|s| {
        for (r, row) in grid.iter_mut().enumerate() {
            s.spawn(move || {
                for (c, cell) in row.iter_mut().enumerate() {
                    *cell = (r * 4 + c) as u32;
                }
            });
        }
    });
    println!("{:?}", grid);
    let text = String::from("the quick brown fox jumps over the lazy dog");
    let words: Vec<&str> = text.split(' ').collect();
    let lens = thread::scope(|s| {
        let h1 = s.spawn(|| words.iter().map(|w| w.len()).max().unwrap());
        let h2 = s.spawn(|| words.iter().filter(|w| w.contains('o')).count());
        (h1.join().unwrap(), h2.join().unwrap())
    });
    println!("{:?}", lens);
    let r = thread::scope(|s| {
        let h = s.spawn(|| -> Result<i32, String> { Err("bad".into()) });
        h.join().unwrap()
    });
    println!("{:?}", r);
}
