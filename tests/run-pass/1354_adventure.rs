// A text adventure engine: rooms graph, items with trait-object behaviors,
// a command parser, a state machine for NPC dialogue, scripted playthrough,
// save/load to a text format, and error chains via Box<dyn Error>.
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::error::Error;
use std::fmt;
use std::rc::Rc;
use std::str::FromStr;

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
    const ALL: [Dir; 6] = [Dir::North, Dir::South, Dir::East, Dir::West, Dir::Up, Dir::Down];
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

impl FromStr for Dir {
    type Err = GameError;
    fn from_str(s: &str) -> Result<Dir, GameError> {
        let l = s.to_ascii_lowercase();
        Dir::ALL.iter().copied().find(|d| {
            let n = format!("{:?}", d).to_lowercase();
            n == l || n[..1] == l
        }).ok_or_else(|| GameError::Parse(format!("unknown direction '{}'", s)))
    }
}

#[derive(Debug)]
enum GameError {
    Parse(String),
    NoExit(Dir),
    NoItem(String),
    Locked { door: String, needs: String },
    Save { line: usize, source: Box<dyn Error> },
}

impl fmt::Display for GameError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            GameError::Parse(m) => write!(f, "I don't understand: {}", m),
            GameError::NoExit(d) => write!(f, "You can't go {:?} from here.", d),
            GameError::NoItem(i) => write!(f, "There is no {} here.", i),
            GameError::Locked { door, needs } => write!(f, "The {} is locked. Perhaps a {} would help.", door, needs),
            GameError::Save { line, .. } => write!(f, "corrupt save file at line {}", line),
        }
    }
}

impl Error for GameError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            GameError::Save { source, .. } => Some(source.as_ref()),
            _ => None,
        }
    }
}

trait Item: fmt::Debug {
    fn name(&self) -> &str;
    fn weight(&self) -> u32 {
        1
    }
    fn on_use(&self, _state: &mut PlayerState) -> Option<String> {
        None
    }
    fn describe(&self) -> String {
        format!("a {}", self.name())
    }
}

#[derive(Debug)]
struct Key {
    opens: &'static str,
}
#[derive(Debug)]
struct Lamp {
    fuel: RefCell<u32>,
}
#[derive(Debug)]
struct Food {
    name: &'static str,
    heal: i32,
}
#[derive(Debug)]
struct Treasure {
    name: String,
    value: u32,
}

impl Item for Key {
    fn name(&self) -> &str {
        "key"
    }
    fn describe(&self) -> String {
        format!("a brass key engraved '{}'", self.opens)
    }
}
impl Item for Lamp {
    fn name(&self) -> &str {
        "lamp"
    }
    fn weight(&self) -> u32 {
        3
    }
    fn on_use(&self, st: &mut PlayerState) -> Option<String> {
        let mut f = self.fuel.borrow_mut();
        if *f == 0 {
            return Some("The lamp sputters; it's out of fuel.".into());
        }
        *f -= 1;
        st.light = !st.light;
        Some(format!("You turn the lamp {}. ({} fuel left)", if st.light { "on" } else { "off" }, f))
    }
}
impl Item for Food {
    fn name(&self) -> &str {
        self.name
    }
    fn on_use(&self, st: &mut PlayerState) -> Option<String> {
        st.health = (st.health + self.heal).min(100);
        Some(format!("You eat the {}. Health is now {}.", self.name, st.health))
    }
}
impl Item for Treasure {
    fn name(&self) -> &str {
        &self.name
    }
    fn weight(&self) -> u32 {
        self.value / 10
    }
    fn describe(&self) -> String {
        format!("a glittering {} (worth {} gold)", self.name, self.value)
    }
}

type ItemRef = Rc<dyn Item>;

