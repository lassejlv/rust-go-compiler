use std::process;

fn validate(values: &[i32]) -> Result<i32, String> {
    let mut sum = 0;
    for &v in values {
        if v < 0 {
            return Err(format!("negative value {}", v));
        }
        sum += v;
    }
    Ok(sum)
}

struct Cleanup;

impl Drop for Cleanup {
    fn drop(&mut self) {
        println!("cleanup ran (should NOT print, exit skips destructors)");
    }
}

fn main() {
    let _c = Cleanup;
    println!("starting");
    match validate(&[1, 2, 3]) {
        Ok(s) => println!("first batch ok: {}", s),
        Err(e) => println!("error: {}", e),
    }
    match validate(&[4, -5, 6]) {
        Ok(s) => println!("second batch ok: {}", s),
        Err(e) => {
            println!("fatal: {}", e);
            process::exit(3);
        }
    }
    println!("unreachable");
}
