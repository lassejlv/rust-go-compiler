pub mod helpers;
use helpers::twice;

pub fn double(x: i32) -> i32 { twice(x) }

#[macro_export]
macro_rules! triple {
    ($e:expr) => { $crate::helpers::times($e, 3) };
}