struct Room {
    name: &'static str,
    desc: &'static str,
    dark: bool,
    exits: BTreeMap<Dir, (usize, Option<&'static str>)>, // target, lock name
    items: Vec<ItemRef>,
    npc: Option<Npc>,
}

#[derive(Debug, Default)]
struct PlayerState {
    room: usize,
    health: i32,
    light: bool,
    moves: u32,
    score: u32,
    visited: BTreeSet<usize>,
    flags: HashMap<&'static str, bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mood {
    Wary,
    Friendly,
    Trading,
    Gone,
}

struct Npc {
    name: &'static str,
    mood: Mood,
    wants: &'static str,
    gives: Option<ItemRef>,
}

impl Npc {
    fn talk(&mut self, has_gift: bool) -> (String, Option<ItemRef>) {
        let (next, line, gift) = match (self.mood, has_gift) {
            (Mood::Wary, _) => (Mood::Friendly, format!("{} eyes you suspiciously, then nods.", self.name), None),
            (Mood::Friendly, false) => (Mood::Friendly, format!("\"Bring me some {} and we'll talk,\" says {}.", self.wants, self.name), None),
            (Mood::Friendly, true) | (Mood::Trading, true) => (Mood::Trading, format!("{} brightens at the sight of the {}.", self.name, self.wants), None),
            (Mood::Trading, false) => (Mood::Friendly, "\"Changed your mind?\"".to_string(), None),
            (Mood::Gone, _) => (Mood::Gone, "Nobody is here.".to_string(), None),
        };
        self.mood = next;
        (line, gift)
    }
    fn trade(&mut self) -> Option<ItemRef> {
        if self.mood == Mood::Trading {
            self.mood = Mood::Gone;
            self.gives.take()
        } else {
            None
        }
    }
}

#[derive(Debug, PartialEq)]
enum Cmd {
    Go(Dir),
    Take(String),
    Drop(String),
    Use(String),
    Look,
    Inventory,
    Talk,
    Give(String),
    Save,
    Quit,
}

fn parse_cmd(line: &str) -> Result<Cmd, GameError> {
    let words: Vec<&str> = line.split_whitespace().filter(|w| !matches!(*w, "the" | "a" | "an" | "to")).collect();
    match words.as_slice() {
        [] => Err(GameError::Parse("say something".into())),
        ["go" | "walk", d] | [d @ ("n" | "s" | "e" | "w" | "u" | "d" | "north" | "south" | "east" | "west" | "up" | "down")] => Ok(Cmd::Go(d.parse()?)),
        ["take" | "get", rest @ ..] if !rest.is_empty() => Ok(Cmd::Take(rest.join(" "))),
        ["drop", rest @ ..] if !rest.is_empty() => Ok(Cmd::Drop(rest.join(" "))),
        ["use" | "eat" | "light", what] => Ok(Cmd::Use(what.to_string())),
        ["look" | "l"] => Ok(Cmd::Look),
        ["i" | "inv" | "inventory"] => Ok(Cmd::Inventory),
        ["talk", ..] => Ok(Cmd::Talk),
        ["give", what, ..] => Ok(Cmd::Give(what.to_string())),
        ["save"] => Ok(Cmd::Save),
        ["quit" | "q"] => Ok(Cmd::Quit),
        [verb, ..] => Err(GameError::Parse(format!("'{}'", verb))),
    }
}

struct Game {
    rooms: Vec<Room>,
    st: PlayerState,
    inv: Vec<ItemRef>,
    log: Vec<String>,
}

impl Game {
    fn new() -> Game {
        let mk = |name, desc, dark| Room { name, desc, dark, exits: BTreeMap::new(), items: vec![], npc: None };
        let mut rooms = vec![
            mk("Clearing", "A sunny clearing. A path leads north.", false),
            mk("Forest", "Tall pines. Paths go east and west; a cave mouth yawns below.", false),
            mk("Cave", "A damp cave. Something glints in the dark.", true),
            mk("Hut", "A cozy hut with a heavy oak door to the north.", false),
            mk("Vault", "The vault! Gold everywhere.", false),
            mk("River", "A rushing river. A fisherman sits on the bank.", false),
        ];
        let link = |rooms: &mut Vec<Room>, a: usize, d: Dir, b: usize, lock: Option<&'static str>| {
            rooms[a].exits.insert(d, (b, lock));
            rooms[b].exits.insert(d.opposite(), (a, None));
        };
        link(&mut rooms, 0, Dir::North, 1, None);
        link(&mut rooms, 1, Dir::Down, 2, None);
        link(&mut rooms, 1, Dir::East, 3, None);
        link(&mut rooms, 3, Dir::North, 4, Some("oak door"));
        link(&mut rooms, 1, Dir::West, 5, None);
        rooms[0].items.push(Rc::new(Lamp { fuel: RefCell::new(3) }));
        rooms[0].items.push(Rc::new(Food { name: "apple", heal: 10 }));
        rooms[2].items.push(Rc::new(Treasure { name: "ruby".into(), value: 120 }));
        rooms[2].items.push(Rc::new(Food { name: "worms", heal: -5 }));
        rooms[4].items.push(Rc::new(Treasure { name: "crown".into(), value: 500 }));
        rooms[5].npc = Some(Npc { name: "Old Pete", mood: Mood::Wary, wants: "worms", gives: Some(Rc::new(Key { opens: "oak door" })) });
        let mut st = PlayerState { health: 50, ..Default::default() };
        st.visited.insert(0);
        Game { rooms, st, inv: vec![], log: vec![] }
    }

