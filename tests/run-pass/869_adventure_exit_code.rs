use std::collections::BTreeMap;
use std::fmt;
use std::process::ExitCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Dir { N, S, E, W }
impl std::str::FromStr for Dir {
    type Err = String;
    fn from_str(s: &str) -> Result<Dir, String> { match s { "n" | "north" => Ok(Dir::N), "s" | "south" => Ok(Dir::S), "e" | "east" => Ok(Dir::E), "w" | "west" => Ok(Dir::W), _ => Err(format!("'{}' is not a direction", s)) } }
}

#[derive(Debug)]
enum Cmd { Go(Dir), Take(String), Drop(String), Use(String, Option<String>), Look, Inventory, Quit }

fn parse(line: &str) -> Result<Cmd, String> {
    let words: Vec<&str> = line.split_whitespace().filter(|w| !matches!(*w, "the" | "a" | "an")).collect();
    match words.as_slice() {
        [] => Err("say something".into()),
        ["go", d] | [d @ ("n" | "s" | "e" | "w" | "north" | "south" | "east" | "west")] if true => d.parse().map(Cmd::Go),
        ["take" | "get", item] => Ok(Cmd::Take(item.to_string())),
        ["drop", item] => Ok(Cmd::Drop(item.to_string())),
        ["use", item] => Ok(Cmd::Use(item.to_string(), None)),
        ["use", item, "on" | "with", target] => Ok(Cmd::Use(item.to_string(), Some(target.to_string()))),
        ["look"] | ["l"] => Ok(Cmd::Look),
        ["i"] | ["inventory"] => Ok(Cmd::Inventory),
        ["quit"] => Ok(Cmd::Quit),
        [verb, ..] => Err(format!("I don't know how to '{}'", verb)),
    }
}

struct Room { name: &'static str, items: Vec<String>, exits: BTreeMap<Dir, usize>, locked: Option<(Dir, &'static str)> }
struct Game { rooms: Vec<Room>, here: usize, inv: Vec<String>, moves: u32, score: i32 }

impl fmt::Display for Game {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let r = &self.rooms[self.here];
        write!(f, "[{}] items: {:?}; exits: {:?}", r.name, r.items, r.exits.keys().collect::<Vec<_>>())
    }
}

impl Game {
    fn step(&mut self, cmd: Cmd) -> Result<String, String> {
        self.moves += 1;
        let here = self.here;
        match cmd {
            Cmd::Go(d) => {
                if let Some((ld, key)) = self.rooms[here].locked { if ld == d { return Err(format!("the way {:?} is locked (needs {})", d, key)); } }
                let to = *self.rooms[here].exits.get(&d).ok_or_else(|| format!("no exit {:?}", d))?;
                self.here = to;
                Ok(format!("you walk {:?} to {}", d, self.rooms[to].name))
            }
            Cmd::Take(item) => {
                let pos = self.rooms[here].items.iter().position(|i| *i == item).ok_or(format!("no {} here", item))?;
                let it = self.rooms[here].items.remove(pos);
                self.score += if it == "gem" { 50 } else { 5 };
                self.inv.push(it);
                Ok(format!("taken {}", item))
            }
            Cmd::Drop(item) => {
                let pos = self.inv.iter().position(|i| *i == item).ok_or(format!("you have no {}", item))?;
                let it = self.inv.swap_remove(pos);
                self.rooms[here].items.push(it);
                Ok("dropped".into())
            }
            Cmd::Use(item, target) => {
                if !self.inv.contains(&item) { return Err(format!("you have no {}", item)); }
                match (&self.rooms[here].locked, target.as_deref()) {
                    (Some((_, key)), Some("door") | None) if *key == item => { self.rooms[here].locked = None; self.score += 20; Ok(format!("the {} unlocks the door", item)) }
                    _ => Err(format!("nothing happens with {}", item)),
                }
            }
            Cmd::Look => Ok(self.to_string()),
            Cmd::Inventory => Ok(format!("carrying {:?}", self.inv)),
            Cmd::Quit => Err("quit".into()),
        }
    }
}

fn main() -> ExitCode {
    let mut rooms = vec![
        Room { name: "hall", items: vec!["lamp".into()], exits: BTreeMap::from([(Dir::N, 1), (Dir::E, 2)]), locked: Some((Dir::E, "key")) },
        Room { name: "library", items: vec!["key".into(), "book".into()], exits: BTreeMap::from([(Dir::S, 0)]), locked: None },
        Room { name: "vault", items: vec!["gem".into()], exits: BTreeMap::from([(Dir::W, 0)]), locked: None },
    ];
    rooms[0].items.sort();
    let mut g = Game { rooms, here: 0, inv: vec![], moves: 0, score: 0 };
    let script = ["look", "go east", "take the lamp", "n", "take key", "take sword", "dance wildly", "s", "use key on door", "e", "get gem", "drop lamp", "look", "i", "go up", "", "w", "quit", "look"];
    for line in script {
        let res = parse(line).and_then(|c| g.step(c));
        match res {
            Ok(msg) => println!("> {:<16} {}", line, msg),
            Err(e) if e == "quit" => { println!("> {:<16} bye after {} moves, score {}", line, g.moves, g.score); break; }
            Err(e) => println!("> {:<16} ! {}", line, e),
        }
    }
    let won = g.inv.iter().any(|i| i == "gem");
    println!("won={} final: {}", won, g);
    if won { ExitCode::from(7) } else { ExitCode::FAILURE }
}
