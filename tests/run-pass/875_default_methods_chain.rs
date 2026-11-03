trait Counter {
    fn next_val(&mut self) -> Option<u32>;

    fn take_n(&mut self, n: usize) -> Vec<u32> {
        let mut v = Vec::new();
        for _ in 0..n {
            match self.next_val() {
                Some(x) => v.push(x),
                None => break,
            }
        }
        v
    }

    fn sum_all(&mut self) -> u32 {
        let mut s = 0;
        while let Some(x) = self.next_val() {
            s += x;
        }
        s
    }

    fn skip_then_sum(&mut self, k: usize) -> u32 {
        let _ = self.take_n(k);
        self.sum_all()
    }
}

struct Upto { cur: u32, max: u32 }
impl Counter for Upto {
    fn next_val(&mut self) -> Option<u32> {
        if self.cur < self.max {
            self.cur += 1;
            Some(self.cur)
        } else {
            None
        }
    }
}

struct Evens { cur: u32, limit: u32 }
impl Counter for Evens {
    fn next_val(&mut self) -> Option<u32> {
        if self.cur > self.limit { return None; }
        let r = self.cur;
        self.cur += 2;
        Some(r)
    }
    // override one default
    fn sum_all(&mut self) -> u32 {
        let mut s = 1000;
        while let Some(x) = self.next_val() { s += x; }
        s
    }
}

trait Animal {
    fn name(&self) -> String;
    fn sound(&self) -> String { "...".to_string() }
    fn speak(&self) -> String { format!("{} says {}", self.name(), self.sound()) }
    fn speak_twice(&self) -> String { format!("{}; {}", self.speak(), self.speak()) }
}
struct Dog;
struct Fish;
impl Animal for Dog {
    fn name(&self) -> String { "Dog".into() }
    fn sound(&self) -> String { "woof".into() }
}
impl Animal for Fish {
    fn name(&self) -> String { "Fish".into() }
}

fn chorus(xs: &[Box<dyn Animal>]) -> Vec<String> {
    xs.iter().map(|a| a.speak()).collect()
}

fn generic_counter<C: Counter>(mut c: C) -> (Vec<u32>, u32) {
    let first = c.take_n(2);
    let rest = c.sum_all();
    (first, rest)
}

fn main() {
    let mut u = Upto { cur: 0, max: 5 };
    println!("{:?}", u.take_n(3));
    println!("{}", u.sum_all());
    println!("{}", Upto { cur: 0, max: 10 }.skip_then_sum(4));
    let mut e = Evens { cur: 0, limit: 10 };
    println!("{:?}", e.take_n(2));
    println!("{}", e.sum_all());
    println!("{}", Evens { cur: 0, limit: 6 }.skip_then_sum(1));
    println!("{:?}", generic_counter(Upto { cur: 2, max: 6 }));
    println!("{:?}", generic_counter(Evens { cur: 1, limit: 9 }));
    let zoo: Vec<Box<dyn Animal>> = vec![Box::new(Dog), Box::new(Fish)];
    for s in chorus(&zoo) { println!("{}", s); }
    println!("{}", zoo[0].speak_twice());
    let mut dc: Box<dyn Counter> = Box::new(Upto { cur: 0, max: 3 });
    println!("{:?} {}", dc.take_n(1), dc.sum_all());
}
