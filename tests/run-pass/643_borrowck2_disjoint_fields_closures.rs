// Valid programs that a naive borrow checker / front end might reject.
// Each module is an independent program; main runs them in order.

mod p04_disjoint_field_iter {
    struct S { src: Vec<i32>, dst: Vec<i32>, total: i32, keep: Vec<i32> }
    impl S {
        fn copy(&mut self) {
            for x in self.src.iter() {
                self.dst.push(*x * 2);
                self.total += *x;
            }
        }
        fn filter(&mut self) {
            self.dst.retain(|x| self.keep.contains(x));
        }
        fn mut_both(&mut self) {
            let S { src, dst, .. } = self;
            for (a, b) in src.iter_mut().zip(dst.iter()) { *a += *b; }
        }
    }
    pub fn main() {
        let mut s = S { src: vec![1, 2, 3], dst: vec![], total: 0, keep: vec![2, 6] };
        s.copy();
        s.filter();
        s.mut_both();
        let a = &mut s.src;
        let b = &s.dst;
        a.push(b.len() as i32);
        println!("{:?} {:?} {}", s.src, s.dst, s.total);
    }
}

mod p06_closure_disjoint_capture {
    #[derive(Debug)]
    struct State { count: u32, name: String, log: Vec<String> }
    pub fn main() {
        let mut st = State { count: 0, name: "st".into(), log: vec![] };
        let mut inc = || st.count += 1;
        st.log.push(st.name.clone());
        inc();
        inc();
        let name_len = || st.name.len();
        st.count += name_len() as u32;
        let mut push = |s: &str| st.log.push(s.to_string());
        push("a");
        push("b");
        println!("{:?}", st);
        let mut pair = (vec![1], vec![2]);
        let mut f = || pair.0.push(3);
        pair.1.push(4);
        f();
        println!("{:?}", pair);
    }
}

mod p36_dyn_lifetimes_closures {
    struct Bus<'a> { subs: Vec<Box<dyn FnMut(&str) + 'a>> }
    impl<'a> Bus<'a> {
        fn sub(&mut self, f: impl FnMut(&str) + 'a) { self.subs.push(Box::new(f)); }
        fn emit(&mut self, m: &str) { for s in self.subs.iter_mut() { s(m); } }
    }
    trait Visitor { fn visit(&mut self, n: i32); fn result(&self) -> String { String::from("?") } }
    struct Sum<'a> { total: &'a mut i32 }
    impl<'a> Visitor for Sum<'a> { fn visit(&mut self, n: i32) { *self.total += n; } }
    fn walk(v: &mut dyn Visitor, xs: &[i32]) { for x in xs { v.visit(*x); } }
    pub fn main() {
        let mut log: Vec<String> = Vec::new();
        let mut count = 0;
        {
            let mut bus = Bus { subs: vec![] };
            bus.sub(|m| log.push(m.to_uppercase()));
            bus.sub(|_| count += 1);
            bus.emit("a");
            bus.emit("b");
        }
        let mut total = 0;
        let mut s = Sum { total: &mut total };
        walk(&mut s, &[1, 2, 3]);
        let res = s.result();
        println!("{:?} {} {} {}", log, count, total, res);
    }
}

fn main() {
    p04_disjoint_field_iter::main();
    p06_closure_disjoint_capture::main();
    p36_dyn_lifetimes_closures::main();
}
