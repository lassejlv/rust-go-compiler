//~ ERROR E0382
struct Token(String);

impl Token {
    fn into_inner(self) -> String {
        self.0
    }
}

fn main() {
    let t = Token(String::from("tok"));
    let a = t.into_inner();
    let b = t.into_inner();
    println!("{} {}", a, b);
}
