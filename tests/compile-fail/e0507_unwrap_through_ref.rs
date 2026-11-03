//~ ERROR E0507
struct Cache {
    value: Option<String>,
}

fn peek(c: &Cache) -> String {
    c.value.unwrap()
}

fn main() {
    let c = Cache { value: Some("v".into()) };
    println!("{}", peek(&c));
}
