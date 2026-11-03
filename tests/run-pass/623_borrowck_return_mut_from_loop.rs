// Returning &mut from inside a loop over a mutable slice / tree descent.
#[derive(Debug)]
struct Tree {
    key: i32,
    left: Option<Box<Tree>>,
    right: Option<Box<Tree>>,
}

fn find_mut(mut t: &mut Tree, key: i32) -> Option<&mut Tree> {
    loop {
        if key == t.key {
            return Some(t);
        }
        let next = if key < t.key { &mut t.left } else { &mut t.right };
        match next {
            Some(child) => t = child,
            None => return None,
        }
    }
}

fn insert(t: &mut Tree, key: i32) {
    let slot = if key < t.key { &mut t.left } else { &mut t.right };
    match slot {
        Some(child) => insert(child, key),
        None => *slot = Some(Box::new(Tree { key, left: None, right: None })),
    }
}

fn inorder(t: &Tree, out: &mut Vec<i32>) {
    if let Some(l) = &t.left {
        inorder(l, out);
    }
    out.push(t.key);
    if let Some(r) = &t.right {
        inorder(r, out);
    }
}

fn main() {
    let mut root = Tree { key: 50, left: None, right: None };
    for k in [30, 70, 20, 40, 60, 80] {
        insert(&mut root, k);
    }
    if let Some(n) = find_mut(&mut root, 40) {
        n.key = 45;
    }
    println!("{}", find_mut(&mut root, 99).is_none());
    let mut out = Vec::new();
    inorder(&root, &mut out);
    println!("{:?}", out);
}
