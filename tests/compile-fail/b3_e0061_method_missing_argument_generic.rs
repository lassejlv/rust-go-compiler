//~ ERROR E0061
struct Cache<K, V> { k: Vec<K>, v: Vec<V> }
impl<K: PartialEq, V> Cache<K, V> {
    fn put(&mut self, k: K, v: V) { self.k.push(k); self.v.push(v); }
}
fn main() { let mut c = Cache { k: vec![], v: vec![] }; c.put(1); let _: &Vec<i32> = &c.v; }
