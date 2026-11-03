// Valid programs that a naive borrow checker / front end might reject.
// Each module is an independent program; main runs them in order.

mod p03_get_or_insert_with {
    use std::collections::HashMap;
    struct Lazy { cache: Option<Vec<u32>>, hits: u32 }
    impl Lazy {
        fn get(&mut self) -> &Vec<u32> {
            self.hits += 1;
            self.cache.get_or_insert_with(|| (1..=3).collect())
        }
    }
    pub fn main() {
        let mut o: Option<String> = None;
        let s = o.get_or_insert_with(String::new);
        s.push_str("hi");
        s.push('!');
        println!("{:?}", o);
        let mut l = Lazy { cache: None, hits: 0 };
        let a = l.get().len();
        let b = l.get()[2];
        println!("{} {} {}", a, b, l.hits);
        let mut m: HashMap<&str, Vec<i32>> = HashMap::new();
        for (k, v) in [("a", 1), ("b", 2), ("a", 3)] {
            m.entry(k).or_insert_with(Vec::new).push(v);
        }
        let e = m.entry("c").or_default();
        e.push(9);
        let mut keys: Vec<_> = m.iter().map(|(k, v)| format!("{}={:?}", k, v)).collect();
        keys.sort();
        println!("{:?}", keys);
    }
}

mod p13_get_mut_else_insert {
    use std::collections::HashMap;
    fn add(m: &mut HashMap<String, u32>, k: &str) {
        if let Some(x) = m.get_mut(k) { *x += 1; } else { m.insert(k.to_string(), 1); }
    }
    fn get_or_default<'m>(m: &'m mut HashMap<String, u32>, k: &str) -> &'m mut u32 {
        if !m.contains_key(k) { m.insert(k.to_string(), 0); }
        m.get_mut(k).unwrap()
    }
    pub fn main() {
        let mut m = HashMap::new();
        for w in "a b a c a b".split(' ') { add(&mut m, w); }
        *get_or_default(&mut m, "z") += 7;
        let mut v: Vec<_> = m.into_iter().collect();
        v.sort();
        println!("{:?}", v);
    }
}

mod p44_generic_struct_methods_refs {
    use std::collections::HashMap;
    use std::hash::Hash;
    struct Index<K, V> { map: HashMap<K, Vec<V>> }
    impl<K: Hash + Eq + Clone, V: Clone + PartialEq> Index<K, V> {
        fn new() -> Self { Index { map: HashMap::new() } }
        fn add(&mut self, k: &K, v: V) -> &mut Self {
            let e = self.map.entry(k.clone()).or_default();
            if !e.contains(&v) { e.push(v); }
            self
        }
        fn get(&self, k: &K) -> &[V] { self.map.get(k).map(|v| v.as_slice()).unwrap_or(&[]) }
        fn get_or_create(&mut self, k: K) -> &mut Vec<V> { self.map.entry(k).or_insert_with(Vec::new) }
        fn first_mut(&mut self, k: &K) -> Option<&mut V> { self.map.get_mut(k)?.first_mut() }
    }
    pub fn main() {
        let mut ix: Index<&str, u32> = Index::new();
        ix.add(&"a", 1).add(&"a", 2).add(&"a", 1).add(&"b", 9);
        ix.get_or_create("c").push(5);
        if let Some(x) = ix.first_mut(&"a") { *x = 100; }
        println!("{:?} {:?} {:?} {:?}", ix.get(&"a"), ix.get(&"b"), ix.get(&"c"), ix.get(&"zz"));
    }
}

fn main() {
    p03_get_or_insert_with::main();
    p13_get_mut_else_insert::main();
    p44_generic_struct_methods_refs::main();
}
