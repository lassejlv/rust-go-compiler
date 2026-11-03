// Two-phase borrows: the autoref'd &mut receiver is only activated at the call.
fn main() {
    let mut v: Vec<usize> = Vec::new();
    v.push(v.len());
    v.push(v.len());
    v.push(v.len() * 10);
    let n = v.len();
    v[0] = n;
    let i = v.len() - 1;
    v[i] = v[0] + v[1];
    let last = v.len() - 1;
    v[last] += 1;
    let mut s = String::from("x");
    s.push_str(&s.clone());
    s.insert(s.len(), 'y');
    println!("{:?} {}", v, s);
}
