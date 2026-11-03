// Valid programs that a naive borrow checker / front end might reject.
// Each module is an independent program; main runs them in order.

mod p02_option_mut_reborrow {
    #[derive(Debug)]
    struct Node { v: i32, next: Option<Box<Node>> }
    fn bump(o: Option<&mut Vec<i32>>) { if let Some(v) = o { v.push(1); v.push(2); } }
    fn bump_all(head: &mut Node) {
        let mut cur: Option<&mut Node> = Some(head);
        while let Some(n) = cur {
            n.v *= 10;
            cur = n.next.as_deref_mut();
        }
    }
    fn last_val(head: &mut Node) -> &mut i32 {
        let mut cur = head;
        while cur.next.is_some() {
            cur = cur.next.as_mut().unwrap();
        }
        &mut cur.v
    }
    pub fn main() {
        let mut v = vec![0];
        bump(Some(&mut v));
        let mut ov = Some(&mut v);
        if let Some(inner) = ov.as_deref_mut() { inner.push(3); }
        if let Some(inner) = ov.as_mut() { inner.push(4); }
        bump(ov);
        println!("{:?}", v);
        let mut head = Node { v: 1, next: Some(Box::new(Node { v: 2, next: Some(Box::new(Node { v: 3, next: None })) })) };
        bump_all(&mut head);
        *last_val(&mut head) += 5;
        println!("{:?}", head);
    }
}

mod p16_reborrow_passing {
    fn add_one(v: &mut Vec<i32>) { v.push(1); }
    fn add_two(v: &mut Vec<i32>) { add_one(v); add_one(&mut *v); v.push(2); }
    fn len_of(v: &Vec<i32>) -> usize { v.len() }
    struct W<'a> { inner: &'a mut Vec<i32> }
    impl<'a> W<'a> {
        fn push(&mut self, x: i32) -> &mut Self { self.inner.push(x); self }
        fn reborrow(&mut self) -> W<'_> { W { inner: &mut *self.inner } }
    }
    pub fn main() {
        let mut v = vec![];
        let r = &mut v;
        add_two(r);
        add_two(r);
        let n = len_of(r);
        r.push(n as i32);
        {
            let mut w = W { inner: r };
            w.push(5).push(6);
            w.reborrow().push(7);
            w.push(8);
        }
        println!("{:?}", v);
    }
}

mod p41_implicit_reborrow_sites {
    struct W<'a> { v: &'a mut Vec<i32> }
    fn push_it(v: &mut Vec<i32>, x: i32) -> &mut Vec<i32> { v.push(x); v }
    fn pass_through<'a>(v: &'a mut Vec<i32>) -> &'a mut Vec<i32> { let w = W { v: v }; w.v }
    pub fn main() {
        let mut data = vec![];
        let r = &mut data;
        push_it(r, 1);
        push_it(r, 2).push(3);
        let w = W { v: r };
        w.v.push(4);
        let r2: &mut Vec<i32> = w.v;
        r2.push(5);
        let o: Option<&mut Vec<i32>> = Some(r);
        if let Some(x) = o { x.push(6); }
        pass_through(&mut data).push(7);
        let mut n = 0;
        let rn = &mut n;
        { let tmp: &mut i32 = rn; *tmp += 1; }
        *rn += 1;
        println!("{:?} {}", data, n);
    }
}

fn main() {
    p02_option_mut_reborrow::main();
    p16_reborrow_passing::main();
    p41_implicit_reborrow_sites::main();
}
