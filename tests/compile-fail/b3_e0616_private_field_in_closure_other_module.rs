//~ ERROR E0616
mod m { pub struct S { secret: i32 } pub fn mk() -> S { S { secret: 1 } } }
fn main() {
    let v = vec![m::mk()];
    let total: i32 = v.iter().map(|s| s.secret).sum();
    println!("{}", total);
}
