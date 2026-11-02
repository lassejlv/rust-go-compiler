use std::fmt::Debug;
use std::fmt::Display;

fn largest<T: PartialOrd + Copy>(list: &[T]) -> T {
    let mut largest = list[0];
    for &item in list {
        if item > largest {
            largest = item;
        }
    }
    largest
}

fn largest_ref<T: PartialOrd>(list: &[T]) -> &T {
    let mut largest = &list[0];
    for item in list {
        if item > largest {
            largest = item;
        }
    }
    largest
}

fn show_all<T>(items: &[T]) -> String
where
    T: Display,
{
    let parts: Vec<String> = items.iter().map(|i| format!("<{}>", i)).collect();
    parts.join("")
}

fn debug_pair<A: Debug, B: Debug>(a: A, b: B) -> String {
    format!("({:?} & {:?})", a, b)
}

fn swap<T, U>(pair: (T, U)) -> (U, T) {
    (pair.1, pair.0)
}

fn count_matching<T: PartialEq>(items: &[T], target: &T) -> usize {
    let mut n = 0;
    for it in items {
        if it == target {
            n += 1;
        }
    }
    n
}

fn identity<T>(x: T) -> T {
    x
}

fn sum_all<T>(items: &[T]) -> T
where
    T: Copy + std::ops::Add<Output = T> + Default,
{
    let mut acc = T::default();
    for &i in items {
        acc = acc + i;
    }
    acc
}

fn main() {
    println!("largest i32 = {}", largest(&[34, 50, 25, 100, 65]));
    println!("largest f64 = {}", largest(&[1.5, -2.0, 0.25]));
    println!("largest char = {}", largest(&['y', 'm', 'a', 'q']));
    let words = vec![String::from("pear"), String::from("zucchini"), String::from("apple")];
    println!("largest String = {}", largest_ref(&words));
    println!("show_all = {}", show_all(&[1, 2, 3]));
    println!("show_all = {}", show_all(&["a", "b"]));
    println!("debug_pair = {}", debug_pair(1, "x"));
    println!("debug_pair = {}", debug_pair(vec![1], Some(2.5)));
    println!("swap = {:?}", swap((1, "one")));
    println!("count = {}", count_matching(&[1, 2, 1, 3, 1], &1));
    println!("count = {}", count_matching(&["a", "b"], &"c"));
    println!("identity = {} {}", identity(5), identity("str"));
    println!("sum i32 = {}", sum_all(&[1, 2, 3, 4]));
    println!("sum f64 = {}", sum_all(&[0.5, 0.25]));
    let explicit = largest::<u8>(&[3, 9, 2]);
    println!("turbofish = {}", explicit);
}
