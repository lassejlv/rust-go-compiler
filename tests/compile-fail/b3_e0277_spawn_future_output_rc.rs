//~ ERROR E0277
use std::rc::Rc;
use std::future::Future;
fn spawn<F: Future + Send + 'static>(_f: F) where F::Output: Send {}
async fn make() -> Rc<i32> { Rc::new(1) }
fn main() { spawn(make()); }
