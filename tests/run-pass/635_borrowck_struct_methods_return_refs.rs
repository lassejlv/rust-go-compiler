// get-style methods returning refs, used before a later &mut call.
struct Registry {
    names: Vec<String>,
}

impl Registry {
    fn get(&self, i: usize) -> Option<&String> {
        self.names.get(i)
    }
    fn get_or_add(&mut self, name: &str) -> &String {
        if let Some(i) = self.names.iter().position(|n| n == name) {
            return &self.names[i];
        }
        self.names.push(name.to_string());
        self.names.last().unwrap()
    }
    fn rename(&mut self, i: usize, to: &str) {
        self.names[i] = to.to_string();
    }
}

fn main() {
    let mut r = Registry { names: vec!["a".into()] };
    let len = r.get(0).map(|s| s.len()).unwrap_or(0);
    r.rename(0, "alpha");
    let b = r.get_or_add("beta").clone();
    let a = r.get_or_add("alpha").len();
    r.rename(1, &b.to_uppercase());
    println!("{} {} {} {:?}", len, b, a, r.names);
}
