struct Announce(&'static str);
impl Drop for Announce { fn drop(&mut self) { println!("dropping {} during unwind", self.0); } }
fn find(id: i32) -> Option<i32> { if id < 3 { Some(id) } else { None } }
fn main() {
    let _guard = Announce("guard");
    for id in 1..4 {
        let _g2 = Announce("inner");
        println!("looking up {}", id);
        let name = find(id).unwrap();
        println!("found {}", name);
    }
}
