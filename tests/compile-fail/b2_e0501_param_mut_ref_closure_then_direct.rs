//~ ERROR E0501
fn inside(x: &mut i32) { *x += 1; }
fn outside(x: &mut i32) { *x += 2; }
fn foo(a: &mut i32) {
    let mut bar = || inside(a);
    outside(a);
    bar();
}
fn main() { let mut n = 0; foo(&mut n); }
