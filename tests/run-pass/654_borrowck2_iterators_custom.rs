// Valid programs that a naive borrow checker / front end might reject.
// Each module is an independent program; main runs them in order.

mod p31_custom_iter_lifetimes {
    struct Coll { items: Vec<String> }
    struct Iter<'a> { c: &'a Coll, i: usize }
    impl<'a> Iterator for Iter<'a> {
        type Item = &'a str;
        fn next(&mut self) -> Option<&'a str> {
            let r = self.c.items.get(self.i)?;
            self.i += 1;
            Some(r.as_str())
        }
    }
    impl<'a> IntoIterator for &'a Coll {
        type Item = &'a str;
        type IntoIter = Iter<'a>;
        fn into_iter(self) -> Iter<'a> { Iter { c: self, i: 0 } }
    }
    impl Coll {
        fn long(&self, n: usize) -> impl Iterator<Item = &String> + '_ { self.items.iter().filter(move |s| s.len() > n) }
        fn iter_mut(&mut self) -> std::slice::IterMut<'_, String> { self.items.iter_mut() }
    }
    pub fn main() {
        let mut c = Coll { items: vec!["a".into(), "bbb".into(), "cc".into()] };
        let mut collected = Vec::new();
        for s in &c { collected.push(s); }
        let longs: Vec<_> = c.long(1).cloned().collect();
        println!("{:?} {:?}", collected, longs);
        for s in c.iter_mut() { s.push('!'); }
        let first = (&c).into_iter().next().unwrap();
        println!("{} {}", first, c.items.join(","));
    }
}

mod p54_iterator_adapters_struct {
    struct Evens<'a, I: Iterator<Item = &'a i32>> { inner: I }
    impl<'a, I: Iterator<Item = &'a i32>> Iterator for Evens<'a, I> {
        type Item = &'a i32;
        fn next(&mut self) -> Option<&'a i32> { self.inner.by_ref().find(|x| **x % 2 == 0) }
    }
    fn evens<'a>(v: &'a [i32]) -> Evens<'a, std::slice::Iter<'a, i32>> { Evens { inner: v.iter() } }
    fn pairs<'a>(a: &'a [i32], b: &'a [i32]) -> impl Iterator<Item = (&'a i32, &'a i32)> + 'a { a.iter().zip(b.iter()) }
    pub fn main() {
        let v = vec![1, 2, 3, 4, 6];
        let e: Vec<_> = evens(&v).collect();
        let w = vec![10, 20];
        let p: Vec<_> = pairs(&v, &w).map(|(x, y)| x * y).collect();
        let mut it = v.iter().peekable();
        let mut runs = 0;
        while let Some(x) = it.next() { if let Some(&&n) = it.peek() { if n == x + 1 { runs += 1; } } }
        let windows: Vec<i32> = v.windows(2).map(|w| w[1] - w[0]).collect();
        let (small, big): (Vec<&i32>, Vec<&i32>) = v.iter().partition(|x| **x < 3);
        println!("{:?} {:?} {} {:?} {:?} {:?}", e, p, runs, windows, small, big);
    }
}

mod p28_iter_chains_refs {
    use std::collections::HashMap;
    #[derive(Debug)]
    struct Emp { name: String, dept: String, salary: u32 }
    pub fn main() {
        let emps = vec![
            Emp { name: "a".into(), dept: "x".into(), salary: 10 },
            Emp { name: "b".into(), dept: "y".into(), salary: 30 },
            Emp { name: "c".into(), dept: "x".into(), salary: 20 },
        ];
        let mut by: HashMap<&str, Vec<&Emp>> = HashMap::new();
        for e in &emps { by.entry(e.dept.as_str()).or_default().push(e); }
        let mut tops: Vec<(&str, &str)> = by.iter().map(|(d, es)| (*d, es.iter().max_by_key(|e| e.salary).unwrap().name.as_str())).collect();
        tops.sort();
        let threshold = 15;
        let names: Vec<&String> = emps.iter().filter(|e| e.salary > threshold).map(|e| &e.name).collect();
        let total: u32 = emps.iter().map(|e| e.salary).sum();
        println!("{:?} {:?} {}", tops, names, total);
        let mut sal: Vec<&mut u32> = Vec::new();
        let mut emps = emps;
        for e in emps.iter_mut() { sal.push(&mut e.salary); }
        for s in sal { *s += 1; }
        let max = emps.iter().map(|e| e.salary).max().unwrap();
        println!("{}", max);
    }
}

mod p46_sort_by_key_borrows {
    #[derive(Debug, Clone)]
    struct P { name: String, age: u32 }
    pub fn main() {
        let mut ps = vec![P { name: "bob".into(), age: 30 }, P { name: "al".into(), age: 25 }, P { name: "cy".into(), age: 30 }];
        ps.sort_by(|a, b| b.age.cmp(&a.age).then_with(|| a.name.cmp(&b.name)));
        let oldest: Vec<&str> = ps.iter().take_while(|p| p.age == ps[0].age).map(|p| p.name.as_str()).collect();
        let max_name = ps.iter().max_by_key(|p| p.name.len()).map(|p| &p.name).unwrap().clone();
        println!("{:?} {}", oldest, max_name);
        ps.sort_by_key(|p| p.name.clone());
        ps.dedup_by_key(|p| p.age);
        let pos = ps.binary_search_by(|p| p.name.as_str().cmp("bob"));
        let mut names: Vec<&mut String> = ps.iter_mut().map(|p| &mut p.name).collect();
        names.iter_mut().for_each(|n| n.push('_'));
        println!("{:?} {:?}", pos, ps);
    }
}

fn main() {
    p31_custom_iter_lifetimes::main();
    p54_iterator_adapters_struct::main();
    p28_iter_chains_refs::main();
    p46_sort_by_key_borrows::main();
}
