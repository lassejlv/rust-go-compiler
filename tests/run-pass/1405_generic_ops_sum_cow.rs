use std::borrow::{Borrow, Cow};
use std::collections::{BTreeSet, HashMap};
use std::iter::{FromIterator, Product, Sum};
use std::ops::{Add, AddAssign, Index, IndexMut, Mul, Neg, Sub};

#[derive(Debug, Clone, Copy, PartialEq, Default)]
struct V2<T> { x: T, y: T }

impl<T: Add<Output = T>> Add for V2<T> {
    type Output = V2<T>;
    fn add(self, o: Self) -> Self { V2 { x: self.x + o.x, y: self.y + o.y } }
}
impl<'a, T: Add<Output = T> + Copy> Add<&'a V2<T>> for &'a V2<T> {
    type Output = V2<T>;
    fn add(self, o: &'a V2<T>) -> V2<T> { V2 { x: self.x + o.x, y: self.y + o.y } }
}
impl<T: Sub<Output = T>> Sub for V2<T> {
    type Output = Self;
    fn sub(self, o: Self) -> Self { V2 { x: self.x - o.x, y: self.y - o.y } }
}
impl<T: Mul<Output = T> + Copy> Mul<T> for V2<T> {
    type Output = Self;
    fn mul(self, k: T) -> Self { V2 { x: self.x * k, y: self.y * k } }
}
impl Mul<V2<f64>> for f64 {
    type Output = V2<f64>;
    fn mul(self, v: V2<f64>) -> V2<f64> { v * self }
}
impl<T: Neg<Output = T>> Neg for V2<T> {
    type Output = Self;
    fn neg(self) -> Self { V2 { x: -self.x, y: -self.y } }
}
impl<T: AddAssign> AddAssign for V2<T> {
    fn add_assign(&mut self, o: Self) { self.x += o.x; self.y += o.y; }
}
impl<T: Add<Output = T> + Default> Sum for V2<T> {
    fn sum<I: Iterator<Item = Self>>(it: I) -> Self {
        it.fold(V2::default(), |a, b| a + b)
    }
}
impl<'a, T: Add<Output = T> + Default + Copy + 'a> Sum<&'a V2<T>> for V2<T> {
    fn sum<I: Iterator<Item = &'a V2<T>>>(it: I) -> Self {
        it.fold(V2::default(), |a, b| a + *b)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Mod7(u32);
impl Mul for Mod7 { type Output = Mod7; fn mul(self, o: Mod7) -> Mod7 { Mod7(self.0 * o.0 % 7) } }
impl Product for Mod7 {
    fn product<I: Iterator<Item = Mod7>>(it: I) -> Mod7 { it.fold(Mod7(1), Mul::mul) }
}

fn dot<T>(a: &[T], b: &[T]) -> T where T: Mul<Output = T> + Sum<T> + Copy {
    a.iter().zip(b).map(|(x, y)| *x * *y).sum()
}

#[derive(Debug, Default)]
struct Bag<T: Ord> { items: BTreeSet<T>, inserts: usize }
impl<T: Ord> FromIterator<T> for Bag<T> {
    fn from_iter<I: IntoIterator<Item = T>>(it: I) -> Self {
        let mut b = Bag { items: BTreeSet::new(), inserts: 0 };
        b.extend(it);
        b
    }
}
impl<T: Ord> Extend<T> for Bag<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, it: I) {
        for x in it { self.inserts += 1; self.items.insert(x); }
    }
}
impl<'a, T: Ord + Copy + 'a> Extend<&'a T> for Bag<T> {
    fn extend<I: IntoIterator<Item = &'a T>>(&mut self, it: I) {
        self.extend(it.into_iter().copied())
    }
}
impl<'a, T: Ord> IntoIterator for &'a Bag<T> {
    type Item = &'a T;
    type IntoIter = std::collections::btree_set::Iter<'a, T>;
    fn into_iter(self) -> Self::IntoIter { self.items.iter() }
}
impl<T: Ord> IntoIterator for Bag<T> {
    type Item = T;
    type IntoIter = std::collections::btree_set::IntoIter<T>;
    fn into_iter(self) -> Self::IntoIter { self.items.into_iter() }
}

