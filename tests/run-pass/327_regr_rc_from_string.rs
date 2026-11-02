use std::rc::Rc;
fn mk<T: From<String>>(s: String) -> T { T::from(s) }
fn main() {
    let r: Rc<str> = mk(String::from("own")); println!("{}", r);
}
