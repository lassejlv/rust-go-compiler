// Borrowing inside loops: references that do not outlive the iteration.
fn main() {
    let mut data = vec![String::from("x")];
    for i in 0..4 {
        let last = data.last().unwrap();
        let next = format!("{}{}", last, i);
        data.push(next);
    }
    let found = loop {
        let candidate = &data[2];
        if candidate.len() > 1 {
            break candidate.clone();
        }
    };
    data.push(found);
    let mut i = 0;
    while i < data.len() {
        let len = data[i].len();
        if len % 2 == 0 {
            data[i].push('#');
        }
        i += 1;
    }
    println!("{:?}", data);
}
