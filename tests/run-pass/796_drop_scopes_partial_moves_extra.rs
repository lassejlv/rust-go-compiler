// Temporary lifetime extension (and its limits), moves out of fields of
// temporaries and boxes, enum payload remainders kept in their variable,
// and unwinding through partially moved values.
struct T(&'static str, i32);
impl Drop for T { fn drop(&mut self) { println!("drop {}", self.0); } }
impl T { fn get(&self) -> i32 { self.1 } fn me(&self) -> &T { self } }
fn t(n: &'static str) -> T { T(n, n.len() as i32) }
struct H<'a> { r: &'a T }
impl Drop for H<'_> { fn drop(&mut self) { println!("drop H({})", self.r.0); } }
trait Tr { fn n(&self) -> i32; }
impl Tr for T { fn n(&self) -> i32 { self.1 } }
struct P(T, T);

struct D(&'static str);
impl Drop for D { fn drop(&mut self) { println!("drop {}", self.0); } }
struct In { p: D, q: D }
struct S { x: D, y: D, inner: In }
fn consume(d: D) { println!("consume {}", d.0); }
fn mk(n: &'static str) -> Box<S> {
    Box::new(S { x: D(n), y: D("y"), inner: In { p: D("p"), q: D("q") } })
}
fn pair() -> (D, D) { (D("a"), D("b")) }
struct Two { x: D, y: D }
fn two() -> Two { Two { x: D("sx"), y: D("sy") } }
enum E { A(D, D, D), B { x: D, y: D }, C(D) }
fn pick(i: i32) -> E {
    match i {
        0 => E::A(D("a0"), D("a1"), D("a2")),
        1 => E::B { x: D("bx"), y: D("by") },
        _ => E::C(D("c")),
    }
}

fn extension() {
    let a = &t("field-of").1;
    println!("a={}", a);
    let h = H { r: &t("inH") };
    println!("h {}", h.r.0);
    let d: &dyn Tr = &t("dyn");
    println!("d={}", d.n());
    let m = &mut t("mut");
    m.1 += 1;
    println!("m={}", m.1);
    let x = match a { 8 => &t("arm8"), _ => &t("armother") };
    println!("x={}", x.0);
    let s = Some(&t("some"));
    println!("s={}", s.unwrap().0);
    let arr = [&t("a0"), &t("a1")];
    println!("arr={}", arr.len());
    let bx = &*Box::new(t("boxed"));
    println!("bx={}", bx.0);
    let v = &vec![t("v0"), t("v1")][1];
    println!("v={}", v.0);
    let pr = &P(t("p0"), t("p1")).1;
    println!("pr={}", pr.0);
    let me = t("recv").me().get();
    println!("me={}", me);
    let blk = &{ let z = t("inner"); z };
    println!("blk={}", blk.0);
    let nest = { let q = &t("q"); println!("q {}", q.0); &t("nest") };
    println!("nest={}", nest.0);
    for i in 0..2 {
        let r = if i == 0 { &t("loop0") } else { &t("loop1") };
        println!("r={}", r.0);
    }
    let Some(e) = &Some(t("else-ext")) else { return };
    println!("e={}", e.0);
    println!("end extension");
}

fn boxes() {
    {
        let bs = mk("a");
        let q = bs.inner.q;
        println!("moved {}", q.0);
    }
    println!("--");
    {
        let mut bs = mk("b");
        consume(bs.x);
        bs.x = D("newx");
        println!("reinit {}", bs.x.0);
    }
    println!("--");
    for i in 0..2 {
        let bs = mk(if i == 0 { "l0" } else { "l1" });
        if i == 1 {
            let y = (*bs).y;
            println!("took {}", y.0);
        }
        println!("iter {}", i);
    }
    println!("--");
    {
        let bt = Box::new((D("t0"), D("t1"), 5));
        let n = bt.2;
        let t0 = bt.0;
        println!("{} {}", n, t0.0);
    }
}

fn temp_fields() {
    let s = pair().0;
    println!("got {}", s.0);
    let y = two().y;
    println!("got {}", y.0);
    println!("end temp fields");
}

fn remainders() {
    for i in 0..3 {
        let e = pick(i);
        match e {
            E::A(_, m, _) if i > 5 => println!("never {}", m.0),
            E::A(_, m, _) => println!("A mid {}", m.0),
            E::B { y, .. } => println!("B y {}", y.0),
            E::C(c) => println!("C {}", c.0),
        }
        println!("end iter {}", i);
    }
    println!("--");
    {
        let mut e = pick(0);
        if let E::A(first, ..) = e {
            println!("first {}", first.0);
        }
        e = pick(1);
        println!("reassigned");
        if let E::B { x, .. } = e {
            println!("x {}", x.0);
        }
        println!("end block");
    }
    println!("--");
    {
        let mut o = Some((D("w0"), D("w1")));
        let mut n = 0;
        while let Some((a, _)) = o {
            println!("got {}", a.0);
            n += 1;
            o = if n < 2 { Some((D("w2"), D("w3"))) } else { None };
        }
        println!("after while");
    }
    println!("--");
    {
        let e = pick(1);
        let E::B { y, .. } = e else { return };
        println!("let-else y {}", y.0);
    }
}

fn main() {
    extension();
    println!("==");
    boxes();
    println!("==");
    temp_fields();
    println!("==");
    remainders();
    println!("==");
    // unwinding drops what is left of partially moved values
    let st = Two { x: D("ux"), y: D("uy") };
    let ux = st.x;
    let bs = mk("unwind");
    let bx = bs.x;
    let p = Some((D("p0"), D("p1")));
    if let Some((p0, _)) = p { println!("p {}", p0.0); }
    println!("{} {}", ux.0, bx.0);
    let v: Vec<i32> = Vec::new();
    let _ = v[std::hint::black_box(1)];
}
