// Option::as_mut / get_or_insert_with / if let Some(x) = &mut opt.
fn main() {
    let mut o: Option<Vec<i32>> = None;
    o.get_or_insert_with(Vec::new).push(1);
    if let Some(v) = o.as_mut() {
        v.push(2);
    }
    if let Some(v) = &mut o {
        v.push(3);
    }
    match o.as_mut() {
        Some(v) if v.len() > 2 => v[0] = 100,
        _ => {}
    }
    let taken = o.take();
    println!("{:?} {:?}", taken, o);
    let mut cnt: Option<u32> = Some(1);
    if let Some(c) = cnt.as_mut() {
        *c += 41;
    }
    let r = cnt.as_ref().map(|c| *c * 2);
    cnt = None;
    println!("{:?} {:?}", r, cnt);
}
