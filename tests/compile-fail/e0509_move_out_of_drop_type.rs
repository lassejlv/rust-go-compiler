//~ ERROR E0509
struct Guard {
    name: String,
}

impl Drop for Guard {
    fn drop(&mut self) {
        println!("drop");
    }
}

fn main() {
    let g = Guard { name: "g".into() };
    let n = g.name;
    println!("{}", n);
}
