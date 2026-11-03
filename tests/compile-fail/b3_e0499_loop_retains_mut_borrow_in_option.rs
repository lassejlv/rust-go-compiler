//~ ERROR E0499
fn main() {
    let mut data = vec![1, 2, 3];
    let mut last: Option<&mut i32> = None;
    for _ in 0..2 {
        let r = data.iter_mut().next().unwrap();
        if let Some(prev) = last { *prev += 1; }
        last = Some(r);
    }
    let _ = last;
}
