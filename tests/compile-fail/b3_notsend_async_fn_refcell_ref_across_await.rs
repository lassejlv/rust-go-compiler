//~ ERROR notsend
use std::cell::RefCell;
use std::future::Future;
fn require_send<F: Future + Send>(_f: F) {}
async fn yield_now() {}
async fn work(c: &RefCell<i32>) {
    let g = c.borrow();
    yield_now().await;
    println!("{}", *g);
}
fn main() {
    let c = RefCell::new(1);
    require_send(work(&c));
}
