fn min_max(a: i32, b: i32, c: i32) -> (i32, i32) {
    let mut lo = a;
    let mut hi = a;
    if b < lo { lo = b; }
    if c < lo { lo = c; }
    if b > hi { hi = b; }
    if c > hi { hi = c; }
    (lo, hi)
}

fn divmod(a: i64, b: i64) -> (i64, i64) {
    (a / b, a % b)
}

fn main() {
    let t = (1, 2.5, 'c', "str", true);
    println!("{} {} {} {} {}", t.0, t.1, t.2, t.3, t.4);
    let (lo, hi) = min_max(7, -3, 12);
    println!("lo = {}, hi = {}", lo, hi);
    let (q, r) = divmod(47, 5);
    println!("47 = 5 * {} + {}", q, r);
    let mut pair = (10, 20);
    pair.0 += 5;
    pair.1 *= 2;
    println!("pair = ({}, {})", pair.0, pair.1);
    let nested = ((1, 2), (3, (4, 5)));
    println!("nested = {}", (nested.1).1 .1);
    let ((a, b), (c, (d, e))) = nested;
    println!("sum = {}", a + b + c + d + e);
    let (x, _, z) = (100, 200, 300);
    println!("x = {}, z = {}", x, z);
    let unit = ();
    println!("unit debug = {:?}", unit);
    println!("tuple debug = {:?}", (1, "two", 3.0));
    let single = (5,);
    println!("single = {:?}", single);
    let p1 = (1, 2);
    let p2 = (1, 3);
    println!("eq = {}, lt = {}", p1 == p2, p1 < p2);
    let (mut m, n) = (1, 2);
    m += n;
    println!("m = {}", m);
}
