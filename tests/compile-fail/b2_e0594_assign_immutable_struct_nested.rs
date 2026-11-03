//~ ERROR E0594
struct In { v: i32 }
struct Out { i: In }
fn main() {
    let o = Out { i: In { v: 1 } };
    o.i.v = 2;
    println!("{}", o.i.v);
}
