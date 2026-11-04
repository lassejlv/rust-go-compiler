//~ ERROR E0527
fn main() {
    let rgb = [255u8, 128, 0];
    let [r, g] = rgb;
    println!("{} {}", r, g);
}
