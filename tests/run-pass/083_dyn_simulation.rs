trait Creature {
    fn name(&self) -> String;
    fn energy(&self) -> i32;
    fn tick(&mut self, turn: u32) -> Option<String>;
    fn alive(&self) -> bool {
        self.energy() > 0
    }
}

struct Rabbit {
    id: u32,
    energy: i32,
}

struct Fox {
    energy: i32,
    hunts: u32,
}

struct Plant {
    size: i32,
}

impl Creature for Rabbit {
    fn name(&self) -> String {
        format!("rabbit#{}", self.id)
    }
    fn energy(&self) -> i32 {
        self.energy
    }
    fn tick(&mut self, turn: u32) -> Option<String> {
        if turn % 2 == 0 {
            self.energy += 2;
            Some(format!("{} eats grass", self.name()))
        } else {
            self.energy -= 3;
            None
        }
    }
}

impl Creature for Fox {
    fn name(&self) -> String {
        "fox".to_string()
    }
    fn energy(&self) -> i32 {
        self.energy
    }
    fn tick(&mut self, turn: u32) -> Option<String> {
        self.energy -= 4;
        if turn % 3 == 0 {
            self.hunts += 1;
            self.energy += 7;
            Some(format!("fox hunts (total {})", self.hunts))
        } else {
            None
        }
    }
}

impl Creature for Plant {
    fn name(&self) -> String {
        "plant".to_string()
    }
    fn energy(&self) -> i32 {
        self.size
    }
    fn tick(&mut self, _turn: u32) -> Option<String> {
        self.size += 1;
        if self.size % 5 == 0 { Some(format!("plant grows to {}", self.size)) } else { None }
    }
}

fn main() {
    let mut world: Vec<Box<dyn Creature>> = vec![
        Box::new(Rabbit { id: 1, energy: 5 }),
        Box::new(Rabbit { id: 2, energy: 2 }),
        Box::new(Fox { energy: 10, hunts: 0 }),
        Box::new(Plant { size: 3 }),
    ];
    for turn in 1..=6 {
        println!("-- turn {} --", turn);
        for c in world.iter_mut() {
            if let Some(event) = c.tick(turn) {
                println!("  {}", event);
            }
        }
        let before = world.len();
        world.retain(|c| {
            if !c.alive() {
                println!("  {} dies", c.name());
            }
            c.alive()
        });
        let status: Vec<String> = world.iter().map(|c| format!("{}={}", c.name(), c.energy())).collect();
        println!("  [{}] removed {}", status.join(", "), before - world.len());
    }
    let strongest = world.iter().max_by_key(|c| c.energy()).map(|c| c.name());
    println!("strongest: {:?}", strongest);
}
