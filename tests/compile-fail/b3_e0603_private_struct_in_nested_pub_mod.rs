//~ ERROR E0603
mod net {
    pub mod http {
        struct Request;
        pub fn make() {}
    }
}
fn main() { let _r = net::http::Request; net::http::make(); }
