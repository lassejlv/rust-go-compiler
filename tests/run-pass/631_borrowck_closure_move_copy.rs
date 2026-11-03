// move closures copying Copy values, cloning before move, and reuse after.
use std::thread;

fn main() {
    let n = 5;
    let add = move |x: i32| x + n;
    println!("{} {}", add(1), n);
    let names = vec![String::from("a"), String::from("b")];
    let names2 = names.clone();
    let h = thread::spawn(move || names2.len());
    println!("{} {:?}", h.join().unwrap(), names);
    let data = vec![1, 2, 3];
    let r = &data;
    let total = thread::scope(|s| {
        let t = s.spawn(|| r.iter().sum::<i32>());
        let u = s.spawn(|| data.len() as i32);
        t.join().unwrap() + u.join().unwrap()
    });
    println!("{} {:?}", total, data);
}
