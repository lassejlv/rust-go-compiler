#[derive(Debug)]
struct User {
    name: String,
    manager_id: Option<u32>,
}

fn lookup(id: u32) -> Option<User> {
    match id {
        1 => Some(User { name: "ceo".to_string(), manager_id: None }),
        2 => Some(User { name: "cto".to_string(), manager_id: Some(1) }),
        3 => Some(User { name: "dev".to_string(), manager_id: Some(2) }),
        _ => None,
    }
}

fn manager_name(id: u32) -> Option<String> {
    lookup(id).and_then(|u| u.manager_id).and_then(lookup).map(|m| m.name)
}

fn parse_positive(s: &str) -> Option<u32> {
    s.parse::<u32>().ok().filter(|&n| n > 0)
}

fn main() {
    let a = Some(4);
    println!("map: {:?}", a.map(|x| x * x));
    println!("map none: {:?}", None::<i32>.map(|x| x * x));
    println!("and_then: {:?}", a.and_then(|x| if x > 3 { Some(x - 3) } else { None }));
    println!("filter: {:?}", a.filter(|x| x % 2 == 1));
    println!("or: {:?}", None.or(Some(9)));
    println!("or_else: {:?}", None::<i32>.or_else(|| Some(10)));
    println!("xor: {:?}", Some(1).xor(None));
    println!("zip: {:?}", Some(1).zip(Some("one")));
    println!("ok_or: {:?}", None::<i32>.ok_or("missing"));
    println!("ok_or some: {:?}", a.ok_or("missing"));
    println!("map_or: {}", a.map_or(0, |x| x + 100));
    println!("map_or_else: {}", None::<i32>.map_or_else(|| -1, |x| x + 100));
    println!("unwrap_or: {}", None.unwrap_or(5));
    println!("is_some_and: {}", a.is_some_and(|x| x > 2));
    let s: Option<String> = Some("hello".to_string());
    println!("as_ref len: {:?}", s.as_ref().map(|x| x.len()));
    println!("as_deref: {:?}", s.as_deref());
    println!("still have s: {:?}", s);
    for id in 1..=4 {
        println!("manager of {}: {:?}", id, manager_name(id));
    }
    for t in ["12", "0", "-3", "abc", "7"] {
        println!("parse_positive({:?}) = {:?}", t, parse_positive(t));
    }
    let nested: Option<Option<i32>> = Some(Some(3));
    println!("flatten: {:?}", nested.flatten());
    let opts = [Some(1), None, Some(3)];
    let total: i32 = opts.iter().map(|o| o.unwrap_or(0)).sum();
    println!("total = {}", total);
    let all: Option<Vec<i32>> = opts.iter().cloned().collect();
    println!("collect with None: {:?}", all);
    let good = [Some(1), Some(2)];
    let all2: Option<Vec<i32>> = good.iter().cloned().collect();
    println!("collect all Some: {:?}", all2);
    let mut cache: Option<i32> = None;
    let v = *cache.get_or_insert(42);
    println!("get_or_insert: {} {:?}", v, cache);
    println!("user debug: {:?}", lookup(3));
}
