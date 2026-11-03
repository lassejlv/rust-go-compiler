//~ ERROR E0499
fn main() {
    let mut log = Vec::new();
    let mut add = |s: &str| log.push(s.to_string());
    log.push(String::from("direct"));
    add("closure");
}
