// while let Some(x) = stack.pop() { stack.push(..) } — the pop borrow ends.
fn main() {
    let mut stack = vec![5u32];
    let mut seen = Vec::new();
    while let Some(n) = stack.pop() {
        seen.push(n);
        if n > 1 {
            stack.push(n / 2);
            if n % 2 == 1 {
                stack.push(n - 1);
            }
        }
    }
    println!("{:?}", seen);
    let mut q = std::collections::VecDeque::new();
    q.push_back(1);
    while let Some(x) = q.pop_front() {
        if x < 20 {
            q.push_back(x * 3);
            q.push_back(x * 3 + 1);
        }
        if q.len() > 6 {
            break;
        }
    }
    println!("{:?}", q);
}
