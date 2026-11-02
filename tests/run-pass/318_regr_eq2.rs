use std::borrow::Borrow;
pub trait Equivalent<K: ?Sized> { fn equivalent(&self, key: &K) -> bool; }
impl<Q: ?Sized, K: ?Sized> Equivalent<K> for Q where Q: Eq, K: Borrow<Q> {
    fn equivalent(&self, key: &K) -> bool { self == key.borrow() }
}
fn equivalent_key<Q, K, V>(k: &Q) -> impl Fn(&(K, V)) -> bool + '_ where Q: Equivalent<K> + ?Sized {
    move |x| k.equivalent(&x.0)
}
struct Table<T> { items: Vec<T> }
impl<T> Table<T> {
    fn get(&self, mut eq: impl FnMut(&T) -> bool) -> Option<&T> { self.items.iter().find(|e| eq(e)) }
}
struct Map<K, V> { t: Table<(K, V)> }
impl<K, V> Map<K, V> {
    fn get<Q: ?Sized + Equivalent<K>>(&self, k: &Q) -> Option<&V> { self.t.get(equivalent_key(k)).map(|e| &e.1) }
}
fn main() {
    let m = Map { t: Table { items: vec![("a".to_string(), 1), ("b".to_string(), 2)] } };
    println!("{:?} {:?}", m.get("b"), m.get("zz"));
}
