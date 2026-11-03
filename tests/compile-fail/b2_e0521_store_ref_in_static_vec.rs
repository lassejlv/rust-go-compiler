//~ ERROR E0521
use std::sync::Mutex;
static LOG: Mutex<Vec<&'static str>> = Mutex::new(Vec::new());
fn log(msg: &str) {
    LOG.lock().unwrap().push(msg);
}
fn main() { log("a"); }
