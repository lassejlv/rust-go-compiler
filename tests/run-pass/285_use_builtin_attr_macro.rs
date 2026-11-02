// Builtin attribute and bang macros can be imported (libc's prelude!()).
mod p {
    #[allow(unused_imports)]
    pub(crate) use core::prelude::v1::derive;
    #[allow(unused_imports)]
    pub(crate) use core::{assert, cfg};
}
#[allow(unused_imports)]
use p::*;
#[derive(Debug)]
struct S;
fn main() {
    assert!(cfg!(unix));
    println!("{:?}", S);
}
