// Swapping and updating elements by index without overlapping borrows.
#[derive(Debug, Clone)]
struct Acct {
    id: u32,
    bal: i64,
}

fn transfer(accts: &mut [Acct], from: usize, to: usize, amt: i64) {
    accts[from].bal -= amt;
    accts[to].bal += amt;
    let fb = accts[from].bal;
    accts[to].bal += fb / 100;
}

fn main() {
    let mut a = vec![Acct { id: 1, bal: 100 }, Acct { id: 2, bal: 50 }, Acct { id: 3, bal: 0 }];
    transfer(&mut a, 0, 2, 30);
    transfer(&mut a, 1, 0, 10);
    a.swap(0, 2);
    let len = a.len();
    let last = a[len - 1].clone();
    a[0].bal += last.bal;
    if let Some(x) = a.get_mut(1) {
        x.id *= 10;
    }
    let (head, tail) = a.split_first_mut().unwrap();
    head.bal += tail.iter().map(|t| t.bal).sum::<i64>();
    println!("{:?}", a);
}
