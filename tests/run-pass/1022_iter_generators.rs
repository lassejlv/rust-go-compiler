use std::iter;

fn main() {
    let pow2: Vec<u64> = iter::successors(Some(1u64), |&x| x.checked_mul(2).filter(|&v| v < 1000)).collect();
    println!("{:?}", pow2);
    let collatz: Vec<u32> = iter::successors(Some(27u32), |&n| if n == 1 { None } else if n % 2 == 0 { Some(n / 2) } else { Some(3 * n + 1) }).collect();
    println!("collatz len {} max {}", collatz.len(), collatz.iter().max().unwrap());
    let mut state = (0u64, 1u64);
    let fib: Vec<u64> = iter::from_fn(|| {
        let r = state.0;
        state = (state.1, state.0 + state.1);
        if r > 200 { None } else { Some(r) }
    }).collect();
    println!("{:?}", fib);
    let mut seed = 12345u32;
    let rand: Vec<u32> = iter::repeat_with(|| {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        seed % 100
    }).take(8).collect();
    println!("{:?}", rand);
    println!("{:?}", iter::repeat("ab").take(3).collect::<String>());
    println!("{:?}", iter::once(0).chain(5..8).chain(iter::once(99)).collect::<Vec<_>>());
    println!("{:?}", iter::empty::<i32>().next());
    println!("{:?}", iter::repeat_n('x', 4).collect::<String>());
    let a = [1, 2, 3, 4];
    let b = ["one", "two", "three"];
    let c = [1.5, 2.5, 3.5, 4.5];
    for ((x, y), z) in a.iter().zip(b.iter()).zip(c.iter()) {
        println!("{} {} {}", x, y, z);
    }
    let triples: Vec<(i32, &str, f64)> = a.iter().zip(b).zip(c).map(|((&x, y), z)| (x, y, z)).collect();
    println!("{:?}", triples);
    let (evens, odds): (Vec<i32>, Vec<i32>) = (1..10).partition(|x| x % 2 == 0);
    println!("{:?} {:?}", evens, odds);
    let (xs, ys): (Vec<_>, Vec<_>) = triples.iter().map(|t| (t.0, t.2)).unzip();
    println!("{:?} {:?}", xs, ys);
    let mut it = (1..=10).peekable();
    let mut groups = vec![];
    while let Some(x) = it.next() {
        let mut g = vec![x];
        while let Some(&n) = it.peek() {
            if n / 4 == x / 4 { g.push(n); it.next(); } else { break; }
        }
        groups.push(g);
    }
    println!("{:?}", groups);
    let s: Vec<i32> = (1..=5).scan(0, |acc, x| { *acc += x; Some(*acc) }).collect();
    println!("{:?}", s);
    println!("{:?}", (1..20).step_by(4).rev().collect::<Vec<_>>());
    println!("{:?}", (1..=4).flat_map(|x| iter::repeat(x).take(x as usize)).collect::<Vec<_>>());
    println!("{:?}", [[1, 2], [3, 4]].iter().flatten().rev().collect::<Vec<_>>());
    println!("{:?}", (1..=6).map_while(|x| if x < 4 { Some(x * 10) } else { None }).collect::<Vec<_>>());
    println!("{:?}", (1..100).skip_while(|x| x * x < 50).take_while(|x| x * x < 150).collect::<Vec<_>>());
    println!("{:?}", "a-b-c".split('-').intersperse_fallback());
    println!("{:?}", (1..=4).reduce(|a, b| a * b));
    println!("{:?}", (1..=4).try_fold(0i8, |a, b| a.checked_add(b * 40)));
    println!("{:?}", ["1", "2", "x"].iter().map(|s| s.parse::<i32>()).collect::<Result<Vec<_>, _>>().is_err());
    println!("{:?}", [Some(1), Some(2)].into_iter().collect::<Option<Vec<_>>>());
    println!("{:?}", (0..5).cycle().skip(3).take(7).collect::<Vec<_>>());
    println!("{:?}", (1..=3).map(|x| x as f64).sum::<f64>());
    println!("{:?}", (1..=5).product::<u64>());
    println!("{:?}", "hello".chars().rev().enumerate().last());
    println!("{:?}", (0..10).position(|x| x * x > 20));
    println!("{:?}", [3, 1, 4, 1, 5].iter().rposition(|&x| x == 1));
    println!("{}", (1..=3).eq([1, 2, 3].iter().copied()));
    println!("{:?}", (1..4).cmp(1..5));
    println!("{:?}", [1, 5, 2].iter().max_by_key(|&&x| x));
    println!("{:?}", (0..3).inspect(|x| print!("[{}]", x)).count());
    let mut arr_it = [10, 20, 30].into_iter();
    println!("{:?} {:?}", arr_it.next_back(), arr_it.len());
    println!("{:?}", (1..=9).array_chunks_fallback());
}

trait InterFallback { fn intersperse_fallback(self) -> String; }
impl<'a, I: Iterator<Item = &'a str>> InterFallback for I {
    fn intersperse_fallback(self) -> String { self.collect::<Vec<_>>().join("+") }
}
trait ArrChunks { fn array_chunks_fallback(self) -> Vec<[i32; 3]>; }
impl<I: Iterator<Item = i32>> ArrChunks for I {
    fn array_chunks_fallback(self) -> Vec<[i32; 3]> {
        let v: Vec<i32> = self.collect();
        v.chunks_exact(3).map(|c| [c[0], c[1], c[2]]).collect()
    }
}
