// Edition-2021 temporary scope rules: tail-expression temporaries outlive
// block locals, if-let scrutinee temporaries live through the else block,
// lifetime extension for `let r = &temp`, temporaries in conditions, in
// method chains, in `while let`, and in function arguments.
use std::cell::RefCell;

thread_local! { static EV: RefCell<Vec<String>> = RefCell::new(vec![]); }
fn ev(s: &str) { EV.with(|e| e.borrow_mut().push(s.to_string())); }
fn dump(t: &str) { let v = EV.with(|e| std::mem::take(&mut *e.borrow_mut())); println!("{:<18}{}", t, v.join(" ")); }

struct T(&'static str);
impl Drop for T { fn drop(&mut self) { ev(&format!("~{}", self.0)); } }
impl T {
    fn n(&self) -> usize { ev(&format!("n({})", self.0)); self.0.len() }
    fn opt(&self) -> Option<usize> { Some(self.0.len()) }
    fn none(&self) -> Option<usize> { None }
    fn me(&self) -> &T { self }
}
fn mk(s: &'static str) -> T { ev(&format!("+{}", s)); T(s) }
fn take2(a: usize, b: &T) -> usize { ev("take2"); a + b.0.len() }

fn tail_block() -> usize {
    let _local = mk("local");
    mk("tail").n()
}

fn main() {
    let x = { let _l = mk("blk-local"); mk("blk-tail").n() };
    ev(&format!("x={}", x));
    dump("block tail:");

    let y = tail_block();
    ev(&format!("y={}", y));
    dump("fn tail:");

    if let Some(n) = mk("iflet").none() {
        ev(&format!("then {}", n));
    } else {
        ev("else-branch");
    }
    ev("after");
    dump("if-let else:");

    if mk("cond").n() > 2 {
        ev("in-if-body");
    }
    dump("if cond:");

    let r = &mk("extended");
    ev("after let r");
    let len = r.n();
    ev(&format!("len={}", len));
    drop(len);
    dump("let extension:");

    let r2 = mk("chain").me().n();
    ev(&format!("r2={}", r2));
    dump("method chain:");

    let mut count = 0;
    while let Some(k) = mk("wl").opt() {
        count += k;
        if count > 4 { break; }
        ev("loop-body");
    }
    dump("while let:");

    let z = take2(mk("arg1").n(), &mk("arg2"));
    ev(&format!("z={}", z));
    dump("fn args:");

    match mk("m").opt() {
        Some(v) if v > 10 => ev("big"),
        Some(_) => ev("small"),
        None => ev("none"),
    }
    dump("match:");

    let tup = (mk("t0").n(), mk("t1"), mk("t2").n());
    ev("tuple built");
    drop(tup);
    dump("tuple temps:");

    let s = format!("{}-{}", mk("f1").n(), mk("f2").n());
    ev(&s);
    dump("format args:");

    let v: Vec<usize> = vec![mk("v1").n(), mk("v2").n()];
    ev(&format!("{:?}", v));
    dump("vec! temps:");

    let b = mk("and1").n() > 100 && mk("and2").n() > 0;
    ev(&format!("b={}", b));
    dump("short circuit:");

    let _ = mk("underscore");
    ev("after underscore");
    let _keep = mk("keep");
    ev("after keep");
    drop(_keep);
    dump("let _ vs _keep:");

    let s = String::from("abc");
    let lens = [mk("arr0"), mk("arr1")].iter().map(|t| t.n()).sum::<usize>() + s.len();
    ev(&format!("lens={}", lens));
    dump("array temp iter:");
}
