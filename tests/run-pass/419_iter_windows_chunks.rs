// Slice iterators: windows, chunks, chunks_exact (+remainder), rchunks, split, splitn,
// rsplit, split_inclusive, group-by via chunk_by, iter_mut over chunks_mut, and array_windows-like logic.
fn moving_avg(v: &[f64], k: usize) -> Vec<String> {
    v.windows(k).map(|w| format!("{:.2}", w.iter().sum::<f64>() / k as f64)).collect()
}

fn main() {
    let data = [1, 3, 6, 10, 15, 21, 28, 36, 45];
    let diffs: Vec<i32> = data.windows(2).map(|w| w[1] - w[0]).collect();
    println!("diffs {:?}", diffs);
    let second: Vec<i32> = diffs.windows(2).map(|w| w[1] - w[0]).collect();
    println!("second {:?}", second);
    println!("avg {:?}", moving_avg(&[1.0, 2.0, 4.0, 8.0, 16.0], 3));

    let increasing = data.windows(2).all(|w| w[0] < w[1]);
    let peaks = [1, 5, 2, 7, 3, 3, 9, 0].windows(3).filter(|w| w[1] > w[0] && w[1] > w[2]).count();
    println!("increasing {} peaks {}", increasing, peaks);

    for c in data.chunks(4) {
        print!("{:?} ", c);
    }
    println!();
    let ex = data.chunks_exact(4);
    let rem = ex.remainder().to_vec();
    let sums: Vec<i32> = ex.map(|c| c.iter().sum()).collect();
    println!("exact sums {:?} rem {:?}", sums, rem);
    let rc: Vec<Vec<i32>> = data.rchunks(4).map(|c| c.to_vec()).collect();
    println!("rchunks {:?}", rc);
    println!("chunks rev {:?}", data.chunks(3).rev().map(|c| c[0]).collect::<Vec<_>>());

    let mut grid = [0u8; 12];
    for (row, chunk) in grid.chunks_mut(4).enumerate() {
        for (col, cell) in chunk.iter_mut().enumerate() {
            *cell = (row * 10 + col) as u8;
        }
    }
    println!("grid {:?}", grid);
    for c in grid.chunks_exact_mut(3) {
        c.reverse();
    }
    println!("rev3 {:?}", grid);

    let nums = [1, 2, 0, 3, 4, 0, 0, 5];
    let parts: Vec<&[i32]> = nums.split(|x| *x == 0).collect();
    println!("split {:?}", parts);
    let parts2: Vec<&[i32]> = nums.splitn(2, |x| *x == 0).collect();
    println!("splitn {:?}", parts2);
    let parts3: Vec<&[i32]> = nums.rsplit(|x| *x == 0).collect();
    println!("rsplit {:?}", parts3);
    let parts4: Vec<&[i32]> = nums.split_inclusive(|x| *x == 0).collect();
    println!("split_inclusive {:?}", parts4);

    let runs = [1, 1, 2, 2, 2, 3, 1, 1];
    let groups: Vec<(i32, usize)> = runs.chunk_by(|a, b| a == b).map(|g| (g[0], g.len())).collect();
    println!("chunk_by {:?}", groups);
    let asc: Vec<&[i32]> = [1, 2, 3, 2, 5, 6, 1].chunk_by(|a, b| a < b).collect();
    println!("ascending runs {:?}", asc);

    let (l, r) = data.split_at(3);
    println!("split_at {:?} {:?}", l, r);
    if let Some((first, rest)) = data.split_first() {
        println!("first {} rest len {}", first, rest.len());
    }
    if let Some((last, _)) = data.split_last() {
        println!("last {}", last);
    }
    println!("starts {} ends {}", data.starts_with(&[1, 3]), data.ends_with(&[45]));
    println!("contains {} {}", data.contains(&21), data.contains(&22));

    let text = b"key1:val1,key2:val2,key3";
    for part in text.split(|b| *b == b',') {
        let mut kv = part.splitn(2, |b| *b == b':');
        let k = std::str::from_utf8(kv.next().unwrap()).unwrap();
        let v = kv.next().map(|v| std::str::from_utf8(v).unwrap());
        println!("{} => {:?}", k, v);
    }

    let mut v = vec![5, 4, 3, 2, 1, 0];
    v.rotate_left(2);
    println!("rotl {:?}", v);
    v.rotate_right(3);
    println!("rotr {:?}", v);
    let concat = [[1, 2], [3, 4], [5, 6]].concat();
    let joined = [vec![1], vec![2, 3]].join(&0);
    println!("{:?} {:?}", concat, joined);
    let words = ["a", "b", "c"].join("-");
    println!("{} {:?}", words, [[1u8; 2]; 2].iter().flatten().count());
}
