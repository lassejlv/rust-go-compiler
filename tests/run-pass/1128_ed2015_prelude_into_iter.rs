// edition:2015
// Edition 2015/2018 prelude and method-resolution differences:
//  * `array.into_iter()` resolves to the slice iterator (yields &T)
//  * TryFrom/TryInto/FromIterator are not in the prelude, so a user trait
//    providing `try_into` is unambiguous
//  * `$p:pat` does not match a top-level or-pattern in macros
//  * trait objects with extra bounds written without `dyn`
use std::collections::HashMap;
use std::convert::TryFrom;
use std::fmt::Debug;

trait TryInto<T> {
    fn try_into(self) -> Option<T>;
}

impl TryInto<u8> for i32 {
    fn try_into(self) -> Option<u8> {
        if self >= 0 && self < 256 { Some(self as u8) } else { None }
    }
}

macro_rules! which {
    ($a:pat | $b:pat) => { "two alternatives" };
    ($a:pat) => { "one pattern" };
}

macro_rules! matches_any {
    ($e:expr, $($p:pat)|+) => {
        match $e { $($p)|+ => true, _ => false }
    };
}

fn describe(items: &[Box<Debug + Send>]) -> Vec<String> {
    items.iter().map(|i| format!("{:?}", i)).collect()
}

fn sum_refs<'a, I: Iterator<Item = &'a i32>>(it: I) -> i32 {
    it.fold(0, |a, b| a + *b)
}

fn main() {
    let arr = [10, 20, 30];
    let refs: Vec<&i32> = arr.into_iter().collect();
    println!("refs: {:?}", refs);
    println!("sum via refs: {}", sum_refs(arr.into_iter()));
    let doubled: Vec<i32> = arr.into_iter().map(|x| *x * 2).collect();
    println!("doubled: {:?}", doubled);
    let owned: Vec<i32> = arr.iter().cloned().collect();
    println!("owned: {:?}", owned);
    let nested = [[1, 2], [3, 4]];
    let flat: Vec<&i32> = nested.into_iter().flat_map(|row| row.into_iter()).collect();
    println!("flat: {:?}", flat);

    let small: Option<u8> = 200.try_into();
    let big: Option<u8> = 300.try_into();
    println!("user try_into: {:?} {:?}", small, big);
    println!("std TryFrom: {:?} {:?}", u8::try_from(255i32).ok(), u8::try_from(256i32).is_err());

    println!("{} / {}", which!(1 | 2), which!(Some(_)));
    for n in 0..6 {
        print!("{}{}", matches_any!(n, 1 | 3 | 5), if n < 5 { "," } else { "\n" });
    }

    let items: Vec<Box<Debug + Send>> = vec![Box::new(1u8), Box::new("two"), Box::new(Some(3.5)), Box::new(vec!['x'])];
    println!("{}", describe(&items).join(" | "));

    let mut counts: HashMap<&str, i32> = HashMap::new();
    for w in "a b a c b a".split(' ') {
        *counts.entry(w).or_insert(0) += 1;
    }
    let mut v: Vec<(&&str, &i32)> = counts.iter().collect();
    v.sort();
    println!("{:?}", v);
}
