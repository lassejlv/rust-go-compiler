//~ ERROR E0373
fn make_greeter(name: String) -> Box<dyn Fn() -> String> {
    Box::new(|| format!("hi {}", name))
}

fn main() {
    println!("{}", make_greeter("x".into())());
}
