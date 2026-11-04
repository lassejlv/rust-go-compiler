// `?` on std::ops::ControlFlow (early exit with Break, continue with the
// Continue payload) in functions, generic functions and closures; and array
// patterns that fix the type of an otherwise unconstrained initializer
// (`let [a, b, c] = v.try_into().ok()?;`).
use std::collections::BTreeMap;
use std::ops::ControlFlow;

fn walk(v: &[i32], depth: u32) -> ControlFlow<String, u32> {
    let mut n = 0;
    for &x in v {
        if x < 0 {
            return ControlFlow::Break(format!("negative {} at depth {}", x, depth));
        }
        n += x as u32;
    }
    ControlFlow::Continue(n)
}

fn total(vs: &[Vec<i32>]) -> ControlFlow<String, u32> {
    let mut t = 0;
    for (i, v) in vs.iter().enumerate() {
        t += walk(v, i as u32)?;
    }
    ControlFlow::Continue(t)
}

fn unit_break(x: i32) -> ControlFlow<()> {
    let c: ControlFlow<(), i32> = if x > 3 { ControlFlow::Break(()) } else { ControlFlow::Continue(x) };
    let y = c?;
    println!("got {}", y);
    ControlFlow::Continue(())
}

fn generic<B: Clone>(b: Option<B>) -> ControlFlow<B, ()> {
    let r: ControlFlow<B, ()> = match b {
        Some(b) => ControlFlow::Break(b),
        None => ControlFlow::Continue(()),
    };
    r?;
    println!("generic continued");
    ControlFlow::Continue(())
}

// a visitor over a tree that stops at the first match
enum Tree {
    Leaf(i64),
    Node(Vec<Tree>),
}

fn visit(t: &Tree, path: &mut Vec<usize>, f: &mut impl FnMut(i64) -> bool) -> ControlFlow<Vec<usize>> {
    match t {
        Tree::Leaf(v) => {
            if f(*v) {
                return ControlFlow::Break(path.clone());
            }
        }
        Tree::Node(kids) => {
            for (i, k) in kids.iter().enumerate() {
                path.push(i);
                visit(k, path, f)?;
                path.pop();
            }
        }
    }
    ControlFlow::Continue(())
}

fn parse_rgb(s: &str) -> Option<(u8, u8, u8)> {
    let [r, g, b] = s.split(',').map(|p| p.trim().parse::<u8>().ok()).collect::<Option<Vec<_>>>()?.try_into().ok()?;
    Some((r, g, b))
}

fn pairs(v: Vec<u32>) -> Result<u32, String> {
    let [a, b] = v.try_into().map_err(|v: Vec<u32>| format!("need 2, got {}", v.len()))?;
    Ok(a * 10 + b)
}

fn main() {
    println!("{:?}", total(&[vec![1, 2], vec![3]]));
    println!("{:?}", total(&[vec![1, 2], vec![3, -4], vec![]]));
    println!("{:?} {:?}", unit_break(2), unit_break(5));
    println!("{:?} {:?}", generic::<&str>(None), generic(Some(vec![1u8])));
    let r = (|| -> ControlFlow<i32, i32> {
        let a = ControlFlow::<i32, i32>::Continue(1)?;
        let b = ControlFlow::<i32, i32>::Break(a + 41)?;
        ControlFlow::Continue(b)
    })();
    println!("{:?} {} {}", r, r.is_break(), r.is_continue());

    let t = Tree::Node(vec![Tree::Leaf(1), Tree::Node(vec![Tree::Leaf(5), Tree::Node(vec![Tree::Leaf(8), Tree::Leaf(13)])]), Tree::Leaf(21)]);
    let mut seen = vec![];
    let found = visit(&t, &mut vec![], &mut |v| {
        seen.push(v);
        v % 2 == 0
    });
    println!("found {:?} after {:?}", found, seen);
    println!("none: {:?}", visit(&t, &mut vec![], &mut |v| v > 100));

    let mut budget: BTreeMap<&str, i32> = BTreeMap::new();
    let res = ["rent:-900", "pay:2500", "food:-400", "car:-1500", "fun:-300"].iter().try_for_each(|entry| {
        let (k, v) = entry.split_once(':').unwrap();
        let v: i32 = v.parse().unwrap();
        let bal = budget.values().sum::<i32>() + v;
        if bal < 0 {
            return ControlFlow::Break((k, bal));
        }
        budget.insert(k, v);
        ControlFlow::Continue(())
    });
    println!("{:?} {:?}", res, budget);

    println!("{:?} {:?} {:?}", parse_rgb("1, 2,3"), parse_rgb("1,2"), parse_rgb("1,x,3"));
    println!("{:?} {:?}", pairs(vec![4, 2]), pairs(vec![1, 2, 3]));
}
