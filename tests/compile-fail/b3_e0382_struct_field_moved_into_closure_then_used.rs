//~ ERROR E0382
struct App { name: String, port: u16 }
fn main() {
    let app = App { name: "x".into(), port: 1 };
    let f = move || println!("{}", app.name);
    f();
    println!("{}", app.name);
}
