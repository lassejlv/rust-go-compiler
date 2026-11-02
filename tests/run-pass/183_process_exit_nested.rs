use std::process;

struct Guard(&'static str);
impl Drop for Guard {
    fn drop(&mut self) {
        // Never printed: process::exit does not run destructors.
        println!("drop {}", self.0);
    }
}

fn validate(depth: u32, values: &[i32]) -> i32 {
    let _g = Guard("validate");
    if depth == 0 {
        let total: i32 = values.iter().sum();
        println!("depth 0 total {}", total);
        if total > 10 {
            println!("total too large, exiting with {}", total % 256);
            process::exit(total % 256);
        }
        return total;
    }
    println!("descend {}", depth);
    let r = validate(depth - 1, values);
    println!("unwind {}", depth);
    r
}

fn main() {
    let _g = Guard("main");
    let argc = std::env::args().count();
    println!("argc = {}", argc);
    println!("args().skip(1) empty: {}", std::env::args().skip(1).next().is_none());
    let ok = validate(2, &[1, 2, 3]);
    println!("first ok {}", ok);
    let closure = |v: &[i32]| validate(3, v);
    let results: Vec<i32> = vec![vec![4], vec![5, 6, 7], vec![1]].iter().map(|v| closure(v)).collect();
    println!("never printed {:?}", results);
}
