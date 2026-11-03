//~ ERROR E0308
struct Cfg { host: String, port: u16 }
fn main() {
    let c = Cfg { host: "localhost", port: 80 };
    let _ = (c.host, c.port);
}
