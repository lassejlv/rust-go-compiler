#[derive(Debug, Clone)]
struct Span { lo: i32, hi: i32 }

impl Iterator for Span {
    type Item = i32;
    fn next(&mut self) -> Option<i32> { if self.lo < self.hi { self.lo += 1; Some(self.lo - 1) } else { None } }
    fn size_hint(&self) -> (usize, Option<usize>) { let n = (self.hi - self.lo).max(0) as usize; (n, Some(n)) }
}
impl DoubleEndedIterator for Span {
    fn next_back(&mut self) -> Option<i32> { if self.lo < self.hi { self.hi -= 1; Some(self.hi) } else { None } }
}
impl ExactSizeIterator for Span {}
impl std::iter::FusedIterator for Span {}

fn span(lo: i32, hi: i32) -> Span { Span { lo, hi } }

fn take_two<I: Iterator>(it: &mut I) -> Vec<I::Item> { it.take(2).collect() }

fn sum_rest<I: Iterator<Item = i32>>(it: I) -> i32 { it.sum() }

struct Tree { val: i32, kids: Vec<Tree> }
impl Tree {
    fn iter(&self) -> Box<dyn Iterator<Item = i32> + '_> {
        Box::new(std::iter::once(self.val).chain(self.kids.iter().flat_map(|k| k.iter())))
    }
}

fn main() {
    let s = span(0, 6);
    println!("{} {:?}", s.len(), s.size_hint());
    println!("{:?}", s.clone().rev().collect::<Vec<_>>());
    let mut it = span(1, 10);
    println!("{:?} {:?} {:?}", it.next(), it.next_back(), it.len());
    println!("{:?}", take_two(&mut it));
    println!("{:?}", take_two(&mut it.by_ref().rev()));
    println!("{} {:?}", sum_rest(&mut it), it.next());
    let mut it2 = span(0, 10);
    let firsts: Vec<_> = it2.by_ref().take_while(|&x| x < 3).collect();
    println!("{:?} rest={:?}", firsts, it2.collect::<Vec<_>>());
    let mut mixed = span(0, 8);
    let a: Vec<_> = mixed.by_ref().step_by(3).take(2).collect();
    println!("{:?} {:?}", a, mixed.collect::<Vec<_>>());
    println!("{:?}", span(0, 5).zip(span(10, 20).rev()).collect::<Vec<_>>());
    println!("{:?}", span(0, 10).rposition(|x| x % 4 == 1));
    println!("{:?}", span(1, 5).rfold(String::new(), |a, x| a + &x.to_string()));
    println!("{:?}", span(0, 7).rev().skip(1).step_by(2).collect::<Vec<_>>());
    println!("{:?}", span(0, 4).chain(span(10, 12)).rev().collect::<Vec<_>>());
    println!("{:?}", span(0, 4).map(|x| x * x).rev().nth(1));
    println!("{:?}", span(3, 3).last());
    let t = Tree { val: 1, kids: vec![Tree { val: 2, kids: vec![Tree { val: 3, kids: vec![] }] }, Tree { val: 4, kids: vec![] }] };
    println!("{:?} {}", t.iter().collect::<Vec<_>>(), t.iter().max().unwrap());
    let mut pk = span(0, 5).peekable();
    let mut groups = vec![];
    while let Some(x) = pk.next() {
        if let Some(&n) = pk.peek() { if n % 2 == 0 { pk.next(); groups.push((x, n)); continue; } }
        groups.push((x, -1));
    }
    println!("{:?}", groups);
    let v: Vec<i32> = span(0, 20).filter(|x| x % 3 == 0).skip_while(|&x| x < 5).map_while(|x| if x < 16 { Some(x * 10) } else { None }).collect();
    println!("{:?}", v);
}
