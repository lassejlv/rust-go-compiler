use std::sync::atomic::{AtomicUsize, Ordering};
static INIT: AtomicUsize = AtomicUsize::new(0);
extern "C" fn init() { INIT.fetch_add(1, Ordering::SeqCst); }
#[used]
#[link_section = ".init_array"]
static CTOR: extern "C" fn() = init;
fn main() { println!("{}", INIT.load(Ordering::SeqCst)); }
