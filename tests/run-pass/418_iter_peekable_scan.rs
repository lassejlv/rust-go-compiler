// Peekable (peek, peek_mut, next_if, next_if_eq), scan with state, take_while/skip_while,
// successors, from_fn, fuse, and a run-length encoder built from peekable.
use std::iter;

fn rle(s: &str) -> String {
    let mut out = String::new();
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        let mut n = 1;
        while it.next_if_eq(&c).is_some() {
            n += 1;
        }
        if n > 1 {
            out.push_str(&n.to_string());
        }
        out.push(c);
    }
    out
}

fn group_numbers(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut it = s.chars().peekable();
    while let Some(&c) = it.peek() {
        if c.is_ascii_digit() {
            let mut num = String::new();
            while let Some(d) = it.next_if(|ch| ch.is_ascii_digit()) {
                num.push(d);
            }
            out.push(format!("N{}", num));
        } else if c.is_alphabetic() {
            let mut w = String::new();
            while let Some(d) = it.next_if(|ch| ch.is_alphabetic()) {
                w.push(d);
            }
            out.push(format!("W{}", w));
        } else {
            it.next();
        }
    }
    out
}

struct Flaky {
    n: u32,
}
impl Iterator for Flaky {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        self.n += 1;
        if self.n % 3 == 0 {
            None
        } else {
            Some(self.n)
        }
    }
}

fn main() {
    println!("{}", rle("aaabccddddde"));
    println!("{}", rle(""));
    println!("{:?}", group_numbers("abc123 de4f 56,,g"));

    let mut p = vec![1, 2, 3].into_iter().peekable();
    if let Some(x) = p.peek_mut() {
        *x *= 100;
    }
    let snapshot: Vec<i32> = p.clone().collect();
    println!("{:?} {:?}", p.peek(), snapshot);
    p.next();
    println!("after next peek {:?}", p.peek());

    let running: Vec<i32> = [3, 1, 4, 1, 5, 9, 2, 6].iter().scan(0, |acc, &x| {
        *acc += x;
        Some(*acc)
    }).collect();
    println!("running {:?}", running);

    let until_big: Vec<i32> = [3, 1, 4, 1, 5, 9, 2, 6].iter().scan(1, |prod, &x| {
        *prod *= x;
        if *prod > 100 { None } else { Some(*prod) }
    }).collect();
    println!("until_big {:?}", until_big);

    let tw: Vec<&str> = "the quick brown fox".split(' ').take_while(|w| w.len() < 5 || w.starts_with('q')).collect();
    let sw: Vec<&str> = "the quick brown fox".split(' ').skip_while(|w| !w.starts_with('b')).collect();
    println!("{:?} {:?}", tw, sw);

    let powers: Vec<u64> = iter::successors(Some(1u64), |&x| x.checked_mul(10)).skip(15).collect();
    println!("powers {:?}", powers);

    let mut state = (0u32, 1u32);
    let fib: Vec<u32> = iter::from_fn(|| {
        let r = state.0;
        state = (state.1, state.0 + state.1);
        if r > 100 { None } else { Some(r) }
    }).collect();
    println!("fib {:?}", fib);

    let mut f = Flaky { n: 0 };
    let a: Vec<u32> = f.by_ref().collect();
    let b: Vec<u32> = f.by_ref().collect();
    println!("flaky {:?} {:?}", a, b);
    let mut fused = Flaky { n: 0 }.fuse();
    let c: Vec<u32> = fused.by_ref().collect();
    let d = fused.next();
    println!("fused {:?} {:?}", c, d);

    // peekable over tokens to parse "k=v;k2=v2"
    let input = "a=1;bb=22;;ccc=333;d";
    let mut chars = input.chars().peekable();
    let mut pairs = Vec::new();
    loop {
        let key: String = iter::from_fn(|| chars.next_if(|c| *c != '=' && *c != ';')).collect();
        if key.is_empty() && chars.peek().is_none() {
            break;
        }
        let val = if chars.next_if_eq(&'=').is_some() {
            Some(iter::from_fn(|| chars.next_if(|c| *c != ';')).collect::<String>())
        } else {
            None
        };
        chars.next_if_eq(&';');
        if !key.is_empty() {
            pairs.push((key, val));
        }
    }
    println!("{:?}", pairs);

    let dedup: Vec<i32> = {
        let mut it = [1, 1, 2, 3, 3, 3, 1, 4, 4].iter().peekable();
        let mut v = Vec::new();
        while let Some(&x) = it.next() {
            while it.peek() == Some(&&x) {
                it.next();
            }
            v.push(x);
        }
        v
    };
    println!("dedup {:?}", dedup);
}
