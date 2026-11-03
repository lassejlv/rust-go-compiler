struct D(&'static str);
impl Drop for D { fn drop(&mut self) { println!("  drop {}", self.0); } }
struct Pair { a: D, b: D, c: D }
fn consume(d: D) { println!("  consume {}", d.0); }
fn maybe(flag: bool) {
    println!("maybe({})", flag);
    let x = D("x");
    let y = D("y");
    if flag { consume(x); } else { let _keep = y; println!("  kept y in branch"); }
    println!("  end of maybe");
}
fn loop_moves(n: usize) {
    println!("loop_moves({})", n);
    let mut slot = Some(D("slot0"));
    let names = ["s1", "s2", "s3"];
    for i in 0..n {
        if i % 2 == 0 { if let Some(d) = slot.take() { consume(d); } }
        else { slot = Some(D(names[i % 3])); }
    }
    println!("  loop done");
}
fn partial(flag: bool) {
    println!("partial({})", flag);
    let p = Pair { a: D("pa"), b: D("pb"), c: D("pc") };
    if flag { consume(p.b); }
    let Pair { a, .. } = p;
    println!("  got a={}", a.0);
}
fn match_move(v: Option<D>, guard: bool) {
    println!("match_move");
    match v {
        Some(d) if guard => consume(d),
        Some(ref d) => println!("  ref {}", d.0),
        None => println!("  none"),
    }
    println!("  after match");
}
fn reassign() {
    println!("reassign");
    let mut a = D("a1");
    a = D("a2");
    let b = a;
    a = D("a3");
    println!("  b={} a={}", b.0, a.0);
}
fn ret_early(k: u8) -> Option<D> {
    println!("ret_early({})", k);
    let t1 = D("t1");
    let t2 = D("t2");
    if k == 0 { return Some(t1); }
    if k == 1 { return None; }
    drop(t1);
    Some(t2)
}
fn temporaries() {
    println!("temporaries");
    let n = { let _t = D("blocktemp"); D("blockval").0.len() };
    println!("  n={}", n);
    let len = D("expr_temp").0.len() + D("expr_temp2").0.len();
    println!("  len={}", len);
    match D("scrutinee").0 { s => println!("  in match {}", s) }
    if let "iflet" = D("iflet").0 { println!("  in if let"); } else { println!("  else"); }
    let _ = D("underscore");
    let _u = D("named_underscore");
    println!("  end temporaries");
}
fn closures() {
    println!("closures");
    let a = D("cap_move");
    let b = D("cap_ref");
    let f = move || println!("  calling with {}", a.0);
    let g = || println!("  ref {}", b.0);
    f(); g();
    let h = f;
    h();
    println!("  closures end");
}
fn vec_ops() {
    println!("vec_ops");
    let mut v = vec![D("v0"), D("v1"), D("v2"), D("v3")];
    let x = v.remove(1);
    v.truncate(2);
    println!("  truncated");
    v.insert(0, D("v_new"));
    v[1] = D("v_replaced");
    let _ = v.pop();
    println!("  popped, x={}", x.0);
    std::mem::drop(v);
    println!("  vec dropped");
}
fn main() {
    maybe(true); maybe(false);
    loop_moves(4); loop_moves(3);
    partial(true); partial(false);
    match_move(Some(D("m1")), true); match_move(Some(D("m2")), false); match_move(None, true);
    reassign();
    for k in 0..3 { let r = ret_early(k); println!("  got {:?}", r.as_ref().map(|d| d.0)); }
    temporaries();
    closures();
    vec_ops();
    let boxed: Box<dyn Fn()> = { let d = D("boxed_cap"); Box::new(move || println!("  boxed {}", d.0)) };
    boxed();
    drop(boxed);
    let arr = [D("arr0"), D("arr1"), D("arr2")];
    let [first, _, third] = arr;
    println!("main end {} {}", first.0, third.0);
}
