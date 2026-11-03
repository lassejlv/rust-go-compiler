//~ ERROR E0507
struct Job { name: String, f: Box<dyn Fn()> }
fn take(j: &Job) -> Box<dyn Fn()> { j.f }
fn main() { let j = Job { name: "x".into(), f: Box::new(|| {}) }; take(&j); let _ = j.name; }
