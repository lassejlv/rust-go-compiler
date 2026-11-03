// `Self::Item` of a trait impl is related to the arguments by the impl's
// definition (`type Item = (K, Group<'a, K, I, F>)`): the key K holds what
// the key function returns (borrows of `irises`), not the borrow of the
// temporary ChunkBy that `&chunk_by(..)` takes (itertools' chunk_by).

use std::cell::RefCell;
use std::collections::HashMap;

pub struct ChunkBy<K, I: Iterator, F> {
    inner: RefCell<Inner<K, I, F>>,
}

struct Inner<K, I: Iterator, F> {
    iter: I,
    key: F,
    pending: Option<(K, I::Item)>,
}

pub struct Groups<'a, K: 'a, I: Iterator + 'a, F: 'a> {
    parent: &'a ChunkBy<K, I, F>,
}

pub struct Group<'a, K: 'a, I: Iterator + 'a, F: 'a> {
    parent: &'a ChunkBy<K, I, F>,
    key: K,
    first: Option<I::Item>,
}

impl<'a, K, I, F> IntoIterator for &'a ChunkBy<K, I, F>
where
    I: Iterator,
    I::Item: 'a,
    F: FnMut(&I::Item) -> K,
    K: PartialEq + Clone,
{
    type Item = (K, Group<'a, K, I, F>);
    type IntoIter = Groups<'a, K, I, F>;
    fn into_iter(self) -> Self::IntoIter {
        Groups { parent: self }
    }
}

impl<'a, K, I, F> Iterator for Groups<'a, K, I, F>
where
    I: Iterator,
    I::Item: 'a,
    F: FnMut(&I::Item) -> K,
    K: PartialEq + Clone,
{
    type Item = (K, Group<'a, K, I, F>);
    fn next(&mut self) -> Option<Self::Item> {
        let mut inner = self.parent.inner.borrow_mut();
        let (key, elt) = match inner.pending.take() {
            Some(p) => p,
            None => {
                let elt = inner.iter.next()?;
                let key = (inner.key)(&elt);
                (key, elt)
            }
        };
        Some((key.clone(), Group { parent: self.parent, key, first: Some(elt) }))
    }
}

impl<'a, K, I, F> Iterator for Group<'a, K, I, F>
where
    I: Iterator,
    F: FnMut(&I::Item) -> K,
    K: PartialEq,
{
    type Item = I::Item;
    fn next(&mut self) -> Option<I::Item> {
        if let Some(x) = self.first.take() {
            return Some(x);
        }
        let mut inner = self.parent.inner.borrow_mut();
        let elt = inner.iter.next()?;
        let key = (inner.key)(&elt);
        if key == self.key {
            Some(elt)
        } else {
            inner.pending = Some((key, elt));
            None
        }
    }
}

fn chunk_by<K, I: Iterator, F: FnMut(&I::Item) -> K>(iter: I, key: F) -> ChunkBy<K, I, F> {
    ChunkBy { inner: RefCell::new(Inner { iter, key, pending: None }) }
}

struct Iris {
    name: String,
    len: f32,
}

// an impl whose item borrows from the iterator itself (Self::Item = &'a T)
struct Window<'a, T> {
    data: &'a [T],
    pos: usize,
}

impl<'a, T> Iterator for Window<'a, T> {
    type Item = &'a [T];
    fn next(&mut self) -> Option<&'a [T]> {
        if self.pos + 2 > self.data.len() {
            return None;
        }
        self.pos += 1;
        Some(&self.data[self.pos - 1..self.pos + 1])
    }
}

fn windows<'a, T>(data: &'a [T]) -> Window<'a, T> {
    Window { data, pos: 0 }
}

fn main() {
    let irises = vec![
        Iris { name: "setosa".to_string(), len: 5.1 },
        Iris { name: "setosa".to_string(), len: 4.9 },
        Iris { name: "versicolor".to_string(), len: 7.0 },
        Iris { name: "virginica".to_string(), len: 6.3 },
        Iris { name: "virginica".to_string(), len: 5.8 },
    ];
    let mut symbols = "+ox".chars().cycle();
    let mut symbolmap = HashMap::new();
    for (species, chunk) in &chunk_by(irises.iter(), |iris| &iris.name) {
        symbolmap.entry(species).or_insert_with(|| symbols.next().unwrap());
        println!("{} (symbol={})", species, symbolmap[species]);
        for iris in chunk {
            println!("  {:.1}", iris.len);
        }
    }
    let mut keys: Vec<_> = symbolmap.into_iter().collect();
    keys.sort();
    println!("{:?}", keys);

    let nums = vec![1, 2, 3, 4];
    let mut longest: Option<&[i32]> = None;
    {
        let mut w = windows(&nums);
        while let Some(s) = w.next() {
            if s[0] % 2 == 1 {
                longest = Some(s);
            }
        }
    }
    println!("{:?}", longest);
}
