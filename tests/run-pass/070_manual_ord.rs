use std::cmp::Ordering;

#[derive(Debug, Clone)]
struct Task {
    name: String,
    priority: u32,
    duration: u32,
}

impl PartialEq for Task {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority && self.duration == other.duration
    }
}

impl Eq for Task {}

impl PartialOrd for Task {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Task {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .priority
            .cmp(&self.priority)
            .then_with(|| self.duration.cmp(&other.duration))
    }
}

#[derive(Debug, Clone, Copy)]
struct Celsius(f64);

impl PartialEq for Celsius {
    fn eq(&self, other: &Self) -> bool {
        (self.0 - other.0).abs() < 0.01
    }
}

impl PartialOrd for Celsius {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self == other {
            Some(Ordering::Equal)
        } else {
            self.0.partial_cmp(&other.0)
        }
    }
}

fn describe(o: Ordering) -> &'static str {
    match o {
        Ordering::Less => "less",
        Ordering::Equal => "equal",
        Ordering::Greater => "greater",
    }
}

fn main() {
    let mut tasks = vec![
        Task { name: "write".to_string(), priority: 2, duration: 30 },
        Task { name: "test".to_string(), priority: 5, duration: 60 },
        Task { name: "deploy".to_string(), priority: 5, duration: 15 },
        Task { name: "lunch".to_string(), priority: 1, duration: 45 },
        Task { name: "review".to_string(), priority: 2, duration: 10 },
    ];
    tasks.sort();
    for t in &tasks {
        println!("{:<7} p={} d={}", t.name, t.priority, t.duration);
    }
    let a = &tasks[0];
    let b = &tasks[1];
    println!("{} vs {}: {}", a.name, b.name, describe(a.cmp(b)));
    let dup = Task { name: "other".to_string(), priority: 5, duration: 15 };
    println!("eq ignoring name: {}", tasks[0] == dup);
    println!("max: {}", tasks.iter().max().unwrap().name);
    let t1 = Celsius(20.001);
    let t2 = Celsius(20.0);
    let t3 = Celsius(25.0);
    println!("t1 == t2: {}", t1 == t2);
    println!("t1 < t3: {}", t1 < t3);
    println!("t3 > t2: {}", t3 > t2);
    println!("cmp t1 t2: {:?}", t1.partial_cmp(&t2));
    println!("reverse: {:?}", Ordering::Less.reverse());
    println!("then: {:?}", Ordering::Equal.then(Ordering::Greater));
    println!("is_lt: {}", 3.cmp(&4).is_lt());
    let mut nums = vec![5, 1, 4, 2, 3];
    nums.sort_by(|a, b| b.cmp(a));
    println!("desc: {:?}", nums);
    nums.sort_by_key(|&x| std::cmp::Reverse(x % 3));
    println!("by Reverse(x%3): {:?}", nums);
    println!("max_by: {:?}", ["aa", "b", "cccc", "dd"].iter().max_by(|a, b| a.len().cmp(&b.len())));
    println!("min_by_key: {:?}", ["aa", "b", "cccc", "dd"].iter().min_by_key(|s| s.len()));
}
