//~ ERROR E0603
mod cfg {
    const LIMIT: usize = 10;
}

fn main() {
    println!("{}", cfg::LIMIT);
}
