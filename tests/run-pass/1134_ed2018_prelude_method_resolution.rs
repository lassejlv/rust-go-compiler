// edition:2018
// Edition-2018 semantics that changed in 2021:
//  * `array.into_iter()` yields references (resolves through &[T; N])
//  * TryInto/TryFrom/FromIterator are not in the prelude, so user traits
//    with those method names are picked without ambiguity
//  * single-argument panic! does not format its literal
//  * the `pat` macro fragment does not accept top-level `|`
use std::num::TryFromIntError;

trait TryInto<T> {
    fn try_into(self) -> Result<T, TryFromIntError>;
}
impl TryInto<u8> for i32 {
    fn try_into(self) -> Result<u8, TryFromIntError> {
        Ok(self as u8) // deliberately wrapping
    }
}

trait FromIterator {
    fn from_iter(v: Vec<u8>) -> Self;
}
#[derive(Debug)]
struct Bytes(usize);
impl FromIterator for Bytes {
    fn from_iter(v: Vec<u8>) -> Self { Bytes(v.len()) }
}

macro_rules! arms {
    ($a:pat | $b:pat) => { 2 };
    ($a:pat) => { 1 };
}

fn main() {
    let arr = [1u8, 2, 3];
    let sizes: Vec<usize> = arr.into_iter().map(|x| std::mem::size_of_val(&x)).collect();
    println!("element sizes via into_iter: {:?}", sizes);
    let total: u32 = arr.into_iter().map(|&x| x as u32).sum();
    println!("total {}", total);
    let max = arr.into_iter().max();
    println!("max {:?}", max);

    let wrapped: Result<u8, TryFromIntError> = 300i32.try_into();
    println!("user try_into(300) = {:?}", wrapped);
    let b = Bytes::from_iter(vec![1, 2, 3, 4]);
    println!("{:?}", b);

    std::panic::set_hook(Box::new(|_| {}));
    let p = std::panic::catch_unwind(|| panic!("{} is not formatted")).unwrap_err();
    println!("payload: {:?}", p.downcast_ref::<&str>());
    println!("arms: {} {} {}", arms!(1 | 2), arms!(_), arms!(Some(3)));
}
