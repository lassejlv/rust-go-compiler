struct Counter {
    count: u32,
    limit: u32,
}

impl Counter {
    fn new(limit: u32) -> Counter {
        Counter { count: 0, limit }
    }
}

impl Iterator for Counter {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.count < self.limit {
            self.count += 1;
            Some(self.count)
        } else {
            None
        }
    }
}

struct Fib {
    a: u64,
    b: u64,
}

impl Iterator for Fib {
    type Item = u64;
    fn next(&mut self) -> Option<u64> {
        let r = self.a;
        self.a = self.b;
        self.b += r;
        Some(r)
    }
}

struct Countdown(i32);

impl Iterator for Countdown {
    type Item = i32;
    fn next(&mut self) -> Option<i32> {
        if self.0 <= 0 {
            None
        } else {
            self.0 -= 1;
            Some(self.0 + 1)
        }
    }
}

struct Grid {
    w: usize,
    h: usize,
}

struct GridIter<'a> {
    grid: &'a Grid,
    pos: usize,
}

impl Grid {
    fn cells(&self) -> GridIter<'_> {
        GridIter { grid: self, pos: 0 }
    }
}

impl<'a> Iterator for GridIter<'a> {
    type Item = (usize, usize);
    fn next(&mut self) -> Option<(usize, usize)> {
        if self.pos >= self.grid.w * self.grid.h {
            return None;
        }
        let p = self.pos;
        self.pos += 1;
        Some((p % self.grid.w, p / self.grid.w))
    }
}

fn main() {
    let c = Counter::new(5);
    let v: Vec<u32> = c.collect();
    println!("counter: {:?}", v);
    let mut c2 = Counter::new(3);
    println!("manual: {:?} {:?} {:?} {:?}", c2.next(), c2.next(), c2.next(), c2.next());
    let s: u32 = Counter::new(5).zip(Counter::new(5).skip(1)).map(|(a, b)| a * b).filter(|x| x % 3 == 0).sum();
    println!("zip/skip/map/filter sum: {}", s);
    let fibs: Vec<u64> = Fib { a: 0, b: 1 }.take(15).collect();
    println!("fib: {:?}", fibs);
    let big = Fib { a: 0, b: 1 }.find(|&x| x > 1_000_000).unwrap();
    println!("first fib > 1e6: {}", big);
    let even_fib_sum: u64 = Fib { a: 1, b: 2 }.take_while(|&x| x < 4_000_000).filter(|x| x % 2 == 0).sum();
    println!("even fib sum: {}", even_fib_sum);
    for n in Countdown(3) {
        print!("{}... ", n);
    }
    println!("liftoff!");
    let g = Grid { w: 3, h: 2 };
    let cells: Vec<(usize, usize)> = g.cells().collect();
    println!("cells: {:?}", cells);
    println!("count: {}", g.cells().count());
    println!("last: {:?}", Counter::new(4).last());
    println!("max via fold: {}", Counter::new(10).fold(0, |m, x| if x * 7 % 10 > m { x * 7 % 10 } else { m }));
    let mut it = Counter::new(10);
    let firsts: Vec<u32> = it.by_ref().take(3).collect();
    let rest: Vec<u32> = it.collect();
    println!("by_ref: {:?} then {:?}", firsts, rest);
}
