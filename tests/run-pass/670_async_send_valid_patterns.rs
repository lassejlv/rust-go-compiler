use std::rc::Rc; use std::future::Future; use std::sync::{Arc, Mutex};
fn spawn<F: Future + Send + 'static>(_f: F) {}
async fn tick() {}
fn main() {
    spawn(async { { let r = Rc::new(1); println!("{}", r); } tick().await; });
    let m = Arc::new(Mutex::new(0));
    spawn(async move { let v = { let g = m.lock().unwrap(); *g }; tick().await; println!("{}", v); });
    spawn(async { let a = Arc::new(5); tick().await; println!("{}", a); });
    println!("ok");
}
