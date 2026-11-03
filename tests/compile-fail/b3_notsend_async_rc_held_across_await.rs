//~ ERROR notsend
use std::rc::Rc;
use std::future::Future;
fn spawn<F: Future + Send + 'static>(_f: F) {}
async fn tick() {}
fn main() {
    spawn(async {
        let r = Rc::new(1);
        tick().await;
        println!("{}", r);
    });
}
