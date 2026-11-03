//~ ERROR notsend
use std::sync::Mutex;
use std::future::Future;
fn spawn<F: Future + Send + 'static>(_f: F) {}
async fn io() {}
static M: Mutex<i32> = Mutex::new(0);
fn main() {
    spawn(async {
        let g = M.lock().unwrap();
        io().await;
        println!("{}", *g);
    });
}
