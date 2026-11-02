use std::thread;

fn parallel_sum(data: &[u64], parts: usize) -> u64 {
    let chunk = (data.len() + parts - 1) / parts;
    thread::scope(|s| {
        let handles: Vec<_> = data.chunks(chunk).map(|c| s.spawn(move || c.iter().sum::<u64>())).collect();
        handles.into_iter().map(|h| h.join().unwrap()).sum()
    })
}

fn main() {
    let data: Vec<u64> = (1..=1000).collect();
    println!("sum {}", parallel_sum(&data, 4));
    println!("sum {}", parallel_sum(&data, 7));

    let mut a = vec![1, 2, 3];
    let mut b = vec![10, 20];
    let label = String::from("scoped");
    thread::scope(|s| {
        s.spawn(|| {
            a.push(4);
            a.iter_mut().for_each(|x| *x *= 2);
        });
        s.spawn(|| {
            b.retain(|&x| x > 10);
            b.push(99);
        });
        s.spawn(|| {
            // shared immutable borrow across threads
            assert_eq!(label.len(), 6);
        });
    });
    println!("{} a={:?} b={:?}", label, a, b);

    let mut results = vec![0u32; 5];
    thread::scope(|s| {
        for (i, slot) in results.iter_mut().enumerate() {
            s.spawn(move || {
                *slot = (i as u32 + 1).pow(3);
            });
        }
    });
    println!("{:?}", results);

    let text = "the quick brown fox jumps over the lazy dog";
    let words: Vec<&str> = text.split(' ').collect();
    let lens: Vec<usize> = thread::scope(|s| {
        let hs: Vec<_> = words.iter().map(|w| s.spawn(move || w.len())).collect();
        hs.into_iter().map(|h| h.join().unwrap()).collect()
    });
    println!("{:?}", lens);

    let mut matrix = vec![vec![1i32; 4]; 3];
    thread::scope(|s| {
        for (r, row) in matrix.iter_mut().enumerate() {
            s.spawn(move || {
                for (c, v) in row.iter_mut().enumerate() {
                    *v = (r * 10 + c) as i32;
                }
            });
        }
    });
    for row in &matrix {
        println!("{:?}", row);
    }
    let outer = 5;
    let r = thread::scope(|s| {
        let h = s.spawn(|| {
            thread::scope(|s2| s2.spawn(|| outer * 2).join().unwrap())
        });
        h.join().unwrap() + 1
    });
    println!("nested scope {}", r);
}
