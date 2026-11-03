//~ ERROR E0603
mod m {
    fn secret() -> i32 {
        1
    }
}

fn main() {
    println!("{}", m::secret());
}
