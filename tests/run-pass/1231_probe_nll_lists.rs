mod c01 {
    #[derive(Debug)]
    struct Node { v: i32, next: Option<Box<Node>> }
    fn push_back(head: &mut Option<Box<Node>>, v: i32) {
        let mut cur = head;
        while let Some(node) = cur { cur = &mut node.next; }
        *cur = Some(Box::new(Node { v, next: None }));
    }
    fn remove_all(head: &mut Option<Box<Node>>, v: i32) {
        let mut cur = head;
        loop {
            match cur {
                None => break,
                Some(n) if n.v == v => { *cur = n.next.take(); }
                Some(n) => { cur = &mut n.next; }
            }
        }
    }
    fn last_mut(head: &mut Option<Box<Node>>) -> Option<&mut i32> {
        let mut cur = head.as_mut()?;
        while cur.next.is_some() { cur = cur.next.as_mut().unwrap(); }
        Some(&mut cur.v)
    }
    fn to_vec(head: &Option<Box<Node>>) -> Vec<i32> {
        let mut out = vec![];
        let mut c = head;
        while let Some(n) = c { out.push(n.v); c = &n.next; }
        out
    }
    pub fn main() {
        let mut h = None;
        for v in [1, 2, 3, 2, 4, 2] { push_back(&mut h, v); }
        println!("{:?}", to_vec(&h));
        remove_all(&mut h, 2);
        println!("{:?}", to_vec(&h));
        if let Some(l) = last_mut(&mut h) { *l *= 100; }
        println!("{:?}", to_vec(&h));
    }
}

mod c02 {
    use std::collections::HashMap;
    struct Cache { map: HashMap<u32, String>, hits: u32 }
    impl Cache {
        fn get(&mut self, k: u32) -> &String {
            if self.map.contains_key(&k) { self.hits += 1; return &self.map[&k]; }
            self.map.insert(k, format!("v{}", k));
            &self.map[&k]
        }
        fn get_or_compute(&mut self, k: u32, f: impl FnOnce(u32) -> String) -> &mut String {
            self.map.entry(k).or_insert_with(|| f(k))
        }
        fn longest(&self) -> Option<&str> { self.map.values().max_by_key(|s| s.len()).map(|s| s.as_str()) }
    }
    fn first_word(s: &mut String) -> &str { s.push('!'); let i = s.find(' ').unwrap_or(s.len()); &s[..i] }
    pub fn main() {
        let mut c = Cache { map: HashMap::new(), hits: 0 };
        let a = c.get(1).clone();
        let b = c.get(1).len();
        c.get_or_compute(7, |k| "x".repeat(k as usize)).push('y');
        println!("{} {} {} {:?}", a, b, c.hits, c.longest());
        let mut s = String::from("hello world");
        let w = first_word(&mut s);
        println!("{}", w);
        let mut v = vec![1, 2, 3];
        let last = v.last().copied();
        if let Some(l) = last { v.push(l + 1); }
        let first = &mut v[0];
        *first += 10;
        let total: i32 = v.iter().sum();
        v.push(total);
        println!("{:?}", v);
        let mut stack = vec![vec![1], vec![2, 3]];
        while let Some(mut top) = stack.pop() {
            if let Some(x) = top.pop() { if x > 1 { stack.push(vec![x - 1; 1]); } }
            if stack.len() > 5 { break; }
        }
        println!("{}", stack.len());
        let mut m: HashMap<&str, Vec<i32>> = HashMap::new();
        for (k, v) in [("a", 1), ("b", 2), ("a", 3)] {
            match m.get_mut(k) { Some(list) => list.push(v), None => { m.insert(k, vec![v]); } }
        }
        let mut keys: Vec<_> = m.iter().map(|(k, v)| (*k, v.len())).collect();
        keys.sort();
        println!("{:?}", keys);
    }
}

mod c03 {
    #[derive(Debug, Default)]
    struct State { items: Vec<String>, selected: Option<usize>, log: Vec<String> }
    impl State {
        fn selected_mut(&mut self) -> Option<&mut String> { let i = self.selected?; self.items.get_mut(i) }
        fn select_and_edit(&mut self, i: usize) {
            self.selected = Some(i);
            let name = self.items[i].clone();
            if let Some(s) = self.selected_mut() { s.push_str("*"); }
            self.log.push(format!("edited {}", name));
        }
        fn split(&mut self) -> (&mut Vec<String>, &mut Vec<String>) { (&mut self.items, &mut self.log) }
    }
    pub fn main() {
        let mut st = State { items: vec!["a".into(), "b".into()], ..Default::default() };
        st.select_and_edit(1);
        let (items, log) = st.split();
        items.push("c".into());
        log.push(format!("{} items", items.len()));
        println!("{:?}", st);
        let mut arr = [1, 2, 3, 4];
        let (x, y) = arr.split_at_mut(2);
        x[0] += y[1];
        let [a, b, ..] = &mut arr;
        std::mem::swap(a, b);
        println!("{:?}", arr);
        let mut s = String::new();
        {
            let r = &mut s;
            r.push('a');
            let r2 = &mut *r;
            r2.push('b');
            r.push('c');
        }
        println!("{}", s);
        let mut v = vec![String::from("x")];
        let n = v.iter().filter(|s| s.len() == 1).count();
        v[0].push_str(&n.to_string());
        let moved: Vec<String> = v.drain(..).map(|mut s| { s.push('!'); s }).collect();
        println!("{:?} {:?}", moved, v);
        let mut opt = Some(String::from("o"));
        if let Some(ref mut s) = opt { s.push('k'); }
        let taken = opt.take();
        println!("{:?} {:?}", taken, opt);
        let mut nums = vec![3, 1, 2];
        let max_i = (0..nums.len()).max_by_key(|&i| nums[i]).unwrap();
        nums[max_i] = 0;
        nums.sort_by_key(|&x| std::cmp::Reverse(x));
        println!("{:?}", nums);
    }
}

fn main() {
    println!("== c01");
    c01::main();
    println!("== c02");
    c02::main();
    println!("== c03");
    c03::main();
}
