// process::exit called from a spawned thread while main is blocked; destructors of the
// main thread do not run, stdout written so far is flushed.
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

struct Guard(&'static str);
impl Drop for Guard {
    fn drop(&mut self) {
        println!("drop {} (must not print)", self.0);
    }
}

fn main() {
    let _g = Guard("main guard");
    let (tx, rx) = mpsc::channel::<u32>();
    println!("main: spawning");
    let worker = thread::spawn(move || {
        let _w = Guard("worker guard");
        let mut sum = 0;
        for v in rx.iter() {
            sum += v;
            if sum > 10 {
                println!("worker: sum {} exceeded, exiting with {}", sum, sum % 256);
                std::process::exit((sum % 256) as i32);
            }
        }
        sum
    });
    for v in [3, 4, 5, 6] {
        tx.send(v).unwrap();
        thread::sleep(Duration::from_millis(2));
    }
    let r = worker.join();
    println!("unreachable {:?}", r.is_ok());
}
