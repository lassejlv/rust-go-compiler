use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum Room {
    Hall,
    Library,
    Kitchen,
    Cellar,
    Garden,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Dir {
    North,
    South,
    East,
    West,
    Down,
    Up,
}

#[derive(Debug)]
enum Command {
    Go(Dir),
    Take(String),
    Use(String),
    Look,
    Inventory,
    Unknown(String),
}

fn parse(input: &str) -> Command {
    let words: Vec<&str> = input.split_whitespace().collect();
    match words.as_slice() {
        ["go", d] | [d] if parse_dir(d).is_some() => Command::Go(parse_dir(d).unwrap()),
        ["take", item] | ["get", item] => Command::Take(item.to_string()),
        ["use", item] => Command::Use(item.to_string()),
        ["look"] | ["l"] => Command::Look,
        ["inventory"] | ["i"] => Command::Inventory,
        _ => Command::Unknown(input.to_string()),
    }
}

fn parse_dir(s: &str) -> Option<Dir> {
    Some(match s {
        "north" | "n" => Dir::North,
        "south" | "s" => Dir::South,
        "east" | "e" => Dir::East,
        "west" | "w" => Dir::West,
        "down" | "d" => Dir::Down,
        "up" | "u" => Dir::Up,
        _ => return None,
    })
}

struct Game {
    room: Room,
    exits: HashMap<(Room, Dir), Room>,
    items: HashMap<Room, Vec<String>>,
    inventory: Vec<String>,
    cellar_unlocked: bool,
    score: u32,
    won: bool,
}

impl Game {
    fn new() -> Self {
        let mut exits = HashMap::new();
        let links = [
            (Room::Hall, Dir::North, Room::Library),
            (Room::Hall, Dir::East, Room::Kitchen),
            (Room::Kitchen, Dir::Down, Room::Cellar),
            (Room::Library, Dir::West, Room::Garden),
        ];
        for (a, d, b) in links {
            exits.insert((a, d), b);
            let back = match d {
                Dir::North => Dir::South,
                Dir::South => Dir::North,
                Dir::East => Dir::West,
                Dir::West => Dir::East,
                Dir::Down => Dir::Up,
                Dir::Up => Dir::Down,
            };
            exits.insert((b, back), a);
        }
        let mut items = HashMap::new();
        items.insert(Room::Library, vec!["book".to_string(), "key".to_string()]);
        items.insert(Room::Garden, vec!["shovel".to_string()]);
        items.insert(Room::Cellar, vec!["treasure".to_string()]);
        Game { room: Room::Hall, exits, items, inventory: vec![], cellar_unlocked: false, score: 0, won: false }
    }

    fn describe(&self) -> String {
        let here = self.items.get(&self.room).map(|v| v.join(", ")).unwrap_or_default();
        let mut dirs: Vec<String> = self
            .exits
            .keys()
            .filter(|(r, _)| *r == self.room)
            .map(|(_, d)| format!("{:?}", d).to_lowercase())
            .collect();
        dirs.sort();
        format!("{:?}: items [{}], exits [{}]", self.room, here, dirs.join(" "))
    }

    fn step(&mut self, cmd: Command) -> String {
        match cmd {
            Command::Go(d) => match self.exits.get(&(self.room, d)).copied() {
                Some(Room::Cellar) if !self.cellar_unlocked => "The cellar door is locked.".to_string(),
                Some(r) => {
                    self.room = r;
                    self.describe()
                }
                None => format!("You can't go {:?} from here.", d),
            },
            Command::Take(item) => {
                let list = self.items.entry(self.room).or_default();
                if let Some(pos) = list.iter().position(|i| *i == item) {
                    let it = list.remove(pos);
                    self.score += 10;
                    if it == "treasure" {
                        self.won = true;
                        self.score += 100;
                    }
                    self.inventory.push(it);
                    format!("Taken {}.", item)
                } else {
                    format!("No {} here.", item)
                }
            }
            Command::Use(item) if !self.inventory.contains(&item) => format!("You don't have {}.", item),
            Command::Use(item) => match (item.as_str(), self.room) {
                ("key", Room::Kitchen) => {
                    self.cellar_unlocked = true;
                    self.score += 25;
                    "Click! The cellar door unlocks.".to_string()
                }
                ("book", _) => "You read about hidden treasure below the kitchen.".to_string(),
                _ => format!("Nothing happens when you use {}.", item),
            },
            Command::Look => self.describe(),
            Command::Inventory => format!("You carry: {:?}", self.inventory),
            Command::Unknown(s) => format!("I don't understand '{}'.", s),
        }
    }
}

fn main() {
    let mut g = Game::new();
    println!("{}", g.describe());
    let script = [
        "go down", "e", "d", "w", "n", "take key", "take sword", "use book", "take book", "use book",
        "w", "get shovel", "e", "s", "east", "use key", "dance", "down", "look", "take treasure", "i", "up",
    ];
    for line in script.iter() {
        let cmd = parse(line);
        println!("> {}", line);
        println!("  {}", g.step(cmd));
        if g.won {
            println!("*** You found the treasure! ***");
            g.won = false;
        }
    }
    println!("final score: {}", g.score);
    let mut rooms: Vec<&Room> = g.items.keys().collect();
    rooms.sort();
    for r in rooms {
        println!("left in {:?}: {:?}", r, g.items[r]);
    }
}
