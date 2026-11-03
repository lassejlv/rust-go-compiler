// Valid programs that a naive borrow checker / front end might reject.
// Each module is an independent program; main runs them in order.

mod p09_hrtb_closures {
    fn apply<F>(f: F, items: &[String]) -> Vec<&str> where F: for<'a> Fn(&'a str) -> &'a str {
        items.iter().map(|s| f(s)).collect()
    }
    fn call_with_local<F: for<'a> FnMut(&'a mut Vec<i32>)>(mut f: F) -> Vec<i32> {
        let mut v = vec![1];
        f(&mut v);
        f(&mut v);
        v
    }
    fn compose<'x>(fs: &'x [Box<dyn Fn(&str) -> String>]) -> impl Fn(&str) -> String + 'x {
        move |s| fs.iter().fold(s.to_string(), |acc, f| f(&acc))
    }
    pub fn main() {
        let items = vec!["  a  ".to_string(), "b ".to_string()];
        let trimmed = apply(|s| s.trim(), &items);
        let firsts = apply(|s| &s[..1], &items);
        let mut counter = 0;
        let v = call_with_local(|v| { counter += 1; v.push(counter); });
        let fs: Vec<Box<dyn Fn(&str) -> String>> = vec![Box::new(|s| s.to_uppercase()), Box::new(|s| format!("<{}>", s))];
        let c = compose(&fs);
        println!("{:?} {:?} {:?} {}", trimmed, firsts, v, c("hi"));
    }
}

mod p45_closure_returning_borrows {
    fn make_lookup<'a>(table: &'a [(&'a str, i32)]) -> impl Fn(&str) -> Option<i32> + 'a {
        move |k| table.iter().find(|(n, _)| *n == k).map(|(_, v)| *v)
    }
    fn compose<A, B, C>(f: impl Fn(A) -> B, g: impl Fn(B) -> C) -> impl Fn(A) -> C { move |x| g(f(x)) }
    pub fn main() {
        let table = vec![("one", 1), ("two", 2)];
        let look = make_lookup(&table);
        let v = vec![10, 20, 30];
        let get = |i: usize| -> &i32 { &v[i] };
        let a = get(0);
        let b = get(2);
        let mut words = vec!["x".to_string()];
        let mut add = |w: &str| { words.push(w.to_string()); words.len() };
        add("y");
        let n = add("z");
        let f = compose(|x: i32| x + 1, |y: i32| y.to_string() + "!");
        println!("{:?} {:?} {} {} {} {:?} {}", look("two"), look("three"), a, b, n, words, f(41));
    }
}

mod p59_closures_in_struct_generic {
    struct Cache<F: Fn(u64) -> u64> { f: F, memo: std::collections::HashMap<u64, u64> }
    impl<F: Fn(u64) -> u64> Cache<F> {
        fn get(&mut self, k: u64) -> u64 {
            if let Some(v) = self.memo.get(&k) { return *v; }
            let v = (self.f)(k);
            self.memo.insert(k, v);
            v
        }
    }
    fn retry<T, E>(mut op: impl FnMut(u32) -> Result<T, E>, n: u32) -> Result<T, E> {
        let mut last = op(0);
        for i in 1..n { if last.is_ok() { break; } last = op(i); }
        last
    }
    pub fn main() {
        let calls = std::cell::Cell::new(0);
        let mut c = Cache { f: |k| { calls.set(calls.get() + 1); k * k }, memo: Default::default() };
        let a = c.get(4) + c.get(4) + c.get(5);
        let mut attempts = vec![];
        let r: Result<u32, String> = retry(|i| { attempts.push(i); if i == 2 { Ok(i * 7) } else { Err(format!("e{}", i)) } }, 5);
        println!("{} {} {:?} {:?}", a, calls.get(), r, attempts);
    }
}

fn main() {
    p09_hrtb_closures::main();
    p45_closure_returning_borrows::main();
    p59_closures_in_struct_generic::main();
}
