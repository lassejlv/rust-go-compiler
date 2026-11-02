fn check(v: bool) -> bool {
    println!("check({})", v);
    v
}

fn main() {
    let x = 5;
    let x = x + 1;
    {
        let x = x * 2;
        println!("inner x = {}", x);
    }
    println!("outer x = {}", x);
    let s = "42";
    let s: i32 = s.parse().unwrap();
    println!("shadowed type: {}", s + 1);

    let t = true;
    let f = false;
    println!("t && f = {}", t && f);
    println!("t || f = {}", t || f);
    println!("!t = {}", !t);
    println!("t ^ f = {}", t ^ f);
    println!("t & t = {}", t & t);
    println!("f | f = {}", f | f);
    println!("t == f = {}", t == f);
    println!("t != f = {}", t != f);

    // short circuit
    let r = check(false) && check(true);
    println!("r = {}", r);
    let r2 = check(true) || check(false);
    println!("r2 = {}", r2);

    let a = 3;
    let b = 7;
    let in_range = a >= 1 && a <= 5 && !(b < 5);
    println!("in_range = {}", in_range);
    println!("bool as int: {} {}", true as i32, false as u8);
    println!("cmp: {} {} {} {}", a < b, a > b, a <= 3, b >= 8);
}
