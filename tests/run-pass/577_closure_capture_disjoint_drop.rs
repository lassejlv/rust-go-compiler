// edition-2021 disjoint field capture in closures and its effect on moves and drop order
struct Noisy(&'static str);
impl Drop for Noisy {
    fn drop(&mut self) {
        println!("  drop {}", self.0);
    }
}

struct Pair {
    a: Noisy,
    b: Noisy,
}

struct Deep {
    inner: Pair,
    tag: String,
    n: i32,
}

fn part1() {
    println!("part1: move closure captures only p.a");
    let p = Pair { a: Noisy("p.a"), b: Noisy("p.b") };
    let c = move || {
        let _x = &p.a;
        println!("  closure sees {}", p.a.0);
    };
    c();
    println!("  dropping closure");
    drop(c);
    println!("  p.b still usable: {}", p.b.0);
    println!("  end of part1");
}

fn part2() {
    println!("part2: closure consumes one field");
    let p = Pair { a: Noisy("q.a"), b: Noisy("q.b") };
    let c = || {
        let taken = p.b;
        println!("  took {}", taken.0);
    };
    println!("  before call");
    c();
    println!("  after call, q.a = {}", p.a.0);
}

fn part3() {
    println!("part3: nested field paths");
    let mut d = Deep { inner: Pair { a: Noisy("d.inner.a"), b: Noisy("d.inner.b") }, tag: String::from("t"), n: 1 };
    let mut bump = || d.n += 10;
    let tag_ref = &d.tag;
    bump();
    println!("  tag {} n {}", tag_ref, d.n);
    let mv = move || println!("  moved-in {}", d.inner.a.0);
    println!("  d.inner.b still here: {}", d.inner.b.0);
    mv();
    drop(mv);
    println!("  after dropping mv");
    d.tag.push('!');
    println!("  {}", d.tag);
}

fn part4() {
    println!("part4: closures in a loop with move");
    let items = vec![Noisy("i0"), Noisy("i1"), Noisy("i2")];
    let mut fs: Vec<Box<dyn Fn() -> &'static str>> = vec![];
    for it in items {
        fs.push(Box::new(move || it.0));
    }
    println!("  built {} closures", fs.len());
    let names: Vec<&str> = fs.iter().map(|f| f()).collect();
    println!("  {:?}", names);
    fs.remove(1);
    println!("  removed one");
    fs.reverse();
    drop(fs);
    println!("  end of part4");
}

fn part5() {
    println!("part5: FnOnce moving captured whole struct vs field");
    let p = Pair { a: Noisy("r.a"), b: Noisy("r.b") };
    let once = move || p;
    let back = once();
    println!("  got back {} {}", back.a.0, back.b.0);
    let Pair { a, b: _ } = back;
    println!("  kept {}", a.0);
}

fn part6() {
    println!("part6: tuple field capture");
    let t = (Noisy("t.0"), Noisy("t.1"), 5);
    let c = move || {
        let n = t.2;
        println!("  t.2 = {}", n);
        let _r = &t.1;
    };
    c();
    drop(c);
    println!("  t.0 still: {}", t.0 .0);
}

fn main() {
    part1();
    part2();
    part3();
    part4();
    part5();
    part6();
    println!("main end");
}
