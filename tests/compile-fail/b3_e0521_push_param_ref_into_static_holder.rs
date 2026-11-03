//~ ERROR E0521
fn register(name: &str, reg: &mut Vec<&'static str>) {
    reg.push(name);
}
fn main() { let mut r = vec![]; register("a", &mut r); }
