#[derive(Clone, Copy)] enum N { A, B }
struct S { id: u16, n: N }
const X: S = S { id: N::B as _, n: N::B };
fn main(){ let y: u16 = N::A as _; println!("{} {}", X.id, y); let _ = X.n; main2(); }
fn main2(){ let v = vec![1u8]; let p = &v as *const Vec<u8>; let q: *const u8 = v.as_ptr() as _; let f = main as fn(); let r: &dyn std::fmt::Debug = &3 as &_; println!("{:?} {:?} {:?}", p.is_null(), q.is_null(), r); let _ = f; }
