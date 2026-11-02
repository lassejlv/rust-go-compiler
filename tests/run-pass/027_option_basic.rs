fn find_index(haystack: &[i32], needle: i32) -> Option<usize> {
    for i in 0..haystack.len() {
        if haystack[i] == needle {
            return Some(i);
        }
    }
    None
}

fn safe_div(a: i32, b: i32) -> Option<i32> {
    if b == 0 { None } else { Some(a / b) }
}

fn describe(o: Option<i32>) -> String {
    match o {
        Some(0) => String::from("zero"),
        Some(n) if n > 0 => format!("positive {}", n),
        Some(n) => format!("negative {}", n),
        None => String::from("nothing"),
    }
}

fn main() {
    let data = [4, 8, 15, 16, 23, 42];
    println!("find 15: {:?}", find_index(&data, 15));
    println!("find 99: {:?}", find_index(&data, 99));
    match find_index(&data, 23) {
        Some(i) => println!("23 at index {}", i),
        None => println!("23 not found"),
    }
    println!("safe_div(10, 3) = {:?}", safe_div(10, 3));
    println!("safe_div(10, 0) = {:?}", safe_div(10, 0));
    println!("unwrap = {}", safe_div(20, 4).unwrap());
    println!("unwrap_or = {}", safe_div(1, 0).unwrap_or(-1));
    println!("unwrap_or_default = {}", safe_div(1, 0).unwrap_or_default());
    println!("unwrap_or_else = {}", safe_div(1, 0).unwrap_or_else(|| 77));
    println!("expect = {}", Some(3).expect("must exist"));
    println!("is_some/is_none: {} {}", Some(1).is_some(), None::<i32>.is_none());
    for o in [Some(0), Some(5), Some(-3), None] {
        println!("{}", describe(o));
    }
    let mut slot: Option<String> = None;
    println!("slot = {:?}", slot);
    slot = Some(String::from("filled"));
    println!("slot = {:?}", slot);
    if let Some(s) = &slot {
        println!("slot len = {}", s.len());
    }
    let taken = slot.take();
    println!("taken = {:?}, slot = {:?}", taken, slot);
    let mut counter = Some(3);
    let old = counter.replace(4);
    println!("old = {:?}, new = {:?}", old, counter);
    let x: Option<&str> = Some("hi");
    println!("x == Some(\"hi\"): {}", x == Some("hi"));
    println!("Some(1) < Some(2): {}, None < Some(0): {}", Some(1) < Some(2), None < Some(0));
}
