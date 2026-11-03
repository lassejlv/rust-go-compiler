//~ ERROR E0521
struct Worker { id: u32 }
impl Worker {
    fn start(&self) {
        std::thread::spawn(move || println!("{}", self.id));
    }
}
fn main() { Worker { id: 1 }.start(); }
