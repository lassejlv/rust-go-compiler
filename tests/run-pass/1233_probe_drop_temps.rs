mod e01 {
    struct D(&'static str);
    impl Drop for D { fn drop(&mut self) { println!("  drop {}", self.0); } }
    impl D { fn len(&self) -> usize { self.0.len() } fn ok(&self) -> bool { true } fn get(&self) -> Option<&'static str> { Some(self.0) } }
    fn tail() -> usize { let _x = D("local"); D("tail-temp").len() }
    fn tail_block() -> usize { let _y = D("outer"); { let _z = D("inner"); D("inner-tail").len() } }
    pub fn main() {
        println!("tail:"); println!("{}", tail());
        println!("tail_block:"); println!("{}", tail_block());
        println!("if cond:");
        if D("cond").ok() { println!("  in then"); }
        println!("if let:");
        if let Some(s) = D("iflet").get() { println!("  in then {}", s); } else { println!("  else"); }
        println!("match guard:");
        match D("scrut").get() { Some(x) if D("guard").ok() => println!("  arm {}", x), _ => println!("  other") }
        println!("while let:");
        let mut n = 0;
        while let Some(_) = (if n < 2 { D("wl").get() } else { None }) { println!("  body {}", n); n += 1; }
        println!("vec order:");
        { let _v = vec![D("v0"), D("v1"), D("v2")]; }
        println!("array order:");
        { let _a = [D("a0"), D("a1")]; }
        println!("tuple order:");
        { let _t = (D("t0"), D("t1")); }
        println!("let tuple pattern:");
        { let (_p, _q) = (D("p"), D("q")); }
        println!("truncate:");
        { let mut v = vec![D("w0"), D("w1"), D("w2"), D("w3")]; v.truncate(1); println!("  truncated"); }
        println!("assign through ref:");
        { let mut a = D("old"); let r = &mut a; *r = D("new"); println!("  assigned"); }
        println!("option replace:");
        { let mut o = Some(D("o1")); o = None; println!("  set none"); o = Some(D("o2")); let _ = o; println!("  end"); }
        println!("closure captures:");
        { let c1 = D("cap1"); let c2 = D("cap2"); let f = move || { let _ = (&c2, &c1); }; f(); println!("  before drop f"); drop(f); println!("  after drop f"); }
        println!("box slice:");
        { let _b: Box<[D]> = vec![D("b0"), D("b1")].into_boxed_slice(); }
        println!("shadowing:");
        { let s = D("s1"); let s = D("s2"); let _ = &s; println!("  shadowed"); }
        println!("struct expr temp:");
        { struct W { a: D, b: D } let w = W { b: D("wb"), a: D("wa") }; let W { a, .. } = w; println!("  moved a {}", a.0); }
        println!("fn arg temps:");
        fn take(a: &D, b: D) -> usize { a.len() + b.len() }
        println!("  {}", take(&D("argref"), D("argval")));
        println!("method chain temps:");
        let l = D("chain").get().map(|s| s.len()).unwrap_or(0);
        println!("  {}", l);
        println!("end");
    }
}

mod e02 {
    use std::sync::Mutex;
    use std::cell::RefCell;
    pub fn main() {
        let m = Mutex::new(vec![1, 2, 3]);
        while let Some(x) = m.lock().unwrap().pop() { println!("popped {}", x); if x == 2 { break; } }
        println!("{:?}", m.lock().unwrap());
        let c = RefCell::new(vec![5, 6]);
        let len = c.borrow().len();
        c.borrow_mut().push(len);
        match c.borrow().last() { Some(&x) => println!("last {}", x), None => {} }
        c.borrow_mut().push(9);
        println!("{:?}", c.borrow());
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| { match c.borrow().first() { Some(_) => { c.borrow_mut().push(1); } None => {} } }));
        println!("conflict panicked: {}", r.is_err());
        let v = { let g = c.borrow(); g.iter().sum::<usize>() };
        c.borrow_mut().clear();
        println!("{} {:?}", v, c.borrow());
    }
}

fn main() {
    println!("== e01");
    e01::main();
    println!("== e02");
    e02::main();
}
