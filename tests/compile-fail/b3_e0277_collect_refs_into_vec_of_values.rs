//~ ERROR E0277
fn main() {
    let v: Vec<u32> = "1,2".split(',').map(|s| s.parse::<u32>().unwrap()).collect();
    let w: Vec<&u32> = v.iter().step_by(1).collect::<Vec<u32>>();
    println!("{:?}", w);
}
