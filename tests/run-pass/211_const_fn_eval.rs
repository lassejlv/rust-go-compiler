// const fn evaluation used in array lengths, consts, statics and associated consts.

const fn fib(n: u32) -> u64 {
    let mut a = 0u64;
    let mut b = 1u64;
    let mut i = 0;
    while i < n {
        let t = a + b;
        a = b;
        b = t;
        i += 1;
    }
    a
}

const fn next_pow2(mut n: usize) -> usize {
    if n == 0 {
        return 1;
    }
    n -= 1;
    n |= n >> 1;
    n |= n >> 2;
    n |= n >> 4;
    n |= n >> 8;
    n |= n >> 16;
    n + 1
}

const fn crc8_table() -> [u8; 256] {
    let mut table = [0u8; 256];
    let mut i = 0;
    while i < 256 {
        let mut crc = i as u8;
        let mut j = 0;
        while j < 8 {
            crc = if crc & 0x80 != 0 { (crc << 1) ^ 0x07 } else { crc << 1 };
            j += 1;
        }
        table[i] = crc;
        i += 1;
    }
    table
}

static CRC8: [u8; 256] = crc8_table();

fn crc8(data: &[u8]) -> u8 {
    let mut c = 0u8;
    for &b in data {
        c = CRC8[(c ^ b) as usize];
    }
    c
}

const fn str_len_sum(parts: &[&str]) -> usize {
    let mut i = 0;
    let mut n = 0;
    while i < parts.len() {
        n += parts[i].len();
        i += 1;
    }
    n
}

const NAMES: [&str; 3] = ["alpha", "beta", "gamma"];
const NAMES_LEN: usize = str_len_sum(&NAMES);
const FIB20: u64 = fib(20);
const CAP: usize = next_pow2(100);

struct Ring {
    buf: [u32; CAP],
    head: usize,
}

impl Ring {
    const MASK: usize = CAP - 1;
    const fn new() -> Self {
        Ring { buf: [0; CAP], head: 0 }
    }
    fn push(&mut self, v: u32) {
        self.buf[self.head & Self::MASK] = v;
        self.head += 1;
    }
}

struct Matrix<const R: usize, const C: usize>;
impl<const R: usize, const C: usize> Matrix<R, C> {
    const CELLS: usize = R * C;
    const IS_SQUARE: bool = R == C;
}

const fn gcd(a: u64, b: u64) -> u64 {
    if b == 0 {
        a
    } else {
        gcd(b, a % b)
    }
}

const fn is_prime(n: u32) -> bool {
    if n < 2 {
        return false;
    }
    let mut d = 2;
    while d * d <= n {
        if n % d == 0 {
            return false;
        }
        d += 1;
    }
    true
}

const fn primes<const N: usize>() -> [u32; N] {
    let mut out = [0u32; N];
    let mut count = 0;
    let mut n = 2;
    while count < N {
        if is_prime(n) {
            out[count] = n;
            count += 1;
        }
        n += 1;
    }
    out
}

const PRIMES: [u32; 12] = primes::<12>();
const GCD: u64 = gcd(1071, 462);
const FLAGS: u32 = (1 << 3) | (1 << 5) | u8::MAX as u32;
const SHIFTED: [u8; fib(6) as usize] = [7; fib(6) as usize];

fn main() {
    println!("fib20={} cap={} names_len={}", FIB20, CAP, NAMES_LEN);
    let arr = [0u8; NAMES_LEN];
    println!("arr len {}", arr.len());
    println!("crc8(123456789) = {:#04x}", crc8(b"123456789"));
    println!("crc8(hello) = {:#04x}", crc8(b"hello"));
    let mut r = Ring::new();
    for i in 0..130 {
        r.push(i);
    }
    println!("mask={} head={} buf[0]={} buf[1]={} buf[2]={}", Ring::MASK, r.head, r.buf[0], r.buf[1], r.buf[2]);
    println!("{} {}", Matrix::<3, 4>::CELLS, Matrix::<3, 4>::IS_SQUARE);
    println!("{} {}", Matrix::<5, 5>::CELLS, Matrix::<5, 5>::IS_SQUARE);
    println!("primes {:?}", PRIMES);
    println!("gcd={} flags={:#b} shifted={:?}", GCD, FLAGS, SHIFTED);
    const LOCAL: u64 = fib(10) * 2 + gcd(12, 18);
    println!("local={}", LOCAL);
    let runtime: Vec<u64> = (0..10).map(fib).collect();
    println!("{:?}", runtime);
    println!("{:?}", [next_pow2(0), next_pow2(1), next_pow2(17), next_pow2(64), next_pow2(65)]);
}
