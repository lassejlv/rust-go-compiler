fn main() {
    println!("{}", testpkg::text::shout(&testpkg::fib(7).to_string()));
}

#[test]
fn bin_test() {
    assert_eq!(testpkg::doubled(21), 42);
    assert_eq!(testpkg::get_fib10!(), 55);
}
