// ref / ref mut bindings, default binding modes through &/&mut, mutating through matched
// references in Vec<enum>, Option<&mut T>, as_mut/as_deref_mut, and box-free tree editing.
#[derive(Debug)]
enum Node {
    Leaf { val: i32, hits: u32 },
    Pair(Box<Node>, Box<Node>),
    Empty,
}

fn bump_all(n: &mut Node) {
    match n {
        Node::Leaf { val, hits } => {
            *val *= 2;
            *hits += 1;
        }
        Node::Pair(l, r) => {
            bump_all(l);
            bump_all(r);
        }
        Node::Empty => {}
    }
}

fn prune(n: &mut Node) {
    if let Node::Pair(l, r) = n {
        prune(l);
        prune(r);
        let both_empty = matches!((&**l, &**r), (Node::Empty, Node::Empty));
        if both_empty {
            *n = Node::Empty;
        }
    } else if let Node::Leaf { val, .. } = n {
        if *val < 0 {
            *n = Node::Empty;
        }
    }
}

fn sum(n: &Node) -> i32 {
    match *n {
        Node::Leaf { val, .. } => val,
        Node::Pair(ref l, ref r) => sum(l) + sum(r),
        Node::Empty => 0,
    }
}

#[derive(Debug, Default)]
struct Account {
    name: String,
    balance: i64,
    history: Vec<i64>,
}

fn main() {
    let mut tree = Node::Pair(
        Box::new(Node::Leaf { val: 3, hits: 0 }),
        Box::new(Node::Pair(Box::new(Node::Leaf { val: -4, hits: 0 }), Box::new(Node::Leaf { val: 5, hits: 0 }))),
    );
    bump_all(&mut tree);
    println!("sum after bump {}", sum(&tree));
    prune(&mut tree);
    println!("{:?}", tree);
    println!("sum after prune {}", sum(&tree));
    let mut t2 = Node::Pair(Box::new(Node::Leaf { val: -1, hits: 0 }), Box::new(Node::Empty));
    prune(&mut t2);
    println!("{:?}", t2);

    // ref and ref mut in let and match
    let mut pair = (String::from("left"), vec![1, 2, 3]);
    {
        let (ref l, ref mut r) = pair;
        r.push(l.len() as i32);
    }
    match pair {
        (ref mut s, ref v) if v.len() > 3 => s.push_str("+long"),
        (ref mut s, _) => s.push_str("+short"),
    }
    println!("{:?}", pair);

    // default binding modes
    let mut accounts = vec![
        Account { name: "a".into(), balance: 10, history: vec![] },
        Account { name: "b".into(), balance: -5, history: vec![] },
        Account { name: "c".into(), ..Default::default() },
    ];
    for Account { name, balance, history } in accounts.iter_mut() {
        if *balance < 0 {
            history.push(*balance);
            *balance = 0;
            name.push_str("(reset)");
        } else {
            *balance += 1;
            history.push(1);
        }
    }
    for a in &accounts {
        let Account { name, balance, history } = a;
        println!("{} {} {:?}", name, balance, history);
    }

    // Option<&mut T>
    let mut opt: Option<Account> = Some(Account { name: "opt".into(), balance: 1, history: vec![] });
    if let Some(acc) = opt.as_mut() {
        acc.balance *= 100;
    }
    if let Some(Account { ref mut history, .. }) = opt {
        history.push(42);
    }
    match &mut opt {
        Some(a) if a.balance > 50 => a.name = a.name.to_uppercase(),
        Some(_) => {}
        None => {}
    }
    println!("{:?}", opt);

    let mut name: Option<String> = Some("hello".into());
    if let Some(s) = name.as_deref_mut() {
        s.make_ascii_uppercase();
    }
    println!("{:?} {:?}", name.as_deref(), name.as_ref().map(|s| s.len()));

    // nested references
    let data = vec![(1, "one"), (2, "two")];
    let refs: Vec<&(i32, &str)> = data.iter().collect();
    for &&(n, s) in &refs {
        print!("{}:{} ", n, s);
    }
    println!();
    let mut grid = vec![vec![0; 3]; 2];
    for (i, row) in grid.iter_mut().enumerate() {
        for (j, cell) in row.iter_mut().enumerate() {
            *cell = i * 3 + j;
        }
    }
    if let [first, second] = &mut grid[..] {
        std::mem::swap(&mut first[0], &mut second[2]);
    }
    println!("{:?}", grid);
}
