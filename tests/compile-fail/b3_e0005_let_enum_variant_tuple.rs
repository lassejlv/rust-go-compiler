//~ ERROR E0005
enum Token { Num(i32), Op(char) }
fn main() {
    let t = Token::Num(1);
    let Token::Num(n) = t;
    println!("{}", n);
}
