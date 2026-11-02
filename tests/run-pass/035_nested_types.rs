#[derive(Debug, Clone, PartialEq)]
enum Category {
    Tool,
    Food { calories: u32 },
    Weapon(u32),
}

#[derive(Debug, Clone)]
struct Item {
    name: String,
    weight: f32,
    category: Category,
}

#[derive(Debug)]
struct Inventory {
    owner: String,
    items: Vec<Item>,
    capacity: f32,
}

impl Inventory {
    fn new(owner: &str, capacity: f32) -> Self {
        Inventory { owner: owner.to_string(), items: Vec::new(), capacity }
    }

    fn total_weight(&self) -> f32 {
        let mut w = 0.0;
        for it in &self.items {
            w += it.weight;
        }
        w
    }

    fn add(&mut self, item: Item) -> Result<(), String> {
        if self.total_weight() + item.weight > self.capacity {
            return Err(format!("{} is too heavy for {}", item.name, self.owner));
        }
        self.items.push(item);
        Ok(())
    }

    fn total_calories(&self) -> u32 {
        let mut c = 0;
        for it in &self.items {
            if let Category::Food { calories } = it.category {
                c += calories;
            }
        }
        c
    }

    fn strongest_weapon(&self) -> Option<&Item> {
        let mut best: Option<&Item> = None;
        for it in &self.items {
            if let Category::Weapon(dmg) = it.category {
                let better = match best {
                    None => true,
                    Some(b) => match b.category {
                        Category::Weapon(bd) => dmg > bd,
                        _ => true,
                    },
                };
                if better {
                    best = Some(it);
                }
            }
        }
        best
    }
}

fn item(name: &str, weight: f32, category: Category) -> Item {
    Item { name: name.to_string(), weight, category }
}

fn main() {
    let mut inv = Inventory::new("hero", 20.0);
    let candidates = vec![
        item("hammer", 3.5, Category::Tool),
        item("apple", 0.25, Category::Food { calories: 95 }),
        item("sword", 6.0, Category::Weapon(12)),
        item("anvil", 15.0, Category::Tool),
        item("bread", 0.5, Category::Food { calories: 250 }),
        item("dagger", 1.0, Category::Weapon(5)),
    ];
    for c in candidates {
        match inv.add(c) {
            Ok(()) => println!("added, weight now {}", inv.total_weight()),
            Err(e) => println!("error: {}", e),
        }
    }
    println!("items: {}", inv.items.len());
    println!("calories: {}", inv.total_calories());
    if let Some(w) = inv.strongest_weapon() {
        println!("strongest: {} {:?}", w.name, w.category);
    }
    let first = inv.items[0].clone();
    println!("first: {:?}", first);
    println!("first is tool: {}", first.category == Category::Tool);
    println!("{:?}", inv.items[1]);
    println!("owner {} capacity {}", inv.owner, inv.capacity);
}
