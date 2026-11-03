// Valid programs that a naive borrow checker / front end might reject.
// Each module is an independent program; main runs them in order.

mod p01_two_phase_method_chain {
    struct Q { items: Vec<usize>, log: Vec<String> }
    impl Q {
        fn add(&mut self) { self.items.push(self.items.len()); }
        fn note(&mut self, s: String) -> &mut Self { self.log.push(s); self }
    }
    pub fn main() {
        let mut v: Vec<usize> = vec![10];
        v.push(v.len());
        v.insert(v.len(), v[0] + v[1]);
        let n = v.iter().sum::<usize>();
        v.push(n);
        let mut q = Q { items: vec![], log: vec![] };
        q.add(); q.add();
        let l = q.items.len();
        q.note(format!("a{}", l)).note(format!("b{}", l + 1));
        q.note(format!("c{}", q.items[1]));
        let mut s = String::from("ab");
        s.push_str(&s.clone());
        s.truncate(s.len() - 1);
        println!("{:?} {:?} {:?} {}", v, q.items, q.log, s);
    }
}

mod p12_nll_after_last_use {
    pub fn main() {
        let mut v = vec![1, 2, 3];
        let r = &v[0];
        let x = *r;
        v.push(x);
        let m = &mut v;
        m.push(9);
        let len = v.len();
        let first = v.first_mut().unwrap();
        *first = len as i32;
        println!("{:?}", v);
        let mut s = String::from("hello");
        let w = s.as_str().split('l').next().unwrap().len();
        s.push_str(&w.to_string());
        let mut count = 0;
        let mut opt = Some(&mut count);
        if let Some(c) = opt.take() { *c += 1; }
        count += 1;
        println!("{} {}", s, count);
    }
}

mod p34_conditional_borrows {
    pub fn main() {
        let mut a = vec![1];
        let mut b = vec![2];
        let flag = a.len() == 1;
        let r: &Vec<i32>;
        if flag { r = &a; } else { r = &b; }
        let x = r[0];
        a.push(x);
        b.push(x);
        let target: &mut Vec<i32> = if a.len() > b.len() { &mut a } else { &mut b };
        target.push(100);
        let at = |i: usize| &a[i];
        let first = at(0);
        let second = at(1);
        println!("{} {} {:?}", first, second, b);
        let mut p = (1, 2);
        std::mem::swap(&mut p.0, &mut p.1);
        let (ra, rb) = (&mut p.0, &mut p.1);
        *ra += *rb;
        let pair = (&mut a, &b);
        match pair { (va, vb) => va.extend(vb.iter().copied()) }
        a.extend(a.clone());
        println!("{:?} {:?}", p, a);
    }
}

fn main() {
    p01_two_phase_method_chain::main();
    p12_nll_after_last_use::main();
    p34_conditional_borrows::main();
}
