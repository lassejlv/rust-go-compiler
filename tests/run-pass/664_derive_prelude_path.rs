#[::core::prelude::v1::derive(::core::clone::Clone, ::core::marker::Copy, ::core::fmt::Debug)]
struct P { a: i32 }
#[derive(core::clone::Clone, std::marker::Copy)]
struct Q(u8);
fn need<T: Copy>(_: T) {}
fn main(){ let p = P { a: 1 }; need(p); need(Q(1)); println!("{:?}", p); }
