// Valid programs that a naive borrow checker / front end might reject.
// Each module is an independent program; main runs them in order.

mod p15_split_at_mut_chunks {
    pub fn main() {
        let mut v: Vec<i32> = (1..=8).collect();
        {
            let (a, b) = v.split_at_mut(4);
            for (x, y) in a.iter_mut().zip(b.iter()) { *x += *y; }
            b[0] = a[0];
        }
        for c in v.chunks_mut(3) { c.reverse(); }
        if let [first, .., last] = v.as_mut_slice() { std::mem::swap(first, last); }
        let (l, r) = v.split_first_mut().unwrap();
        *l += r.iter().sum::<i32>();
        let mut grid = vec![vec![0; 3]; 3];
        for (i, row) in grid.iter_mut().enumerate() { for (j, c) in row.iter_mut().enumerate() { *c = i * 3 + j; } }
        let (top, bottom) = grid.split_at_mut(1);
        top[0][0] = bottom[1][2];
        println!("{:?} {:?}", v, grid);
    }
}

mod p17_stack_pop_push_loop {
    #[derive(Debug)]
    enum T { Leaf(i32), Node(Vec<T>) }
    fn sum(t: &T) -> i32 {
        let mut stack = vec![t];
        let mut total = 0;
        while let Some(n) = stack.pop() {
            match n {
                T::Leaf(v) => total += v,
                T::Node(ch) => stack.extend(ch.iter()),
            }
        }
        total
    }
    fn double(t: &mut T) {
        let mut stack: Vec<&mut T> = vec![t];
        while let Some(n) = stack.pop() {
            match n {
                T::Leaf(v) => *v *= 2,
                T::Node(ch) => { for c in ch.iter_mut() { stack.push(c); } }
            }
        }
    }
    pub fn main() {
        let mut t = T::Node(vec![T::Leaf(1), T::Node(vec![T::Leaf(2), T::Leaf(3)]), T::Leaf(4)]);
        double(&mut t);
        println!("{} {:?}", sum(&t), t);
    }
}

mod p26_while_loop_ref_update {
    fn longest<'a>(words: &[&'a str]) -> &'a str {
        let mut best = "";
        let mut i = 0;
        while i < words.len() {
            if words[i].len() > best.len() { best = words[i]; }
            i += 1;
        }
        best
    }
    fn find_idx(v: &mut Vec<i32>, target: i32) -> &mut i32 {
        let idx = match v.iter().position(|&x| x == target) {
            Some(i) => i,
            None => { v.push(target); v.len() - 1 }
        };
        &mut v[idx]
    }
    pub fn main() {
        let s1 = String::from("aa bbbb c");
        let words: Vec<&str> = s1.split(' ').collect();
        let l = longest(&words);
        let mut v = vec![1, 2];
        *find_idx(&mut v, 2) += 10;
        *find_idx(&mut v, 7) += 1;
        let mut cur = &v[..];
        while let [first, rest @ ..] = cur { if *first > 10 { break; } cur = rest; }
        println!("{} {:?} {:?}", l, v, cur);
    }
}

fn main() {
    p15_split_at_mut_chunks::main();
    p17_stack_pop_push_loop::main();
    p26_while_loop_ref_update::main();
}
