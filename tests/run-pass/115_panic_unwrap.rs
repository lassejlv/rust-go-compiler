fn find_user(id: u32) -> Option<&'static str> {
    match id {
        1 => Some("alice"),
        2 => Some("bob"),
        _ => None,
    }
}

struct Announce(&'static str);

impl Drop for Announce {
    fn drop(&mut self) {
        println!("dropping {} during unwind", self.0);
    }
}

fn main() {
    let _guard = Announce("guard");
    for id in 1..=3 {
        println!("looking up {}", id);
        let name = find_user(id).unwrap();
        println!("found {}", name);
    }
    println!("never printed");
}
