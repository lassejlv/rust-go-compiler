use std::fmt::Write;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct NodeId(usize);

#[derive(Debug)]
struct Node<T> {
    value: T,
    parent: Option<NodeId>,
    children: Vec<NodeId>,
    alive: bool,
}

struct Arena<T> {
    nodes: Vec<Node<T>>,
    free: Vec<NodeId>,
}

impl<T: std::fmt::Debug> Arena<T> {
    fn new() -> Self {
        Arena { nodes: Vec::new(), free: Vec::new() }
    }
    fn alloc(&mut self, value: T) -> NodeId {
        let node = Node { value, parent: None, children: Vec::new(), alive: true };
        if let Some(id) = self.free.pop() {
            self.nodes[id.0] = node;
            id
        } else {
            self.nodes.push(node);
            NodeId(self.nodes.len() - 1)
        }
    }
    fn add_child(&mut self, parent: NodeId, value: T) -> NodeId {
        let id = self.alloc(value);
        self.nodes[id.0].parent = Some(parent);
        self.nodes[parent.0].children.push(id);
        id
    }
    fn get(&self, id: NodeId) -> &T {
        &self.nodes[id.0].value
    }
    fn get_mut(&mut self, id: NodeId) -> &mut T {
        &mut self.nodes[id.0].value
    }
    fn remove_subtree(&mut self, id: NodeId) -> usize {
        if let Some(p) = self.nodes[id.0].parent {
            self.nodes[p.0].children.retain(|&c| c != id);
        }
        let mut stack = vec![id];
        let mut removed = 0;
        while let Some(n) = stack.pop() {
            stack.extend(self.nodes[n.0].children.drain(..));
            self.nodes[n.0].alive = false;
            self.nodes[n.0].parent = None;
            self.free.push(n);
            removed += 1;
        }
        removed
    }
    fn path_to_root(&self, mut id: NodeId) -> Vec<NodeId> {
        let mut v = vec![id];
        while let Some(p) = self.nodes[id.0].parent {
            v.push(p);
            id = p;
        }
        v
    }
    fn depth_first(&self, root: NodeId) -> Vec<(usize, NodeId)> {
        let mut out = Vec::new();
        let mut stack = vec![(0, root)];
        while let Some((d, n)) = stack.pop() {
            out.push((d, n));
            for &c in self.nodes[n.0].children.iter().rev() {
                stack.push((d + 1, c));
            }
        }
        out
    }
    fn render(&self, root: NodeId) -> String {
        let mut s = String::new();
        for (d, n) in self.depth_first(root) {
            writeln!(s, "{}{:?} (#{})", "  ".repeat(d), self.get(n), n.0).unwrap();
        }
        s
    }
    fn live_count(&self) -> usize {
        self.nodes.iter().filter(|n| n.alive).count()
    }
    fn lca(&self, a: NodeId, b: NodeId) -> Option<NodeId> {
        let pa = self.path_to_root(a);
        let pb = self.path_to_root(b);
        pa.into_iter().find(|n| pb.contains(n))
    }
}

fn subtree_sum(arena: &Arena<i64>, id: NodeId) -> i64 {
    *arena.get(id) + arena.nodes[id.0].children.iter().map(|&c| subtree_sum(arena, c)).sum::<i64>()
}

fn main() {
    let mut a: Arena<&str> = Arena::new();
    let root = a.alloc("root");
    let usr = a.add_child(root, "usr");
    let bin = a.add_child(usr, "bin");
    let lib = a.add_child(usr, "lib");
    let etc = a.add_child(root, "etc");
    let _hosts = a.add_child(etc, "hosts");
    let rustc = a.add_child(bin, "rustc");
    let cargo = a.add_child(bin, "cargo");
    print!("{}", a.render(root));
    println!("path {:?}", a.path_to_root(cargo).iter().map(|&n| *a.get(n)).collect::<Vec<_>>());
    println!("lca(rustc, lib) = {:?}", a.lca(rustc, lib).map(|n| *a.get(n)));
    println!("lca(cargo, hosts) = {:?}", a.lca(cargo, _hosts).map(|n| *a.get(n)));
    println!("live {}", a.live_count());
    println!("removed {}", a.remove_subtree(usr));
    println!("live {} free {:?}", a.live_count(), a.free);
    let opt = a.add_child(root, "opt");
    let tool = a.add_child(opt, "tool");
    println!("reused ids {:?} {:?}", opt, tool);
    *a.get_mut(tool) = "tool-v2";
    print!("{}", a.render(root));

    let mut nums: Arena<i64> = Arena::new();
    let r = nums.alloc(1);
    let mut frontier = vec![r];
    let mut next_val = 2;
    for _level in 0..3 {
        let mut nf = Vec::new();
        for &p in &frontier {
            for _ in 0..2 {
                nf.push(nums.add_child(p, next_val));
                next_val += 1;
            }
        }
        frontier = nf;
    }
    println!("nodes {} sum {}", nums.live_count(), subtree_sum(&nums, r));
    let left = nums.nodes[r.0].children[0];
    println!("left subtree sum {}", subtree_sum(&nums, left));
    for &leaf in frontier.iter().take(3) {
        *nums.get_mut(leaf) *= 100;
    }
    println!("after scaling sum {}", subtree_sum(&nums, r));
    let depths: Vec<usize> = nums.depth_first(r).iter().map(|(d, _)| *d).collect();
    println!("{:?}", depths);
}
