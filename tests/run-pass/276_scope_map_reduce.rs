// std::thread::scope parallel map-reduce over borrowed data: chunked sums, parallel word count
// merged into a BTreeMap, parallel sort + k-way merge, and scoped threads returning values.
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::thread;

fn par_map_reduce<T, R, M, F>(data: &[T], chunks: usize, map: M, reduce: F, init: R) -> R
where
    T: Sync,
    R: Send + Clone,
    M: Fn(&[T]) -> R + Sync,
    F: Fn(R, R) -> R,
{
    let size = (data.len() + chunks - 1) / chunks.max(1);
    let partials: Vec<R> = thread::scope(|s| {
        let handles: Vec<_> = data.chunks(size.max(1)).map(|c| s.spawn(|| map(c))).collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });
    partials.into_iter().fold(init, reduce)
}

fn kway_merge(runs: Vec<Vec<i64>>) -> Vec<i64> {
    let mut idx = vec![0usize; runs.len()];
    let mut out = Vec::with_capacity(runs.iter().map(|r| r.len()).sum());
    loop {
        let mut best: Option<(usize, i64)> = None;
        for (i, r) in runs.iter().enumerate() {
            if let Some(&v) = r.get(idx[i]) {
                if best.map_or(true, |(_, b)| v < b) {
                    best = Some((i, v));
                }
            }
        }
        match best {
            Some((i, v)) => {
                out.push(v);
                idx[i] += 1;
            }
            None => return out,
        }
    }
}

fn main() {
    let nums: Vec<u64> = (1..=100_000).collect();
    let total = par_map_reduce(&nums, 8, |c| c.iter().sum::<u64>(), |a, b| a + b, 0);
    println!("sum = {}", total);
    let sq = par_map_reduce(&nums, 3, |c| c.iter().map(|x| x * x % 1000).sum::<u64>(), |a, b| a + b, 0);
    println!("sum sq mod = {}", sq);
    let max_digit_sum = par_map_reduce(
        &nums,
        5,
        |c| c.iter().map(|n| n.to_string().bytes().map(|b| (b - b'0') as u32).sum::<u32>()).max().unwrap_or(0),
        |a, b| a.max(b),
        0,
    );
    println!("max digit sum = {}", max_digit_sum);

    let text = "the quick brown fox jumps over the lazy dog the fox barks \
                and the dog sleeps while the quick cat watches the brown fox";
    let words: Vec<&str> = text.split_whitespace().collect();
    let counts = par_map_reduce(
        &words,
        4,
        |c| {
            let mut m = BTreeMap::new();
            for w in c {
                *m.entry(w.to_string()).or_insert(0usize) += 1;
            }
            m
        },
        |mut a, b| {
            for (k, v) in b {
                *a.entry(k).or_insert(0) += v;
            }
            a
        },
        BTreeMap::new(),
    );
    let mut top: Vec<(&String, &usize)> = counts.iter().collect();
    top.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    println!("top words: {:?}", &top[..4]);
    println!("distinct={} total={}", counts.len(), counts.values().sum::<usize>());

    // parallel sort of chunks then merge
    let mut seed = 12345u64;
    let data: Vec<i64> = (0..1000)
        .map(|_| {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            ((seed >> 33) % 10000) as i64 - 5000
        })
        .collect();
    let runs: Vec<Vec<i64>> = thread::scope(|s| {
        data.chunks(250)
            .map(|c| {
                s.spawn(move || {
                    let mut v = c.to_vec();
                    v.sort_unstable();
                    v
                })
            })
            .collect::<Vec<_>>()
            .into_iter()
            .map(|h| h.join().unwrap())
            .collect()
    });
    let merged = kway_merge(runs);
    let mut expected = data.clone();
    expected.sort();
    println!("merged ok={} min={} max={} median={}", merged == expected, merged[0], merged[999], merged[500]);

    // scoped threads mutating disjoint slices in place
    let mut grid = vec![0u32; 64];
    thread::scope(|s| {
        for (row, chunk) in grid.chunks_mut(8).enumerate() {
            s.spawn(move || {
                for (col, cell) in chunk.iter_mut().enumerate() {
                    *cell = (row * col) as u32;
                }
            });
        }
    });
    println!("grid diag {:?}", (0..8).map(|i| grid[i * 8 + i]).collect::<Vec<_>>());

    // shared counters via atomics and a mutex-protected log, sorted afterwards
    let hits = AtomicUsize::new(0);
    let log = Mutex::new(Vec::new());
    thread::scope(|s| {
        for t in 0..6 {
            let hits = &hits;
            let log = &log;
            s.spawn(move || {
                let mine = (0..1000).filter(|x| x % (t + 2) == 0).count();
                hits.fetch_add(mine, Ordering::Relaxed);
                log.lock().unwrap().push(format!("t{}:{}", t, mine));
            });
        }
    });
    let mut log = log.into_inner().unwrap();
    log.sort();
    println!("hits={} log={:?}", hits.load(Ordering::Relaxed), log);
}
