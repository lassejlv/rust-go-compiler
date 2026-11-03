// Valid programs that rely on two-phase borrows, NLL and disjoint borrows;
// all accepted by rustc. Each case prints its result.
use std::collections::HashMap;

struct Counter { vals: Vec<u32>, idx: usize, total: u32, name: String }
impl Counter {
    fn get(&self) -> u32 { self.vals[self.idx] }
    fn set(&mut self, v: u32) { let i = self.idx; self.vals[i] = v; }
    fn bump(&mut self) { self.set(self.get() + 1); }
    fn add_all(&mut self) { for v in &self.vals { self.total += v; } }
    fn first_mut(&mut self) -> &mut u32 { &mut self.vals[0] }
    fn rename(&mut self) { self.name.push_str(&self.name.clone()); }
    fn cur_slot(&mut self) { self.vals[self.idx] = self.vals.len() as u32; }
}

struct Node { val: i32, next: Option<Box<Node>> }

fn last_mut(mut cur: &mut Node) -> &mut Node {
    while let Some(ref mut nxt) = cur.next { cur = nxt; }
    cur
}

fn reborrow_arg(v: &mut Vec<usize>) -> usize { v.push(v.len()); v.len() }
fn takes_two(a: &mut Vec<usize>, n: usize) { a.push(n * 10); }

fn main() {
    let mut v = vec![1usize, 2, 3];
    v.push(v.len());
    v.insert(0, v[2] * 100);
    let n = v.len();
    let last = v.len() - 1;
    v.swap(0, last);
    v.truncate(v.len() - 1);
    println!("1 vec two-phase: {:?} (n was {})", v, n);

    let mut w = vec![10, 20, 30];
    w[0] = w[1] + w[2];
    w[1] += w[2];
    let j = w.len() - 1;
    w[j] *= w[0];
    println!("2 index assign: {:?}", w);

    let mut s = String::from("ab");
    s.push_str(&s.clone());
    s.insert_str(s.len() / 2, "-");
    println!("3 string: {}", s);

    let mut m: HashMap<String, usize> = HashMap::new();
    m.insert("a".into(), m.len());
    m.insert("b".into(), m.len());
    match m.get_mut("a") {
        Some(x) => *x += 100,
        None => { m.insert("a".into(), 0); }
    }
    let mut keys: Vec<_> = m.iter().map(|(k, v)| format!("{}={}", k, v)).collect();
    keys.sort();
    println!("4 map: {:?}", keys);

    let mut c = Counter { vals: vec![5, 6, 7], idx: 1, total: 0, name: "c".into() };
    c.bump();
    c.bump();
    c.add_all();
    *c.first_mut() += c.total;
    c.rename();
    c.cur_slot();
    println!("5 counter: {:?} total={} name={}", c.vals, c.total, c.name);

    let mut list = Node { val: 1, next: Some(Box::new(Node { val: 2, next: Some(Box::new(Node { val: 3, next: None })) })) };
    last_mut(&mut list).next = Some(Box::new(Node { val: 4, next: None }));
    last_mut(&mut list).val *= 10;
    let mut vals = vec![];
    let mut cur = Some(&list);
    while let Some(nd) = cur { vals.push(nd.val); cur = nd.next.as_deref(); }
    println!("6 list: {:?}", vals);

    let mut data = vec![3usize, 1, 2];
    let r = &mut data;
    let got = reborrow_arg(r);
    takes_two(r, r.len());
    r.push(r[0]);
    println!("7 reborrow: {} {:?}", got, data);

    let mut pair = (vec![1], vec![2, 3]);
    let a = &mut pair.0;
    let b = &pair.1;
    a.extend(b.iter().map(|x| x * 2));
    std::mem::swap(&mut pair.0[0], &mut pair.1[1]);
    println!("8 disjoint tuple: {:?}", pair);

    let mut arr = [1, 2, 3, 4, 5, 6];
    let (left, right) = arr.split_at_mut(3);
    left[0] += right[2];
    right[0] = left.iter().sum();
    let [x, .., y] = &mut arr;
    std::mem::swap(x, y);
    println!("9 split_at_mut: {:?}", arr);

    let mut st = Counter { vals: vec![1, 2], idx: 0, total: 9, name: "q".into() };
    let name_ref = &st.name;
    let mut add = |k: u32| st.total += k;
    add(1);
    add(2);
    println!("10 closure disjoint: total={} name={}", st.total, name_ref);

    let mut words = vec!["b".to_string(), "a".to_string()];
    let first_len = words[0].len();
    words.push(format!("{}{}", words[1], first_len));
    words.sort();
    let longest = words.iter().max_by_key(|w| w.len()).unwrap().clone();
    words.retain(|w| *w != longest);
    println!("11 words: {:?} removed {}", words, longest);

    let mut grid = vec![vec![0; 3]; 3];
    for i in 0..3 { for k in 0..3 { grid[i][k] = i * 3 + k; } }
    let tmp = grid[0][2];
    grid[2][0] += tmp + grid[1][1];
    let row1 = grid[1].clone();
    grid[0].extend(row1);
    println!("12 grid: {:?}", grid);

    let mut opt: Option<Vec<i32>> = None;
    opt.get_or_insert_with(Vec::new).push(5);
    if let Some(inner) = opt.as_mut() { inner.push(inner[0] + 1); }
    let len = opt.as_ref().map_or(0, |v| v.len());
    opt.as_mut().unwrap().push(len as i32);
    println!("13 option: {:?}", opt);
}
