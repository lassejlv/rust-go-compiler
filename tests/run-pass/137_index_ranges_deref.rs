use std::ops::{Deref, DerefMut, Index, IndexMut, Range, RangeFrom, RangeFull, RangeInclusive, RangeTo};

struct Buffer {
    data: Vec<i32>,
    reads: std::cell::Cell<usize>,
}

impl Index<usize> for Buffer {
    type Output = i32;
    fn index(&self, i: usize) -> &i32 {
        self.reads.set(self.reads.get() + 1);
        &self.data[i % self.data.len()]
    }
}
impl IndexMut<usize> for Buffer {
    fn index_mut(&mut self, i: usize) -> &mut i32 {
        let n = self.data.len();
        &mut self.data[i % n]
    }
}
impl Index<Range<usize>> for Buffer {
    type Output = [i32];
    fn index(&self, r: Range<usize>) -> &[i32] {
        &self.data[r]
    }
}
impl Index<RangeFrom<usize>> for Buffer {
    type Output = [i32];
    fn index(&self, r: RangeFrom<usize>) -> &[i32] {
        &self.data[r]
    }
}
impl Index<RangeTo<usize>> for Buffer {
    type Output = [i32];
    fn index(&self, r: RangeTo<usize>) -> &[i32] {
        &self.data[r]
    }
}
impl Index<RangeInclusive<usize>> for Buffer {
    type Output = [i32];
    fn index(&self, r: RangeInclusive<usize>) -> &[i32] {
        &self.data[r]
    }
}
impl Index<RangeFull> for Buffer {
    type Output = [i32];
    fn index(&self, _r: RangeFull) -> &[i32] {
        &self.data
    }
}

struct Grid {
    w: usize,
    cells: Vec<char>,
}
impl Index<(usize, usize)> for Grid {
    type Output = char;
    fn index(&self, (r, c): (usize, usize)) -> &char {
        &self.cells[r * self.w + c]
    }
}
impl IndexMut<(usize, usize)> for Grid {
    fn index_mut(&mut self, (r, c): (usize, usize)) -> &mut char {
        &mut self.cells[r * self.w + c]
    }
}

struct Name(String);
impl Deref for Name {
    type Target = String;
    fn deref(&self) -> &String {
        &self.0
    }
}
impl DerefMut for Name {
    fn deref_mut(&mut self) -> &mut String {
        &mut self.0
    }
}

struct Wrapper<T> {
    inner: Vec<T>,
}
impl<T> Deref for Wrapper<T> {
    type Target = Vec<T>;
    fn deref(&self) -> &Vec<T> {
        &self.inner
    }
}
impl<T> DerefMut for Wrapper<T> {
    fn deref_mut(&mut self) -> &mut Vec<T> {
        &mut self.inner
    }
}

fn takes_str(s: &str) -> usize {
    s.chars().filter(|c| c.is_uppercase()).count()
}
fn takes_slice(s: &[u8]) -> u32 {
    s.iter().map(|&b| b as u32).sum()
}

fn main() {
    let mut b = Buffer { data: vec![10, 20, 30, 40, 50], reads: std::cell::Cell::new(0) };
    println!("{} {} {}", b[0], b[4], b[7]);
    b[6] += 5;
    println!("{:?}", &b[1..3]);
    println!("{:?}", &b[3..]);
    println!("{:?}", &b[..2]);
    println!("{:?}", &b[1..=3]);
    println!("{:?}", &b[..]);
    println!("reads = {}", b.reads.get());

    let mut g = Grid { w: 3, cells: "abcdefghi".chars().collect() };
    g[(1, 1)] = 'X';
    for r in 0..3 {
        let row: String = (0..3).map(|c| g[(r, c)]).collect();
        println!("{}", row);
    }

    let mut n = Name("Hello World".to_string());
    println!("len {} upper {}", n.len(), takes_str(&n));
    n.push_str(" Again");
    println!("{} | {}", *n, n.to_lowercase());
    println!("{}", n.starts_with("Hello"));

    let mut w = Wrapper { inner: vec![3u8, 1, 2] };
    w.push(9);
    w.sort();
    println!("{:?} first={:?} sum={}", *w, w.first(), takes_slice(&w));
    let total: u8 = w.iter().sum();
    println!("total {} len {} contains 9: {}", total, w.len(), w.contains(&9));
    let boxed: Box<Name> = Box::new(Name("Boxed".into()));
    println!("{}", boxed.to_uppercase());
}
