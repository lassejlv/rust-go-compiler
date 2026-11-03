//~ ERROR E0499
use std::collections::HashMap;
struct Cache { map: HashMap<u32, String> }
impl Cache {
    fn get(&mut self, k: u32) -> &mut String {
        if let Some(v) = self.map.get_mut(&k) { return v; }
        self.map.insert(k, String::new());
        self.map.get_mut(&k).unwrap()
    }
}
fn main() {
    let mut c = Cache { map: HashMap::new() };
    let a = c.get(1);
    let b = c.get(2);
    a.push_str(b);
}
