use std::fmt::Display;

fn print_it(item: impl Display) {
    println!("[{}]", item);
}

fn total_len(items: impl Iterator<Item = String>) -> usize {
    items.map(|s| s.len()).sum()
}

fn evens(limit: u32) -> impl Iterator<Item = u32> {
    (0..limit).filter(|n| n % 2 == 0)
}

fn squares_of(v: Vec<i32>) -> impl Iterator<Item = i32> {
    v.into_iter().map(|x| x * x)
}

fn make_label(n: i32) -> impl Display {
    format!("label-{}", n)
}

trait Speaker {
    fn speak(&self) -> String;
}

struct Cat;
impl Speaker for Cat {
    fn speak(&self) -> String {
        "meow".to_string()
    }
}

fn get_speaker() -> impl Speaker {
    Cat
}

fn speak_twice(s: &impl Speaker) -> String {
    format!("{} {}", s.speak(), s.speak())
}

fn adder(n: i32) -> impl Fn(i32) -> i32 {
    move |x| x + n
}

fn main() {
    print_it(42);
    print_it("text");
    print_it(3.5);
    print_it(make_label(7));
    let words = vec!["ab".to_string(), "cde".to_string()];
    println!("total_len = {}", total_len(words.into_iter()));
    let e: Vec<u32> = evens(10).collect();
    println!("evens = {:?}", e);
    println!("sum of squares = {}", squares_of(vec![1, 2, 3]).sum::<i32>());
    let sp = get_speaker();
    println!("speaker: {}", speak_twice(&sp));
    let add5 = adder(5);
    println!("adder: {}", add5(10));
    let chained: Vec<u32> = evens(20).skip(2).take(3).collect();
    println!("chained = {:?}", chained);
}
