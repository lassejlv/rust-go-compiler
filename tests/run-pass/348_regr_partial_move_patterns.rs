// Regression: residual drops of partially moved patterns and struct update.
struct D(&'static str);
impl Drop for D {
    fn drop(&mut self) { println!("  drop {}", self.0); }
}
struct S { a: D, b: D, c: D }
struct P(D, D);
fn mk2(x: &'static str, y: &'static str) -> (D, D) { (D(x), D(y)) }

fn main() {
    println!("guard fails then second arm");
    {
        let t = (D("g0"), D("g1"));
        let flag = false;
        match t {
            (a, _) if flag => println!("  first {}", a.0),
            (_, b) => println!("  second {}", b.0),
        }
        println!("  after match");
    }
    println!("temporary scrutinee");
    match mk2("x0", "x1") {
        (a, _) => println!("  got {}", a.0),
    }
    println!("if let partial");
    {
        let o = Some(P(D("o0"), D("o1")));
        if let Some(P(_, y)) = o {
            println!("  y {}", y.0);
        }
    }
    println!("pattern on partially moved local");
    {
        let s = S { a: D("pa"), b: D("pb"), c: D("pc") };
        let first = s.a;
        let S { b, .. } = s;
        println!("  moved {} {}", first.0, b.0);
    }
    println!("struct update keeps given fields");
    {
        let s1 = S { a: D("ua"), b: D("ub"), c: D("uc") };
        let s2 = S { b: D("nb"), ..s1 };
        println!("  s2 {} {} {} / s1.b {}", s2.a.0, s2.b.0, s2.c.0, s1.b.0);
    }
    println!("assigned field then destructure");
    {
        let mut s = S { a: D("aa"), b: D("ab"), c: D("ac") };
        s.c = D("ac2");
        let S { a, .. } = s;
        println!("  got {}", a.0);
    }
    println!("remainder usable after match");
    {
        let t = (D("r0"), D("r1"), String::from("r2"));
        match t {
            (a, _, _) => println!("  took {}", a.0),
        }
        println!("  still have {} {}", t.1 .0, t.2);
        let (_, b, _) = t;
        println!("  then {}", b.0);
    }
    println!("end");
}
