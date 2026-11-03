use std::ops::{Add, Mul};
use std::fmt::Debug;

fn adder<T: Add<Output = T> + Copy>(k: T) -> impl Fn(T) -> T { move |x| x + k }
fn scaler<T: Mul<Output = T> + Copy>(k: T) -> impl Fn(T) -> T { move |x| x * k }
fn repeat_n<T: Clone>(t: T, n: usize) -> impl Iterator<Item = T> { std::iter::repeat(t).take(n) }
fn window_sums<T>(v: Vec<T>, w: usize) -> impl Iterator<Item = T>
where
    T: Copy + Add<Output = T> + Default,
{
    let n = v.len();
    (0..=n.saturating_sub(w)).map(move |i| v[i..i + w].iter().fold(T::default(), |a, &b| a + b))
}
fn evens<I: Iterator<Item = u32>>(it: I) -> impl Iterator<Item = u32> { it.filter(|x| x % 2 == 0) }
fn running<T: Copy + Add<Output = T>>(start: T, steps: Vec<T>) -> impl Iterator<Item = T> {
    steps.into_iter().scan(start, |acc, s| { *acc = *acc + s; Some(*acc) })
}
fn pairwise<T: Copy + Debug>(v: Vec<T>) -> impl Iterator<Item = (T, T)> {
    let w = v.clone();
    v.into_iter().zip(w.into_iter().skip(1))
}

fn main() {
    let add5 = adder(5);
    println!("{}", add5(250u8));
    let half = scaler(0.5);
    println!("{}", half(3.0f32));
    let total: u64 = repeat_n(7, 3).sum();
    println!("{}", total);
    let shifted: u64 = repeat_n(1, 2).map(|x| x << 40).sum();
    println!("{}", shifted);
    let neg: Vec<i8> = repeat_n(-1, 3).collect();
    println!("{:?}", neg);
    let ws: Vec<u16> = window_sums(vec![1, 2, 3, 4, 5], 3).collect();
    println!("{:?}", ws);
    let fs: Vec<f32> = window_sums(vec![0.5, 0.25, 0.125], 2).collect();
    println!("{:?}", fs);
    let ev: Vec<u32> = evens(1..12).collect();
    println!("{:?}", ev);
    let run: Vec<i64> = running(1_000_000_000, vec![2_000_000_000, 3_000_000_000]).collect();
    println!("{:?}", run);
    let pw: Vec<(u8, u8)> = pairwise(vec![10, 20, 30]).collect();
    println!("{:?}", pw);
}
