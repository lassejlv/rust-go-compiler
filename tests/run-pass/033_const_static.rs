const MAX_POINTS: u32 = 100_000;
const GREETING: &str = "hi there";
const PRIMES: [u32; 5] = [2, 3, 5, 7, 11];
const SECONDS_PER_DAY: u64 = 24 * 60 * 60;
static LANGUAGE: &str = "Rust";
static mut COUNTER: u32 = 0;
static TABLE: [(char, u32); 3] = [('a', 1), ('b', 2), ('c', 3)];

const fn square(x: u32) -> u32 {
    x * x
}

const SQ: u32 = square(12);

fn bump() -> u32 {
    unsafe {
        COUNTER += 1;
        COUNTER
    }
}

fn lookup(c: char) -> u32 {
    for (k, v) in TABLE.iter() {
        if *k == c {
            return *v;
        }
    }
    0
}

fn main() {
    println!("MAX_POINTS = {}", MAX_POINTS);
    println!("GREETING = {}", GREETING);
    println!("PRIMES = {:?}", PRIMES);
    let mut s = 0;
    for p in PRIMES {
        s += p;
    }
    println!("sum primes = {}", s);
    println!("week seconds = {}", SECONDS_PER_DAY * 7);
    println!("LANGUAGE = {}", LANGUAGE);
    println!("SQ = {}", SQ);
    const LOCAL: i32 = -7;
    println!("LOCAL = {}", LOCAL);
    for _ in 0..3 {
        bump();
    }
    println!("counter = {}", bump());
    println!("lookup b = {}, z = {}", lookup('b'), lookup('z'));
    let arr = [0u8; SQ as usize / 16];
    println!("arr len = {}", arr.len());
}
