// Text adventure state machine driven by a scripted list of commands.
use std::collections::{BTreeSet, HashMap, VecDeque};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum Room {
    Cellar,
    Hall,
    Library,
    Kitchen,
    Garden,
    Tower,
    Vault,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum Dir {
    North,
    South,
    East,
    West,
    Up,
    Down,
}

impl Dir {
    fn parse(s: &str) -> Option<Dir> {
        Some(match s {
            "n" | "north" => Dir::North,
            "s" | "south" => Dir::South,
            "e" | "east" => Dir::East,
            "w" | "west" => Dir::West,
            "u" | "up" => Dir::Up,
            "d" | "down" => Dir::Down,
            _ => return None,
        })
    }
    fn opposite(self) -> Dir {
        match self {
            Dir::North => Dir::South,
            Dir::South => Dir::North,
            Dir::East => Dir::West,
            Dir::West => Dir::East,
            Dir::Up => Dir::Down,
            Dir::Down => Dir::Up,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum Item {
    Lamp,
    Key,
    Book,
    Bread,
    Rope,
    Gem,
    Coin(u8),
}

impl fmt::Display for Item {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Item::Coin(n) => write!(f, "coin #{}", n),
            other => write!(f, "{}", format!("{:?}", other).to_lowercase()),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Cmd {
    Go(Dir),
    Take(String),
    Drop(String),
    Use(String),
    Read,
    Eat,
    Look,
    Inventory,
    Path(Room),
    Quit,
    Unknown(String),
}

fn parse_cmd(line: &str) -> Cmd {
    let words: Vec<&str> = line.split_whitespace().map(|w| w.trim_matches(|c: char| !c.is_alphanumeric())).filter(|w| !w.is_empty()).collect();
    let lower: Vec<String> = words.iter().map(|w| w.to_lowercase()).collect();
    let rest = || lower[1..].join(" ");
    match lower.first().map(|s| s.as_str()) {
        Some("go") | Some("walk") if lower.len() > 1 => match Dir::parse(&lower[1]) {
            Some(d) => Cmd::Go(d),
            None => Cmd::Unknown(line.to_string()),
        },
        Some(w) if Dir::parse(w).is_some() && lower.len() == 1 => Cmd::Go(Dir::parse(w).unwrap()),
        Some("take") | Some("get") | Some("grab") => Cmd::Take(rest()),
        Some("drop") => Cmd::Drop(rest()),
        Some("use") => Cmd::Use(rest()),
        Some("read") => Cmd::Read,
        Some("eat") => Cmd::Eat,
        Some("look") | Some("l") => Cmd::Look,
        Some("i") | Some("inv") | Some("inventory") => Cmd::Inventory,
        Some("path") if lower.len() > 1 => match lower[1].as_str() {
            "cellar" => Cmd::Path(Room::Cellar),
            "hall" => Cmd::Path(Room::Hall),
            "library" => Cmd::Path(Room::Library),
            "kitchen" => Cmd::Path(Room::Kitchen),
            "garden" => Cmd::Path(Room::Garden),
            "tower" => Cmd::Path(Room::Tower),
            "vault" => Cmd::Path(Room::Vault),
            _ => Cmd::Unknown(line.to_string()),
        },
        Some("quit") => Cmd::Quit,
        _ => Cmd::Unknown(line.to_string()),
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum State {
    Playing,
    Won,
    Dead(&'static str),
    Quit,
}

struct World {
    exits: HashMap<(Room, Dir), Room>,
    items: HashMap<Room, Vec<Item>>,
    locked: BTreeSet<(Room, Dir)>,
    dark: BTreeSet<Room>,
}

struct Game {
    world: World,
    here: Room,
    inv: Vec<Item>,
    hunger: u32,
    moves: u32,
    score: i32,
    visited: BTreeSet<Room>,
    state: State,
    lamp_on: bool,
    log: Vec<String>,
}

impl World {
    fn build() -> World {
        let mut exits = HashMap::new();
        let mut link = |a: Room, d: Dir, b: Room| {
            exits.insert((a, d), b);
            exits.insert((b, d.opposite()), a);
        };
        link(Room::Cellar, Dir::Up, Room::Hall);
        link(Room::Hall, Dir::East, Room::Library);
        link(Room::Hall, Dir::West, Room::Kitchen);
        link(Room::Hall, Dir::North, Room::Garden);
        link(Room::Garden, Dir::Up, Room::Tower);
        link(Room::Library, Dir::Down, Room::Vault);
        let mut items = HashMap::new();
        items.insert(Room::Cellar, vec![Item::Lamp, Item::Coin(1)]);
        items.insert(Room::Kitchen, vec![Item::Bread, Item::Coin(2)]);
        items.insert(Room::Library, vec![Item::Book]);
        items.insert(Room::Garden, vec![Item::Rope, Item::Coin(3)]);
        items.insert(Room::Tower, vec![Item::Key]);
        items.insert(Room::Vault, vec![Item::Gem, Item::Coin(4), Item::Coin(5)]);
        let locked = [(Room::Library, Dir::Down), (Room::Garden, Dir::Up)].into_iter().collect();
        let dark = [Room::Cellar, Room::Vault].into_iter().collect();
        World { exits, items, locked, dark }
    }

    fn bfs(&self, from: Room, to: Room) -> Option<Vec<Dir>> {
        let mut q = VecDeque::new();
        let mut prev: HashMap<Room, (Room, Dir)> = HashMap::new();
        q.push_back(from);
        let mut seen = BTreeSet::new();
        seen.insert(from);
        while let Some(r) = q.pop_front() {
            if r == to {
                let mut path = Vec::new();
                let mut cur = to;
                while cur != from {
                    let (p, d) = prev[&cur];
                    path.push(d);
                    cur = p;
                }
                path.reverse();
                return Some(path);
            }
            let mut ds = [Dir::North, Dir::South, Dir::East, Dir::West, Dir::Up, Dir::Down];
            ds.sort();
            for d in ds {
                if let Some(&n) = self.exits.get(&(r, d)) {
                    if !self.locked.contains(&(r, d)) && seen.insert(n) {
                        prev.insert(n, (r, d));
                        q.push_back(n);
                    }
                }
            }
        }
        None
    }
}

impl Game {
    fn say(&mut self, s: impl Into<String>) {
        let s = s.into();
        println!("  {}", s);
        self.log.push(s);
    }

    fn can_see(&self) -> bool {
        !self.world.dark.contains(&self.here) || (self.lamp_on && self.inv.contains(&Item::Lamp))
    }

    fn find_item(list: &[Item], name: &str) -> Option<usize> {
        list.iter().position(|it| {
            let s = it.to_string();
            s == name || (name == "coin" && matches!(it, Item::Coin(_))) || s.starts_with(name)
        })
    }

    fn describe(&mut self) {
        let here = self.here;
        if !self.can_see() {
            self.say(format!("{:?}: It is pitch dark. You might be eaten by a grue.", here));
            return;
        }
        let items: Vec<String> = self.world.items.get(&here).map(|v| v.iter().map(|i| i.to_string()).collect()).unwrap_or_default();
        let mut exits: Vec<String> = self
            .world
            .exits
            .keys()
            .filter(|(r, _)| *r == here)
            .map(|(_, d)| {
                let lk = if self.world.locked.contains(&(here, *d)) { "*" } else { "" };
                format!("{:?}{}", d, lk).to_lowercase()
            })
            .collect();
        exits.sort();
        self.say(format!(
            "{:?}. Exits: {}. Items: {}",
            here,
            exits.join(", "),
            if items.is_empty() { "none".to_string() } else { items.join(", ") }
        ));
    }

    fn step(&mut self, cmd: Cmd) {
        self.moves += 1;
        match cmd {
            Cmd::Go(d) => {
                if !self.can_see() && self.moves % 3 == 0 {
                    self.state = State::Dead("eaten by a grue");
                    self.say("Something grabs you in the dark!");
                    return;
                }
                match self.world.exits.get(&(self.here, d)).copied() {
                    None => self.say("You can't go that way."),
                    Some(_) if self.world.locked.contains(&(self.here, d)) => self.say("The way is locked."),
                    Some(r) => {
                        self.here = r;
                        self.hunger += 1;
                        if self.visited.insert(r) {
                            self.score += 5;
                        }
                        self.describe();
                    }
                }
            }
            Cmd::Take(name) => {
                if !self.can_see() && name != "lamp" {
                    return self.say("You fumble around in the dark.");
                }
                let here = self.here;
                let list = self.world.items.entry(here).or_default();
                match Game::find_item(list, &name) {
                    Some(i) => {
                        let it = list.remove(i);
                        if let Item::Coin(_) = it {
                            self.score += 10;
                        }
                        if it == Item::Gem {
                            self.score += 50;
                        }
                        self.inv.push(it);
                        self.say(format!("Taken: {}.", it));
                    }
                    None => self.say(format!("No {} here.", name)),
                }
            }
            Cmd::Drop(name) => match Game::find_item(&self.inv, &name) {
                Some(i) => {
                    let it = self.inv.swap_remove(i);
                    self.world.items.entry(self.here).or_default().push(it);
                    self.say(format!("Dropped {}.", it));
                }
                None => self.say("You don't have that."),
            },
            Cmd::Use(name) => {
                let it = Game::find_item(&self.inv, &name).map(|i| self.inv[i]);
                match (it, self.here) {
                    (Some(Item::Lamp), _) => {
                        self.lamp_on = !self.lamp_on;
                        let s = if self.lamp_on { "on" } else { "off" };
                        self.say(format!("The lamp is now {}.", s));
                    }
                    (Some(Item::Rope), Room::Garden) => {
                        self.world.locked.remove(&(Room::Garden, Dir::Up));
                        self.say("You throw the rope up to the tower window.");
                    }
                    (Some(Item::Key), Room::Library) => {
                        self.world.locked.remove(&(Room::Library, Dir::Down));
                        self.say("The trapdoor unlocks with a click.");
                    }
                    (Some(it), r) => self.say(format!("Using the {} in the {:?} does nothing.", it, r)),
                    (None, _) => self.say("You don't have that."),
                }
            }
            Cmd::Read => {
                if self.inv.contains(&Item::Book) {
                    self.say("The book reads: 'The key lies above the garden.'");
                } else {
                    self.say("Nothing to read.");
                }
            }
            Cmd::Eat => match self.inv.iter().position(|&i| i == Item::Bread) {
                Some(i) => {
                    self.inv.remove(i);
                    self.hunger = 0;
                    self.say("Delicious.");
                }
                None => self.say("You have nothing to eat."),
            },
            Cmd::Look => self.describe(),
            Cmd::Inventory => {
                let mut v = self.inv.clone();
                v.sort();
                let names: Vec<String> = v.iter().map(Item::to_string).collect();
                self.say(format!("You carry: [{}]", names.join("; ")));
            }
            Cmd::Path(to) => match self.world.bfs(self.here, to) {
                Some(p) if p.is_empty() => self.say("You are already there."),
                Some(p) => self.say(format!("Route: {:?}", p)),
                None => self.say(format!("No known route to {:?}.", to)),
            },
            Cmd::Quit => self.state = State::Quit,
            Cmd::Unknown(s) => {
                self.moves -= 1;
                self.say(format!("I don't understand {:?}.", s));
            }
        }
        if self.hunger > 12 && self.state == State::Playing {
            self.state = State::Dead("starved");
            self.say("You collapse from hunger.");
        }
        if self.here == Room::Hall && self.inv.contains(&Item::Gem) && self.state == State::Playing {
            self.state = State::Won;
            self.score += 100;
            self.say("You escape with the gem!");
        }
    }
}

fn play(name: &str, script: &[&str]) {
    println!("--- {} ---", name);
    let mut g = Game {
        world: World::build(),
        here: Room::Cellar,
        inv: vec![],
        hunger: 0,
        moves: 0,
        score: 0,
        visited: [Room::Cellar].into_iter().collect(),
        state: State::Playing,
        lamp_on: false,
        log: vec![],
    };
    g.describe();
    for line in script {
        if g.state != State::Playing {
            break;
        }
        println!("> {}", line);
        let c = parse_cmd(line);
        g.step(c);
    }
    let coins = g.inv.iter().filter(|i| matches!(i, Item::Coin(_))).count();
    println!(
        "result: {:?}, score {}, moves {}, hunger {}, coins {}, visited {:?}, messages {}",
        g.state, g.score, g.moves, g.hunger, coins, g.visited, g.log.len()
    );
}

fn main() {
    play(
        "winning run",
        &[
            "take lamp", "use lamp", "take lamp", "take coin", "look", "up", "west", "take bread", "take coin",
            "go east", "east", "take book", "read", "go down", "path tower", "west", "north", "take rope",
            "use rope", "path tower", "up", "take key", "down", "south", "east", "use key", "eat", "d", "look",
            "take gem", "grab coin", "get coin", "i", "path hall", "u", "dance wildly", "w",
        ],
    );
    play("grue", &["up", "down", "take lamp", "north", "north", "north", "north"]);
    play("starve", &["take lamp", "use lamp", "up", "e", "w", "w", "e", "n", "s", "n", "s", "n", "s", "n", "s", "n", "s", "n"]);
    play("quitter", &["inventory", "drop lamp", "UP", "Look!", "path vault", "quit", "look"]);
}
