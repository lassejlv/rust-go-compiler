//~ ERROR E0277
fn main() {
    let v = vec!["1", "x"];
    let nums: Vec<i32> = v.iter().map(|s| s.parse::<i32>()).collect();
    println!("{:?}", nums);
}
