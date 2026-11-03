//~ ERROR E0499
fn main() {
    let mut stack = vec![1, 2, 3];
    let mut keep: Vec<&mut i32> = Vec::new();
    while let Some(top) = stack.last_mut() {
        keep.push(top);
        if keep.len() > 2 { break; }
    }
}
