// Valid programs that a naive borrow checker / front end might reject.
// Each module is an independent program; main runs them in order.

mod p23_privacy_paths {
    mod outer {
        pub(crate) fn crate_vis() -> u8 { 1 }
        fn private_parent() -> u8 { 2 }
        pub struct Thing { pub(crate) a: u8, pub(super) b: u8, c: u8 }
        impl Thing { pub fn new() -> Thing { Thing { a: 1, b: 2, c: 3 } } pub fn c(&self) -> u8 { self.c } }
        pub mod inner {
            use super::*;
            pub(in crate::p23_privacy_paths::outer) fn only_outer() -> u8 { private_parent() + 10 }
            pub(super) fn to_parent() -> u8 { 20 }
            pub fn touch(t: &mut Thing) { t.c += 1; }
            pub mod deep {
                pub(in crate::p23_privacy_paths::outer) fn deep_fn() -> u8 { super::super::private_parent() * 100 }
                pub fn public_deep() -> u8 { crate::p23_privacy_paths::outer::private_parent() }
            }
        }
        pub use self::inner::deep::public_deep as reexported;
        pub fn sum() -> u8 { inner::only_outer() + inner::to_parent() + inner::deep::deep_fn() }
    }
    mod sibling {
        pub fn read() -> u8 { let t = crate::p23_privacy_paths::outer::Thing::new(); t.a + crate::p23_privacy_paths::outer::crate_vis() }
    }
    use outer::reexported;
    pub fn main() {
        let mut t = outer::Thing::new();
        outer::inner::touch(&mut t);
        println!("{} {} {} {} {} {}", outer::sum(), sibling::read(), reexported(), t.a, t.b, t.c());
    }
}

mod p25_macro_crate_private {
    mod util {
        pub(crate) fn helper(x: i32) -> i32 { x * 3 }
        pub(crate) struct Counter { pub(crate) n: i32 }
        macro_rules! triple { ($e:expr) => { $crate::p25_macro_crate_private::util::helper($e) }; }
        macro_rules! make_counter { ($n:expr) => { $crate::p25_macro_crate_private::util::Counter { n: $n } }; }
        pub(crate) use make_counter;
        pub(crate) use triple;
    }
    fn secret() -> &'static str { "secret" }
    macro_rules! sq_sum {
        ($($x:expr),*) => {{ let mut s = 0; $( s += $x * $x; )* s }};
    }
    macro_rules! reveal { () => { $crate::p25_macro_crate_private::secret() }; }
    macro_rules! gen_fn { ($name:ident, $v:expr) => { fn $name() -> i32 { $v } }; }
    gen_fn!(seven, 7);
    mod user {
        pub fn go() -> i32 { let c = crate::p25_macro_crate_private::util::make_counter!(4); crate::p25_macro_crate_private::util::triple!(c.n) }
    }
    pub fn main() {
        let s = 10;
        let r = sq_sum!(1, 2, s);
        println!("{} {} {} {} {}", util::triple!(2), r, reveal!(), seven(), user::go());
    }
}

mod p49_nested_modules_use_paths {
    mod net {
        pub mod addr {
            #[derive(Debug, Clone, Copy, PartialEq)]
            pub struct Ip(pub [u8; 4]);
            impl Ip { pub const LOCAL: Ip = Ip([127, 0, 0, 1]); pub(crate) fn is_local(&self) -> bool { *self == Self::LOCAL } }
        }
        pub mod conn {
            use super::addr::Ip;
            use crate::p49_nested_modules_use_paths::net::Proto::{self, *};
            pub struct Conn { pub ip: Ip, proto: Proto }
            impl Conn {
                pub fn new(ip: Ip) -> Self { Conn { ip, proto: if ip.is_local() { Udp } else { Tcp } } }
                pub fn proto_name(&self) -> &'static str { match self.proto { Tcp => "tcp", Udp => "udp" } }
            }
            pub(super) fn count() -> usize { 2 }
        }
        #[derive(Clone, Copy)]
        pub enum Proto { Tcp, Udp }
        pub use self::conn::Conn;
        pub fn n() -> usize { conn::count() }
    }
    use net::{addr::Ip, Conn};
    use self::net as network;
    pub fn main() {
        let a = Conn::new(Ip::LOCAL);
        let b = network::conn::Conn::new(Ip([10, 0, 0, 1]));
        println!("{} {} {:?} {}", a.proto_name(), b.proto_name(), b.ip, net::n());
    }
}

fn main() {
    p23_privacy_paths::main();
    p25_macro_crate_private::main();
    p49_nested_modules_use_paths::main();
}
