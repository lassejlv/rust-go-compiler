use std::fmt;

#[derive(Debug)]
enum StepError {
    TooBig { step: u32, value: u64 },
}

impl fmt::Display for StepError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            StepError::TooBig { step, value } => write!(f, "step {} produced {}", step, value),
        }
    }
}

impl std::error::Error for StepError {}

struct Guard(&'static str);
impl Drop for Guard {
    fn drop(&mut self) {
        println!("guard {} dropped", self.0);
    }
}

fn run(limit: u64) -> Result<u64, StepError> {
    let mut v = 1u64;
    for step in 1..=20 {
        v = v * 3 + 1;
        println!("step {} -> {}", step, v);
        if v > limit {
            return Err(StepError::TooBig { step, value: v });
        }
    }
    Ok(v)
}

fn main() -> Result<(), StepError> {
    let _g = Guard("main");
    let ok = run(1_000_000_000_000);
    println!("first run ok: {}", ok.is_ok());
    let v = run(500)?;
    println!("unreachable {}", v);
    Ok(())
}
