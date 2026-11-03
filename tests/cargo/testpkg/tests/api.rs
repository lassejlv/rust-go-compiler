use testpkg::{doubled, fib};

#[test]
fn public_api() {
    assert_eq!(doubled(fib(5) as i32), 10);
}

#[test]
fn cwd_is_package_root() {
    assert!(std::path::Path::new("Cargo.toml").exists());
    assert_eq!(env!("CARGO_PKG_NAME"), "testpkg");
}

mod common {
    pub fn setup() -> u32 { 7 }
}

#[test]
fn uses_module() { assert_eq!(common::setup(), 7); }
