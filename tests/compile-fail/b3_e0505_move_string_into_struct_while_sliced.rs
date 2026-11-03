//~ ERROR E0505
struct Owner { s: String }
fn main() {
    let s = String::from("hello world");
    let word = &s[..5];
    let o = Owner { s };
    println!("{} {}", word, o.s);
}
