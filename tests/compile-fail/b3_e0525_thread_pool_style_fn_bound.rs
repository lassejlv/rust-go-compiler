//~ ERROR E0525
fn submit<F: Fn() + Send + 'static>(_f: F) {}
fn main() {
    let tx = std::sync::mpsc::channel::<i32>().0;
    let job = move || { let t = tx; t.send(1).unwrap(); };
    submit(job);
}