    fn here(&self) -> &Room {
        &self.rooms[self.st.room]
    }

    fn look(&self) -> String {
        let r = self.here();
        if r.dark && !self.st.light {
            return "It is pitch dark. You might be eaten by a grue.".into();
        }
        let mut s = format!("== {} ==\n{}", r.name, r.desc);
        if !r.items.is_empty() {
            let names: Vec<String> = r.items.iter().map(|i| i.describe()).collect();
            s += &format!("\nYou see: {}.", names.join(", "));
        }
        if let Some(n) = r.npc.as_ref().filter(|n| n.mood != Mood::Gone) {
            s += &format!("\n{} is here ({:?}).", n.name, n.mood);
        }
        let exits: Vec<String> = r.exits.iter().map(|(d, (_, lock))| format!("{:?}{}", d, if lock.is_some() { "*" } else { "" })).collect();
        s + &format!("\nExits: {}", exits.join(" "))
    }

    fn take_from(list: &mut Vec<ItemRef>, name: &str) -> Option<ItemRef> {
        let pos = list.iter().position(|i| i.name().eq_ignore_ascii_case(name))?;
        Some(list.remove(pos))
    }

    fn step(&mut self, cmd: Cmd) -> Result<String, GameError> {
        self.st.moves += 1;
        match cmd {
            Cmd::Go(d) => {
                let &(to, lock) = self.here().exits.get(&d).ok_or(GameError::NoExit(d))?;
                if let Some(door) = lock {
                    let has_key = self.inv.iter().any(|i| i.describe().contains(door));
                    if !has_key {
                        return Err(GameError::Locked { door: door.to_string(), needs: "key".into() });
                    }
                }
                if self.here().dark && !self.st.light && d != Dir::Up {
                    self.st.health -= 15;
                    return Ok(format!("You stumble in the dark and hurt yourself. Health {}.", self.st.health));
                }
                self.st.room = to;
                if self.st.visited.insert(to) {
                    self.st.score += 5;
                }
                Ok(self.look())
            }
            Cmd::Take(name) => {
                if self.here().dark && !self.st.light {
                    return Err(GameError::NoItem(name));
                }
                let room = self.st.room;
                let it = Self::take_from(&mut self.rooms[room].items, &name).ok_or(GameError::NoItem(name))?;
                let load: u32 = self.inv.iter().map(|i| i.weight()).sum();
                if load + it.weight() > 60 {
                    let n = it.name().to_string();
                    self.rooms[room].items.push(it);
                    return Ok(format!("The {} is too heavy (carrying {}).", n, load));
                }
                let msg = format!("Taken: {}.", it.describe());
                self.inv.push(it);
                Ok(msg)
            }
            Cmd::Drop(name) => {
                let it = Self::take_from(&mut self.inv, &name).ok_or_else(|| GameError::NoItem(name.clone()))?;
                let msg = format!("Dropped {}.", it.name());
                let room = self.st.room;
                self.rooms[room].items.push(it);
                Ok(msg)
            }
            Cmd::Use(name) => {
                let it = self.inv.iter().find(|i| i.name() == name).cloned().ok_or(GameError::NoItem(name.clone()))?;
                let msg = it.on_use(&mut self.st).unwrap_or_else(|| format!("Nothing happens with the {}.", it.name()));
                if it.weight() == 1 && msg.starts_with("You eat") {
                    Self::take_from(&mut self.inv, &name);
                }
                Ok(msg)
            }
            Cmd::Look => Ok(self.look()),
            Cmd::Inventory => {
                if self.inv.is_empty() {
                    return Ok("You carry nothing.".into());
                }
                let mut v: Vec<(String, u32)> = self.inv.iter().map(|i| (i.name().to_string(), i.weight())).collect();
                v.sort();
                Ok(v.iter().map(|(n, w)| format!("{}({})", n, w)).collect::<Vec<_>>().join(", "))
            }
            Cmd::Talk => {
                let wants = self.here().npc.as_ref().map(|n| n.wants);
                let has = wants.map_or(false, |w| self.inv.iter().any(|i| i.name() == w));
                let room = self.st.room;
                match self.rooms[room].npc.as_mut() {
                    Some(n) => Ok(n.talk(has).0),
                    None => Ok("You talk to yourself.".into()),
                }
            }
            Cmd::Give(what) => {
                let room = self.st.room;
                let npc = self.rooms[room].npc.as_mut().ok_or_else(|| GameError::Parse("give to whom?".into()))?;
                if npc.wants != what {
                    return Ok(format!("{} doesn't want that.", npc.name));
                }
                match npc.trade() {
                    Some(gift) => {
                        Self::take_from(&mut self.inv, &what);
                        let d = gift.describe();
                        self.inv.push(gift);
                        self.st.flags.insert("traded", true);
                        Ok(format!("You hand over the {}. In return you receive {}.", what, d))
                    }
                    None => Ok("Not yet...".into()),
                }
            }
            Cmd::Save => Ok(self.save()),
            Cmd::Quit => Ok(format!("Final score: {}", self.final_score())),
        }
    }

