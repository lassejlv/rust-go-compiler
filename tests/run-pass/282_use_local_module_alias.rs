// Single-segment imports of local items (2018 uniform paths), renamed.
mod generic {
    pub struct Group;
    impl Group {
        pub const X: i32 = 7;
    }
}
use generic as imp;
use self::imp::Group;

mod outer {
    pub(crate) mod b {
        pub fn f() -> u32 {
            3
        }
    }
    pub(crate) use b as imp;
}

pub trait FromZeros {
    fn z() -> i32 {
        5
    }
}
mod r {
    pub struct Ref;
    impl Ref {
        pub fn v(&self) -> i32 {
            4
        }
    }
}
pub use r::*;
#[allow(unused_imports)]
use {FromZeros as FromZeroes, Ref as LayoutVerified};
struct S;
impl FromZeroes for S {}

fn main() {
    println!("{} {}", Group::X, outer::imp::f());
    println!("{} {}", S::z(), LayoutVerified.v());
}
