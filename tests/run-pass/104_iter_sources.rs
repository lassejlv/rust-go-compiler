use std::iter;

fn main() {
    let rep: Vec<&str> = iter::repeat("ab").take(3).collect();
    println!("repeat: {:?}", rep);
    let once: Vec<i32> = iter::once(5).chain(iter::once(6)).collect();
    println!("once: {:?}", once);
    let powers: Vec<u64> = iter::successors(Some(1u64), |&x| if x < 1000 { Some(x * 3) } else { None }).collect();
    println!("successors: {:?}", powers);
    let collatz: Vec<u32> = iter::successors(Some(6u32), |&n| if n == 1 { None } else if n % 2 == 0 { Some(n / 2) } else { Some(3 * n + 1) }).collect();
    println!("collatz 6: {:?}", collatz);
    let mut state = (0u32, 1u32);
    let fib: Vec<u32> = iter::from_fn(|| {
        let r = state.0;
        state = (state.1, state.0 + state.1);
        if r > 100 { None } else { Some(r) }
    })
    .collect();
    println!("from_fn fib: {:?}", fib);
    let mut c = 0;
    let counter = iter::from_fn(move || {
        c += 1;
        if c <= 3 { Some(c * c) } else { None }
    });
    println!("from_fn counter: {:?}", counter.collect::<Vec<_>>());
    let rw: Vec<i32> = iter::repeat_with(|| 7).take(2).collect();
    println!("repeat_with: {:?}", rw);
    let empty: Vec<i32> = iter::empty().collect();
    println!("empty: {:?}", empty);
    let zipped: Vec<(char, i32)> = iter::zip("xyz".chars(), 1..).collect();
    println!("iter::zip: {:?}", zipped);
    let cyc: String = "ab".chars().cycle().take(5).collect();
    println!("cycle: {}", cyc);
    let arr = [1, 2, 3].map(|x| x * 100);
    println!("array map: {:?}", arr);
    let sum_sq: i32 = (1..=10).map(|x| x * x).sum();
    let sq_sum: i32 = (1..=10).sum::<i32>().pow(2);
    println!("sum of squares {} square of sum {} diff {}", sum_sq, sq_sum, sq_sum - sum_sq);
    let primes: Vec<u32> = (2..).filter(|n| (2..*n).take_while(|d| d * d <= *n).all(|d| n % d != 0)).take(10).collect();
    println!("first 10 primes: {:?}", primes);
    let triangles: Vec<u32> = (1..).scan(0, |acc, n| { *acc += n; Some(*acc) }).take_while(|&t| t < 60).collect();
    println!("triangle numbers < 60: {:?}", triangles);
    let char_range: String = ('a'..='e').chain('X'..='Z').collect();
    println!("char ranges: {}", char_range);
    let total: f64 = (1..=4).map(|x| 1.0 / x as f64).sum();
    println!("harmonic 4: {:.4}", total);
    let pyth: Vec<(u32, u32, u32)> = (1..20)
        .flat_map(|a| (a..20).flat_map(move |b| (b..30).map(move |c| (a, b, c))))
        .filter(|&(a, b, c)| a * a + b * b == c * c)
        .collect();
    println!("pythagorean triples: {:?}", pyth);
}
