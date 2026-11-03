// Valid programs that a naive borrow checker / front end might reject.
// Each module is an independent program; main runs them in order.

mod p30_loop_break_with_ref {
    pub fn main() {
        let v = vec![3, 8, 1, 9];
        let mut i = 0;
        let found = loop {
            if i >= v.len() { break None; }
            if v[i] > 5 { break Some(&v[i]); }
            i += 1;
        };
        let mut grid = vec![vec![0; 3]; 2];
        let cell = 'outer: loop {
            for row in grid.iter_mut() {
                for c in row.iter_mut() {
                    if *c == 0 { break 'outer c; }
                }
            }
            unreachable!();
        };
        *cell = 7;
        let label_val = 'blk: {
            if v.len() > 2 { break 'blk v[2]; }
            0
        };
        println!("{:?} {:?} {}", found, grid, label_val);
    }
}

mod p37_linked_list_match_cursor {
    #[derive(Debug)]
    struct Node { v: i32, next: Option<Box<Node>> }
    fn push_back(head: &mut Node, v: i32) {
        let mut cur = head;
        while let Some(ref mut next) = cur.next { cur = next; }
        cur.next = Some(Box::new(Node { v, next: None }));
    }
    fn remove_all(head: &mut Option<Box<Node>>, v: i32) {
        let mut cur = head;
        loop {
            match cur {
                None => break,
                Some(n) if n.v == v => { *cur = n.next.take(); }
                Some(n) => { cur = &mut n.next; }
            }
        }
    }
    pub fn main() {
        let mut h = Node { v: 0, next: None };
        for i in 1..5 { push_back(&mut h, i % 2); }
        let mut list = h.next.take();
        remove_all(&mut list, 1);
        println!("{:?}", list);
    }
}

mod p29_shadow_and_scopes {
    pub fn main() {
        let s = String::from("first");
        let r = &s;
        let s = r.len();
        println!("{} {}", r, s);
        let mut v = vec![1];
        {
            let v = &mut v;
            v.push(2);
        }
        v.push(3);
        let x;
        {
            let y = 5;
            x = y * 2;
        }
        let data = vec![String::from("a"), String::from("b")];
        let refs: Vec<&str> = data.iter().map(|s| s.as_str()).collect();
        let joined = refs.join("-");
        drop(refs);
        let data2 = data;
        println!("{:?} {} {} {:?}", v, x, joined, data2);
    }
}

mod p27_match_guard_borrows {
    #[derive(Debug)]
    enum Cmd { Push(i32), Pop, Dup }
    fn exec(stack: &mut Vec<i32>, cmds: &[Cmd]) {
        for c in cmds {
            match c {
                Cmd::Push(n) if stack.len() < 4 => stack.push(*n),
                Cmd::Push(_) => {}
                Cmd::Pop => { stack.pop(); }
                Cmd::Dup => match stack.last() {
                    Some(&top) => stack.push(top),
                    None => {}
                },
            }
        }
    }
    pub fn main() {
        let mut s = vec![];
        exec(&mut s, &[Cmd::Push(1), Cmd::Dup, Cmd::Push(5), Cmd::Dup, Cmd::Push(9), Cmd::Pop, Cmd::Dup]);
        let mut o = Some(String::from("x"));
        match o { Some(ref mut inner) if inner.len() < 3 => inner.push('y'), _ => {} }
        match &mut o { Some(v) => v.push('z'), None => {} }
        let n = match o.as_deref() { Some("xyz") => 1, Some(_) => 2, None => 3 };
        println!("{:?} {:?} {}", s, o, n);
    }
}

fn main() {
    p30_loop_break_with_ref::main();
    p37_linked_list_match_cursor::main();
    p29_shadow_and_scopes::main();
    p27_match_guard_borrows::main();
}
