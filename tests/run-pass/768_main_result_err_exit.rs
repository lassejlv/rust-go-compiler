// main returning Result<(), Box<dyn Error>> that fails after producing output:
// the process must exit with status 1 (the error is reported on stderr).
// Also exercises Termination-related behavior of `?` in main and drops
// running before exit.
use std::error::Error;
use std::fmt;

#[derive(Debug)]
struct Fatal { stage: &'static str, code: i32 }
impl fmt::Display for Fatal {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "fatal in {} (code {})", self.stage, self.code) }
}
impl Error for Fatal {}

struct Cleanup(&'static str);
impl Drop for Cleanup { fn drop(&mut self) { println!("cleanup {}", self.0); } }

fn stage(n: u32) -> Result<u32, Fatal> {
    println!("stage {}", n);
    if n == 3 { Err(Fatal { stage: "three", code: 42 }) } else { Ok(n * 10) }
}

fn pipeline() -> Result<u32, Box<dyn Error>> {
    let _c = Cleanup("pipeline");
    let mut acc = 0;
    for i in 1..=5 {
        acc += stage(i)?;
        println!("acc {}", acc);
    }
    Ok(acc)
}

fn main() -> Result<(), Box<dyn Error>> {
    let _outer = Cleanup("main");
    let parsed: i32 = "17".parse()?;
    println!("parsed {}", parsed);
    let r = pipeline();
    match &r {
        Ok(v) => println!("pipeline ok {}", v),
        Err(e) => println!("pipeline failed: {} / is Fatal: {}", e, e.downcast_ref::<Fatal>().is_some()),
    }
    let _later = Cleanup("later");
    r?;
    println!("unreachable");
    Ok(())
}
