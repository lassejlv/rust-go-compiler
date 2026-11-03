// mem::take / mem::replace / Option::take to move out of &mut.
use std::mem;

struct Buf {
    data: Vec<String>,
    current: String,
    pending: Option<Box<i32>>,
}

impl Buf {
    fn flush(&mut self) -> Vec<String> {
        let cur = mem::take(&mut self.current);
        if !cur.is_empty() {
            self.data.push(cur);
        }
        mem::replace(&mut self.data, Vec::new())
    }
    fn pop_pending(&mut self) -> i32 {
        match self.pending.take() {
            Some(b) => *b,
            None => -1,
        }
    }
}

fn main() {
    let mut b = Buf { data: vec!["a".into()], current: "b".into(), pending: Some(Box::new(7)) };
    let out = b.flush();
    println!("{:?} {:?} {:?}", out, b.data, b.current);
    println!("{} {}", b.pop_pending(), b.pop_pending());
    let mut s = String::from("old");
    let old = mem::replace(&mut s, String::from("new"));
    println!("{} {}", old, s);
}