    fn final_score(&self) -> u32 {
        self.st.score + self.inv.iter().filter_map(|i| i.describe().split("worth ").nth(1).and_then(|r| r.split(' ').next()).and_then(|n| n.parse::<u32>().ok())).sum::<u32>()
    }

    fn save(&self) -> String {
        let mut out = format!("room={}\nhealth={}\nmoves={}\nscore={}\n", self.st.room, self.st.health, self.st.moves, self.st.score);
        out += &format!("visited={}\n", self.st.visited.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(","));
        out += &format!("inv={}", self.inv.iter().map(|i| i.name()).collect::<Vec<_>>().join(","));
        out
    }
}

fn load_state(text: &str) -> Result<PlayerState, GameError> {
    let mut st = PlayerState::default();
    for (i, line) in text.lines().enumerate() {
        let wrap = |e: Box<dyn Error>| GameError::Save { line: i + 1, source: e };
        let (k, v) = line.split_once('=').ok_or_else(|| wrap("missing '='".into()))?;
        match k {
            "room" => st.room = v.parse().map_err(|e: std::num::ParseIntError| wrap(e.into()))?,
            "health" => st.health = v.parse().map_err(|e: std::num::ParseIntError| wrap(Box::new(e)))?,
            "moves" => st.moves = v.parse().map_err(|e| wrap(Box::new(e)))?,
            "score" => st.score = v.parse().map_err(|e| wrap(Box::new(e)))?,
            "visited" => st.visited = v.split(',').map(|x| x.parse::<usize>()).collect::<Result<_, _>>().map_err(|e| wrap(Box::new(e)))?,
            "inv" => {}
            other => return Err(wrap(format!("unknown key {:?}", other).into())),
        }
    }
    Ok(st)
}

fn main() {
    let mut g = Game::new();
    println!("{}", g.look());
    let script = [
        "take lamp", "take apple", "eat apple", "n", "go down", "take ruby", "light lamp", "look", "take the ruby", "take worms", "up",
        "go up", "w", "talk", "talk", "give worms to pete", "talk", "give worms", "inventory", "e", "e", "n", "take crown", "fly",
        "go sideways", "drop lamp", "drop sword", "", "i", "save", "quit",
    ];
    for line in script {
        let res = parse_cmd(line).and_then(|c| g.step(c));
        match res {
            Ok(msg) => println!("> {}\n{}", line, msg),
            Err(e) => println!("> {}\n!! {}", line, e),
        }
    }
    let saved = g.save();
    let st = load_state(&saved).expect("reload");
    println!("reloaded: room={} health={} moves={} score={} visited={:?}", st.room, st.health, st.moves, st.score, st.visited);
    for bad in ["room=1\nhealth=abc", "room=1\nvisited=1,x", "nonsense", "room=2\ncolor=blue"] {
        match load_state(bad) {
            Ok(_) => println!("loaded?!"),
            Err(e) => {
                let mut chain = vec![e.to_string()];
                let mut src = e.source();
                while let Some(s) = src {
                    chain.push(s.to_string());
                    src = s.source();
                }
                println!("load error: {}", chain.join(" <- "));
            }
        }
    }
    println!("{:?}", Dir::ALL.iter().map(|d| d.opposite()).collect::<Vec<_>>());
    println!("{:?} {:?}", "N".parse::<Dir>().map_err(|e| e.to_string()), "x".parse::<Dir>().map_err(|e| e.to_string()));
    let mut tally: BTreeMap<String, usize> = BTreeMap::new();
    for line in script {
        let k = match parse_cmd(line) {
            Ok(c) => format!("{:?}", c).split(|c: char| !c.is_alphanumeric()).next().unwrap_or("").to_string(),
            Err(_) => "error".into(),
        };
        *tally.entry(k).or_default() += 1;
    }
    println!("{:?}", tally);
}
