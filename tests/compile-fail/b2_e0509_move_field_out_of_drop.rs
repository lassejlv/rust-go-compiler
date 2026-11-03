//~ ERROR E0509
struct Conn { addr: String }
impl Drop for Conn { fn drop(&mut self) {} }
fn addr(c: Conn) -> String { c.addr }
fn main() { println!("{}", addr(Conn { addr: "a".into() })); }
