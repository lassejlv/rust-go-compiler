use std::rc::Rc;

#[derive(Debug)]
struct Config {
    name: String,
    level: u8,
}

#[derive(Debug)]
enum List {
    Cons(i32, Rc<List>),
    Nil,
}

use List::{Cons, Nil};

fn sum(l: &List) -> i32 {
    match l {
        Cons(v, rest) => v + sum(rest),
        Nil => 0,
    }
}

struct Service {
    id: u32,
    cfg: Rc<Config>,
}

fn main() {
    let cfg = Rc::new(Config { name: "shared".to_string(), level: 3 });
    println!("count after create = {}", Rc::strong_count(&cfg));
    let services: Vec<Service> = (1..=3).map(|id| Service { id, cfg: Rc::clone(&cfg) }).collect();
    println!("count with 3 services = {}", Rc::strong_count(&cfg));
    for s in &services {
        println!("service {} uses {} (level {})", s.id, s.cfg.name, s.cfg.level);
    }
    {
        let extra = cfg.clone();
        println!("count in scope = {}, ptr_eq = {}", Rc::strong_count(&extra), Rc::ptr_eq(&extra, &services[0].cfg));
    }
    println!("count after scope = {}", Rc::strong_count(&cfg));
    drop(services);
    println!("count after drop services = {}", Rc::strong_count(&cfg));

    let a = Rc::new(Cons(5, Rc::new(Cons(10, Rc::new(Nil)))));
    println!("a count = {}", Rc::strong_count(&a));
    let b = Cons(3, Rc::clone(&a));
    println!("a count after b = {}", Rc::strong_count(&a));
    {
        let c = Cons(4, Rc::clone(&a));
        println!("a count after c = {}", Rc::strong_count(&a));
        println!("sum c = {}", sum(&c));
    }
    println!("a count after c dropped = {}", Rc::strong_count(&a));
    println!("sum b = {}", sum(&b));
    println!("b = {:?}", b);

    let s: Rc<str> = Rc::from("immutable text");
    let s2 = Rc::clone(&s);
    println!("{} / {} / count {}", s, s2.len(), Rc::strong_count(&s));
    let mut unique = Rc::new(5);
    *Rc::make_mut(&mut unique) += 1;
    println!("make_mut unique = {}", unique);
    let other = Rc::clone(&unique);
    *Rc::make_mut(&mut unique) *= 10;
    println!("make_mut cloned-on-write: unique = {}, other = {}", unique, other);
    println!("try_unwrap = {:?}", Rc::try_unwrap(other));
    let v = Rc::new(vec![1, 2, 3]);
    let v2 = v.clone();
    println!("deref len = {}, sum = {}", v2.len(), v.iter().sum::<i32>());
}
