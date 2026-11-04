#[derive(Debug)] struct S { x: i32, y: i32 }
fn f(a: i32, b: i32) -> (i32, i32) { (a, b) }
fn inc(x: &mut i32) -> i32 { *x += 1; *x }
fn main() {
    let mut x = 1; println!("tuple {:?}", (x, { x += 1; x }));
    let mut x = 1; println!("tuple-call {:?}", (x, inc(&mut x)));
    let mut x = 1; println!("struct {:?}", S { x: x, y: inc(&mut x) });
    let mut x = 1; println!("array {:?}", [x, inc(&mut x)]);
    let mut x = 1; println!("fncall {:?}", f(x, inc(&mut x)));
    let mut x = 1; println!("binop {}", x + inc(&mut x));
    let mut x = 1; println!("binop-block {}", x * 10 + { x += 1; x });
    let mut x = 1; println!("method {:?}", x.max({ x += 5; x }));
    let mut x = 1; let y = (x, { x = 7; x }); println!("assign {:?} {}", y, x);
    let mut s = String::from("a"); println!("string {:?}", (s.len(), { s.push('b'); s.len() }));
    let mut v = 3u8; println!("u8 {:?}", (v, { v = 4; v }, v));
}
