//~ ERROR E0521
fn spawn<F: FnOnce() + Send + 'static>(f: F) { f() }
fn run(data: &Vec<u8>) {
    spawn(move || println!("{}", data.len()));
}
fn main() { run(&vec![1]); }
