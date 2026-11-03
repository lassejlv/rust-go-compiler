#[test]
fn shout_twice() {
    let s = testpkg::text::shout("a");
    assert_eq!(testpkg::text::shout(&s), "A!!");
}
