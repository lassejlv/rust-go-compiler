use std::fmt::Debug;

trait Store {
    type Key: Ord + Debug + Clone;
    type Val: Debug;
    fn entries(&self) -> Vec<(Self::Key, Self::Val)>;
}

trait Indexed {
    type S: Store;
    fn store(&self) -> &Self::S;
    fn first_key(&self) -> Option<<Self::S as Store>::Key> {
        self.store().entries().into_iter().map(|(k, _)| k).min()
    }
}

struct Pairs(Vec<(u32, String)>);
impl Store for Pairs {
    type Key = u32;
    type Val = String;
    fn entries(&self) -> Vec<(u32, String)> { self.0.clone() }
}
struct Db { p: Pairs }
impl Indexed for Db {
    type S = Pairs;
    fn store(&self) -> &Pairs { &self.p }
}

fn dump<I: Indexed>(i: &I) -> Vec<String>
where
    <I::S as Store>::Val: Clone,
{
    let mut e = i.store().entries();
    e.sort_by(|a, b| a.0.cmp(&b.0));
    e.iter().map(|(k, v)| format!("{:?}={:?}", k, v)).collect()
}

fn flatten_sum<I>(outer: I) -> i64
where
    I: IntoIterator,
    I::Item: IntoIterator,
    <I::Item as IntoIterator>::Item: Into<i64>,
{
    outer.into_iter().flat_map(|inner| inner.into_iter().map(Into::into)).sum()
}

fn first_of_first<I>(mut it: I) -> Option<<<I as Iterator>::Item as IntoIterator>::Item>
where
    I: Iterator,
    I::Item: IntoIterator,
{
    it.next()?.into_iter().next()
}

trait Monoid { fn empty() -> Self; fn combine(self, o: Self) -> Self; }
impl Monoid for String { fn empty() -> Self { String::new() } fn combine(self, o: Self) -> Self { self + &o } }
impl Monoid for i32 { fn empty() -> Self { 0 } fn combine(self, o: Self) -> Self { self + o } }
impl<T: Monoid, U: Monoid> Monoid for (T, U) {
    fn empty() -> Self { (T::empty(), U::empty()) }
    fn combine(self, o: Self) -> Self { (self.0.combine(o.0), self.1.combine(o.1)) }
}
impl<T> Monoid for Vec<T> { fn empty() -> Self { vec![] } fn combine(mut self, o: Self) -> Self { self.extend(o); self } }
fn concat_all<I>(it: I) -> I::Item where I: Iterator, I::Item: Monoid {
    it.fold(<I::Item as Monoid>::empty(), Monoid::combine)
}

fn main() {
    let db = Db { p: Pairs(vec![(5, "five".into()), (2, "two".into()), (9, "nine".into())]) };
    println!("{:?}", db.first_key());
    println!("{:?}", dump(&db));
    println!("{}", flatten_sum(vec![vec![1i32, 2], vec![3], vec![]]));
    println!("{}", flatten_sum(vec![[1u8, 2, 3], [4, 5, 6]]));
    println!("{:?}", first_of_first(vec![vec!['a', 'b'], vec!['c']].into_iter()));
    println!("{:?}", first_of_first(vec![Vec::<i32>::new()].into_iter()));
    println!("{:?}", first_of_first(std::iter::empty::<Option<u8>>()));
    println!("{:?}", concat_all(vec!["a".to_string(), "b".into(), "c".into()].into_iter()));
    println!("{:?}", concat_all(vec![(1, "x".to_string()), (2, "y".to_string())].into_iter()));
    println!("{:?}", concat_all(vec![vec![1], vec![2, 3]].into_iter()));
    println!("{:?}", concat_all(std::iter::empty::<(i32, (String, i32))>()));
}
