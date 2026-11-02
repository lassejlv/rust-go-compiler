// An import brings in every namespace its target has, even when the value
// comes from a re-export that resolves later than a same-named private module.
mod krate {
    pub use crate::krate::memchr::memchr;
    mod memchr {
        pub fn memchr() -> i32 {
            1
        }
    }
    pub mod memmem {
        pub fn g() -> i32 {
            2
        }
    }
    pub use self::inner::f as inner;
    mod inner {
        pub fn f() -> i32 {
            3
        }
    }
}
mod user {
    pub use crate::krate::inner;
    pub use crate::krate::memmem;
}
use krate::{memchr, memmem};
fn main() {
    println!("{} {}", memchr(), memmem::g());
    println!("{} {}", user::inner(), user::memmem::g());
}
