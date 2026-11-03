//~ ERROR E0026
struct User { name: String, age: u32 }
fn main() {
    let u = User { name: "a".into(), age: 1 };
    let User { nmae, .. } = u;
}