struct Grid<T> { w: usize, cells: Vec<T> }
impl<T> Index<(usize, usize)> for Grid<T> {
    type Output = T;
    fn index(&self, (r, c): (usize, usize)) -> &T { &self.cells[r * self.w + c] }
}
impl<T> IndexMut<(usize, usize)> for Grid<T> {
    fn index_mut(&mut self, (r, c): (usize, usize)) -> &mut T { &mut self.cells[r * self.w + c] }
}

// Borrow / ToOwned / Cow with custom types
#[derive(Debug, PartialEq, Eq, Hash)]
struct Key(String);
impl Borrow<str> for Key {
    fn borrow(&self) -> &str { &self.0 }
}

#[derive(Debug)]
struct Path2([u8]);
impl Path2 {
    fn new(b: &[u8]) -> &Path2 { unsafe { &*(b as *const [u8] as *const Path2) } }
    fn len(&self) -> usize { self.0.len() }
}
#[derive(Debug, Clone)]
struct PathBuf2(Vec<u8>);
impl Borrow<Path2> for PathBuf2 {
    fn borrow(&self) -> &Path2 { Path2::new(&self.0) }
}
impl ToOwned for Path2 {
    type Owned = PathBuf2;
    fn to_owned(&self) -> PathBuf2 { PathBuf2(self.0.to_vec()) }
}

fn normalize(s: &str) -> Cow<'_, str> {
    if s.contains(' ') { Cow::Owned(s.replace(' ', "_")) } else { Cow::Borrowed(s) }
}

fn ensure_slash(p: &Path2) -> Cow<'_, Path2> {
    if p.0.last() == Some(&b'/') { Cow::Borrowed(p) } else {
        let mut v = p.to_owned();
        v.0.push(b'/');
        Cow::Owned(v)
    }
}

fn main() {
    let a = V2 { x: 1, y: 2 };
    let b = V2 { x: 10, y: 20 };
    println!("{:?} {:?} {:?}", a + b, b - a, -(a * 3));
    println!("{:?}", &a + &b);
    let mut c = a;
    c += b;
    c += V2 { x: 100, y: 100 };
    println!("{:?}", c);
    println!("{:?}", 2.0 * V2 { x: 1.5, y: -0.25 });
    let vs = vec![a, b, c];
    let s1: V2<i32> = vs.iter().sum();
    let s2: V2<i32> = vs.into_iter().sum();
    println!("{:?} {}", s1, s1 == s2);
    let p: Mod7 = (1..=6).map(Mod7).product();
    println!("{:?}", p);
    println!("{} {}", dot(&[1, 2, 3], &[4, 5, 6]), dot(&[0.5f32, 2.0], &[2.0, 0.25]));

    let mut bag: Bag<i32> = vec![5, 3, 5, 1].into_iter().collect();
    bag.extend(&[3, 9]);
    bag.extend(vec![0]);
    println!("{:?} inserts={}", bag.items, bag.inserts);
    let mut tot = 0;
    for x in &bag { tot += x; }
    println!("tot={}", tot);
    let doubled: Bag<i32> = bag.into_iter().map(|x| x * 2).collect();
    println!("{:?}", doubled.items);
    let words: Bag<&str> = "the quick the lazy".split(' ').collect();
    println!("{:?}", words);

    let mut g = Grid { w: 3, cells: vec![0; 9] };
    g[(1, 2)] = 5;
    g[(0, 0)] += 2;
    println!("{:?} {}", g.cells, g[(1, 2)]);

    let mut hm: HashMap<Key, i32> = HashMap::new();
    hm.insert(Key("alpha".into()), 1);
    hm.insert(Key("beta".into()), 2);
    println!("{:?} {:?}", hm.get("alpha"), hm.get("gamma"));

    for s in ["a b c", "abc"] {
        match normalize(s) {
            Cow::Borrowed(b) => println!("borrowed {}", b),
            Cow::Owned(o) => println!("owned {}", o),
        }
    }
    let mut cw = normalize("x");
    cw.to_mut().push('!');
    println!("{}", cw);
    let p1 = Path2::new(b"usr/");
    let p2 = Path2::new(b"usr");
    for p in [p1, p2] {
        let r = ensure_slash(p);
        let kind = match &r { Cow::Borrowed(_) => "b", Cow::Owned(_) => "o" };
        println!("{} {} {:?}", kind, r.len(), std::str::from_utf8(&r.0).unwrap());
    }
    let owned: PathBuf2 = ensure_slash(p2).into_owned();
    println!("{:?}", owned);
}
