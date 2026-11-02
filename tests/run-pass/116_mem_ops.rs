use std::mem;

#[derive(Debug, Default)]
struct Buffer {
    data: Vec<u8>,
    label: String,
}

#[derive(Debug)]
enum State {
    Empty,
    Loaded(Vec<i32>),
}

impl Buffer {
    fn flush(&mut self) -> Vec<u8> {
        mem::take(&mut self.data)
    }
    fn relabel(&mut self, new: &str) -> String {
        mem::replace(&mut self.label, new.to_string())
    }
}

fn main() {
    let mut a = 1;
    let mut b = 2;
    mem::swap(&mut a, &mut b);
    println!("swap ints: a={} b={}", a, b);
    let mut s1 = String::from("left");
    let mut s2 = String::from("right");
    mem::swap(&mut s1, &mut s2);
    println!("swap strings: {} {}", s1, s2);
    let mut v = vec![1, 2, 3, 4];
    v.swap(0, 3);
    println!("vec swap: {:?}", v);
    let (x, y) = v.split_at_mut(2);
    mem::swap(&mut x[0], &mut y[0]);
    println!("swap across split: {:?}", v);
    let mut buf = Buffer { data: vec![1, 2, 3], label: "first".to_string() };
    let flushed = buf.flush();
    println!("flushed {:?}, remaining {:?}", flushed, buf.data);
    let old = buf.relabel("second");
    println!("old label {}, new label {}", old, buf.label);
    let taken = mem::take(&mut buf);
    println!("taken {:?}, left {:?}", taken, buf);
    let mut st = State::Loaded(vec![7, 8]);
    if let State::Loaded(items) = mem::replace(&mut st, State::Empty) {
        println!("moved out items {:?}", items);
    }
    println!("state now {:?}", st);
    let mut opt = Some(5);
    let t = opt.take();
    println!("option take: {:?} {:?}", t, opt);
    let mut name = String::from("temp");
    let moved = mem::take(&mut name);
    println!("string take: '{}' '{}'", moved, name);
    let mut nums = vec![3, 1, 2];
    let sorted = {
        let mut tmp = mem::take(&mut nums);
        tmp.sort();
        tmp
    };
    println!("sorted {:?}, original now {:?}", sorted, nums);
    println!("size_of: u8={} u32={} u64={} f64={} char={} bool={}", mem::size_of::<u8>(), mem::size_of::<u32>(), mem::size_of::<u64>(), mem::size_of::<f64>(), mem::size_of::<char>(), mem::size_of::<bool>());
    let mut grid = vec![vec![0; 3]; 2];
    let row = mem::replace(&mut grid[0], vec![9, 9, 9]);
    println!("replaced row {:?}, grid {:?}", row, grid);
    let mut fib = (0u64, 1u64);
    for _ in 0..10 {
        let next = fib.0 + fib.1;
        fib.0 = mem::replace(&mut fib.1, next);
    }
    println!("fib via replace: {:?}", fib);
}
