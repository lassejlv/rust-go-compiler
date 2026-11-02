#[derive(Debug)]
struct Address {
    street: String,
    zip: u32,
}

#[derive(Debug)]
struct Person {
    name: String,
    age: u8,
    address: Address,
    phones: Vec<String>,
    nickname: Option<String>,
}

#[derive(Debug)]
enum Event {
    Login { user: String, ok: bool },
    Logout(u32),
    Tick,
}

#[derive(Debug)]
struct Pair(i32, i32);

fn main() {
    let p = Person {
        name: "Ann".to_string(),
        age: 41,
        address: Address { street: "Main St 1".to_string(), zip: 12345 },
        phones: vec!["555-1234".to_string(), "555-9876".to_string()],
        nickname: None,
    };
    println!("{:#?}", p);
    let events = vec![
        Event::Login { user: "ann".to_string(), ok: true },
        Event::Logout(7),
        Event::Tick,
    ];
    println!("{:#?}", events);
    println!("{:#?}", Pair(1, 2));
    println!("{:#?}", (1, "two"));
    println!("{:#?}", Some(Some(3)));
    let empty: Vec<i32> = vec![];
    println!("{:#?}", empty);
    println!("{:#?}", [[1, 2], [3, 4]]);
    println!("{:#?}", 5);
    println!("{:#?}", "str");
}
