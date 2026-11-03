//~ ERROR E0027
struct Inner { a: i32, b: i32 }
struct Outer { inner: Inner, c: i32 }
fn main() {
    let o = Outer { inner: Inner { a: 1, b: 2 }, c: 3 };
    let Outer { inner: Inner { a }, c } = o;
    println!("{} {}", a, c);
}
