//~ ERROR E0507
struct Conn { sock: Option<String> }
impl Conn {
    fn sock(&self) -> String { self.sock.unwrap() }
}
fn main() { let c = Conn { sock: None }; let _ = c.sock(); }
