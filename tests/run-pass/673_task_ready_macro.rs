use std::task::{ready, Poll};
fn add1(p: Poll<i32>) -> Poll<i32> {
    let v = ready!(p);
    Poll::Ready(v + 1)
}
fn main() {
    println!("{:?} {:?}", add1(Poll::Ready(1)), add1(Poll::Pending));
    let q: Poll<u8> = (|| { let x = std::task::ready!(Poll::Ready(7u8)); Poll::Ready(x) })();
    println!("{:?}", q);
}
