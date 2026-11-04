//~ ERROR E0015
// `HashMap::new` is not const (unlike `BTreeMap::new`).
use std::collections::HashMap;
use std::sync::Mutex;
static M: Mutex<HashMap<u32, u32>> = Mutex::new(HashMap::new());
fn main() {
    M.lock().unwrap().insert(1, 2);
}
