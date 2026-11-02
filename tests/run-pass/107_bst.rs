#[derive(Debug)]
struct Node {
    key: i32,
    left: Option<Box<Node>>,
    right: Option<Box<Node>>,
}

#[derive(Debug, Default)]
struct Bst {
    root: Option<Box<Node>>,
    size: usize,
}

impl Bst {
    fn insert(&mut self, key: i32) -> bool {
        let mut cur = &mut self.root;
        while let Some(node) = cur {
            if key == node.key {
                return false;
            }
            cur = if key < node.key { &mut node.left } else { &mut node.right };
        }
        *cur = Some(Box::new(Node { key, left: None, right: None }));
        self.size += 1;
        true
    }

    fn contains(&self, key: i32) -> bool {
        let mut cur = &self.root;
        while let Some(node) = cur {
            if key == node.key {
                return true;
            }
            cur = if key < node.key { &node.left } else { &node.right };
        }
        false
    }

    fn inorder(&self) -> Vec<i32> {
        fn walk(n: &Option<Box<Node>>, out: &mut Vec<i32>) {
            if let Some(node) = n {
                walk(&node.left, out);
                out.push(node.key);
                walk(&node.right, out);
            }
        }
        let mut out = Vec::new();
        walk(&self.root, &mut out);
        out
    }

    fn preorder(&self) -> Vec<i32> {
        let mut out = Vec::new();
        let mut stack: Vec<&Box<Node>> = Vec::new();
        if let Some(r) = &self.root {
            stack.push(r);
        }
        while let Some(n) = stack.pop() {
            out.push(n.key);
            if let Some(r) = &n.right {
                stack.push(r);
            }
            if let Some(l) = &n.left {
                stack.push(l);
            }
        }
        out
    }

    fn height(&self) -> usize {
        fn h(n: &Option<Box<Node>>) -> usize {
            match n {
                None => 0,
                Some(b) => 1 + h(&b.left).max(h(&b.right)),
            }
        }
        h(&self.root)
    }

    fn min(&self) -> Option<i32> {
        let mut cur = self.root.as_ref()?;
        while let Some(l) = cur.left.as_ref() {
            cur = l;
        }
        Some(cur.key)
    }

    fn max(&self) -> Option<i32> {
        let mut cur = self.root.as_deref()?;
        while let Some(r) = cur.right.as_deref() {
            cur = r;
        }
        Some(cur.key)
    }

    fn print(&self) {
        fn p(n: &Option<Box<Node>>, depth: usize) {
            if let Some(node) = n {
                p(&node.right, depth + 1);
                println!("{}{}", "    ".repeat(depth), node.key);
                p(&node.left, depth + 1);
            }
        }
        p(&self.root, 0);
    }
}

fn main() {
    let mut t = Bst::default();
    for k in [50, 30, 70, 20, 40, 60, 80, 35, 45, 65] {
        t.insert(k);
    }
    println!("duplicate insert 40: {}", t.insert(40));
    println!("size = {}, height = {}", t.size, t.height());
    println!("inorder = {:?}", t.inorder());
    println!("preorder = {:?}", t.preorder());
    println!("min = {:?}, max = {:?}", t.min(), t.max());
    for k in [45, 55, 80, 10] {
        println!("contains {} = {}", k, t.contains(k));
    }
    t.print();
    let empty = Bst::default();
    println!("empty: min={:?} height={} inorder={:?}", empty.min(), empty.height(), empty.inorder());
    let mut skewed = Bst::default();
    for k in 1..=6 {
        skewed.insert(k);
    }
    println!("skewed height = {}", skewed.height());
}
