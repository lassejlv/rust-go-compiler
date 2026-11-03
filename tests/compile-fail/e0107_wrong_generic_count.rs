//~ ERROR E0107
struct W<T>(T);

fn main() {
    let w: W<i32, i32> = W(1);
    println!("{}", w.0);
}
