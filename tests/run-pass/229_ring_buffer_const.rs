// A fixed-capacity ring buffer generic over T and const N, with iterators and Drop handling.
use std::fmt;
use std::mem::MaybeUninit;

struct Ring<T, const N: usize> {
    buf: [MaybeUninit<T>; N],
    head: usize,
    len: usize,
}

impl<T, const N: usize> Ring<T, N> {
    const CAP: usize = N;

    fn new() -> Self {
        Ring { buf: [const { MaybeUninit::uninit() }; N], head: 0, len: 0 }
    }
    fn is_full(&self) -> bool {
        self.len == N
    }
    fn phys(&self, i: usize) -> usize {
        (self.head + i) % N
    }
    /// Pushes to the back; when full, overwrites and returns the oldest element.
    fn push(&mut self, v: T) -> Option<T> {
        if self.is_full() {
            let old = unsafe { self.buf[self.head].assume_init_read() };
            self.buf[self.head].write(v);
            self.head = (self.head + 1) % N;
            Some(old)
        } else {
            let idx = self.phys(self.len);
            self.buf[idx].write(v);
            self.len += 1;
            None
        }
    }
    fn pop_front(&mut self) -> Option<T> {
        if self.len == 0 {
            return None;
        }
        let v = unsafe { self.buf[self.head].assume_init_read() };
        self.head = (self.head + 1) % N;
        self.len -= 1;
        Some(v)
    }
    fn pop_back(&mut self) -> Option<T> {
        if self.len == 0 {
            return None;
        }
        self.len -= 1;
        let idx = self.phys(self.len);
        Some(unsafe { self.buf[idx].assume_init_read() })
    }
    fn get(&self, i: usize) -> Option<&T> {
        if i < self.len {
            Some(unsafe { self.buf[self.phys(i)].assume_init_ref() })
        } else {
            None
        }
    }
    fn iter(&self) -> RingIter<'_, T, N> {
        RingIter { ring: self, front: 0, back: self.len }
    }
}

struct RingIter<'a, T, const N: usize> {
    ring: &'a Ring<T, N>,
    front: usize,
    back: usize,
}

impl<'a, T, const N: usize> Iterator for RingIter<'a, T, N> {
    type Item = &'a T;
    fn next(&mut self) -> Option<&'a T> {
        if self.front >= self.back {
            return None;
        }
        let r = self.ring.get(self.front);
        self.front += 1;
        r
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.back - self.front;
        (n, Some(n))
    }
}

impl<'a, T, const N: usize> DoubleEndedIterator for RingIter<'a, T, N> {
    fn next_back(&mut self) -> Option<&'a T> {
        if self.front >= self.back {
            return None;
        }
        self.back -= 1;
        self.ring.get(self.back)
    }
}

impl<'a, T, const N: usize> ExactSizeIterator for RingIter<'a, T, N> {}

impl<T, const N: usize> Drop for Ring<T, N> {
    fn drop(&mut self) {
        while self.pop_front().is_some() {}
    }
}

impl<T: fmt::Debug, const N: usize> fmt::Debug for Ring<T, N> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Ring<{}>", N)?;
        f.debug_list().entries(self.iter()).finish()
    }
}

struct Loud(u32);
impl Drop for Loud {
    fn drop(&mut self) {
        println!("  drop Loud({})", self.0);
    }
}

// Moving average over a sliding window using the ring buffer
struct MovingAvg<const W: usize> {
    ring: Ring<f64, W>,
    sum: f64,
}

impl<const W: usize> MovingAvg<W> {
    fn new() -> Self {
        MovingAvg { ring: Ring::new(), sum: 0.0 }
    }
    fn add(&mut self, x: f64) -> f64 {
        if let Some(old) = self.ring.push(x) {
            self.sum -= old;
        }
        self.sum += x;
        self.sum / self.ring.len as f64
    }
}

fn main() {
    let mut r: Ring<i32, 4> = Ring::new();
    println!("cap {}", Ring::<i32, 4>::CAP);
    for i in 1..=6 {
        let ev = r.push(i * 10);
        println!("push {} -> evicted {:?}, {:?}", i * 10, ev, r);
    }
    println!("rev {:?}", r.iter().rev().collect::<Vec<_>>());
    println!("len {} sum {}", r.iter().len(), r.iter().sum::<i32>());
    println!("front {:?} back {:?}", r.pop_front(), r.pop_back());
    println!("{:?} get(1)={:?} get(5)={:?}", r, r.get(1), r.get(5));
    let mut it = r.iter();
    println!("{:?} {:?} {:?}", it.next(), it.next_back(), it.next());

    println!("strings:");
    let mut s: Ring<String, 3> = Ring::new();
    for w in "one two three four five".split(' ') {
        if let Some(old) = s.push(w.to_uppercase()) {
            println!("  evicted {}", old);
        }
    }
    println!("  {:?}", s);

    println!("drops:");
    {
        let mut l: Ring<Loud, 2> = Ring::new();
        l.push(Loud(1));
        l.push(Loud(2));
        let ev = l.push(Loud(3));
        println!("  evicted {}", ev.as_ref().unwrap().0);
        drop(ev);
        println!("  leaving scope");
    }

    let mut ma = MovingAvg::<3>::new();
    let avgs: Vec<String> = [3.0, 6.0, 9.0, 12.0, 0.0, 3.0].iter().map(|&x| format!("{:.2}", ma.add(x))).collect();
    println!("moving avg {:?}", avgs);
}
