// Inventory & crafting simulation: generic containers, trait-based item kinds,
// recipes with generic requirements, error enums with From conversions.
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Display};
use std::ops::{Add, AddAssign, Sub};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Material { Wood, Stone, Iron, Gold, Plank, Stick, Pickaxe, Ring }

impl Material {
    const ALL: [Material; 8] = [Material::Wood, Material::Stone, Material::Iron, Material::Gold, Material::Plank, Material::Stick, Material::Pickaxe, Material::Ring];
    fn weight(self) -> Weight { Weight(match self { Material::Wood => 3, Material::Stone => 5, Material::Iron => 8, Material::Gold => 10, Material::Plank => 1, Material::Stick => 1, Material::Pickaxe => 12, Material::Ring => 1 }) }
    fn value(self) -> Coins { Coins(match self { Material::Gold => 50, Material::Ring => 400, Material::Pickaxe => 90, Material::Iron => 12, _ => 2 }) }
}
impl Display for Material { fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { f.pad(&format!("{:?}", self).to_lowercase()) } }

macro_rules! quantity {
    ($name:ident, $unit:expr) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
        struct $name(u32);
        impl Add for $name { type Output = $name; fn add(self, o: $name) -> $name { $name(self.0 + o.0) } }
        impl Sub for $name { type Output = $name; fn sub(self, o: $name) -> $name { $name(self.0 - o.0) } }
        impl AddAssign for $name { fn add_assign(&mut self, o: $name) { self.0 += o.0 } }
        impl std::ops::Mul<u32> for $name { type Output = $name; fn mul(self, k: u32) -> $name { $name(self.0 * k) } }
        impl std::iter::Sum for $name { fn sum<I: Iterator<Item = $name>>(it: I) -> $name { it.fold($name(0), |a, b| a + b) } }
        impl Display for $name { fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "{}{}", self.0, $unit) } }
    };
}
quantity!(Weight, "kg");
quantity!(Coins, "c");

trait Stackable: Copy + Ord + Display {
    fn max_stack(&self) -> u32;
    fn weight_of(&self) -> Weight;
}
impl Stackable for Material {
    fn max_stack(&self) -> u32 { match self { Material::Pickaxe | Material::Ring => 1, Material::Gold => 16, _ => 64 } }
    fn weight_of(&self) -> Weight { self.weight() }
}

#[derive(Debug, PartialEq)]
enum InvError<T> { Missing { item: T, need: u32, have: u32 }, TooHeavy { limit: Weight, would_be: Weight }, NoSlots(usize) }

impl<T: Display> Display for InvError<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            InvError::Missing { item, need, have } => write!(f, "missing {}: need {} have {}", item, need, have),
            InvError::TooHeavy { limit, would_be } => write!(f, "too heavy: {} > {}", would_be, limit),
            InvError::NoSlots(n) => write!(f, "no free slots ({} used)", n),
        }
    }
}

#[derive(Debug, Clone)]
struct Inventory<T: Stackable, const SLOTS: usize> {
    slots: Vec<(T, u32)>,
    max_weight: Weight,
}

impl<T: Stackable, const SLOTS: usize> Inventory<T, SLOTS> {
    fn new(max_weight: Weight) -> Self { Inventory { slots: Vec::new(), max_weight } }

    fn weight(&self) -> Weight { self.slots.iter().map(|(t, n)| t.weight_of() * *n).sum() }

    fn count(&self, t: T) -> u32 { self.slots.iter().filter(|(x, _)| *x == t).map(|(_, n)| n).sum() }

    fn add(&mut self, t: T, mut n: u32) -> Result<(), InvError<T>> {
        let would_be = self.weight() + t.weight_of() * n;
        if would_be > self.max_weight { return Err(InvError::TooHeavy { limit: self.max_weight, would_be }); }
        // simulate on a copy first so failure leaves inventory unchanged
        let mut slots = self.slots.clone();
        for (x, c) in slots.iter_mut() {
            if *x == t && *c < t.max_stack() {
                let take = (t.max_stack() - *c).min(n);
                *c += take;
                n -= take;
            }
        }
        while n > 0 {
            if slots.len() >= SLOTS { return Err(InvError::NoSlots(slots.len())); }
            let take = t.max_stack().min(n);
            slots.push((t, take));
            n -= take;
        }
        self.slots = slots;
        Ok(())
    }

    fn remove(&mut self, t: T, n: u32) -> Result<(), InvError<T>> {
        let have = self.count(t);
        if have < n { return Err(InvError::Missing { item: t, need: n, have }); }
        let mut left = n;
        for (x, c) in self.slots.iter_mut().rev() {
            if *x == t && left > 0 { let take = (*c).min(left); *c -= take; left -= take; }
        }
        self.slots.retain(|(_, c)| *c > 0);
        Ok(())
    }

    fn summary(&self) -> BTreeMap<T, u32> {
        let mut m = BTreeMap::new();
        for (t, n) in &self.slots { *m.entry(*t).or_insert(0) += n; }
        m
    }
}

impl<T: Stackable, const SLOTS: usize> Display for Inventory<T, SLOTS> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[{}/{} slots, {}/{}]", self.slots.len(), SLOTS, self.weight(), self.max_weight)?;
        for (t, n) in self.summary() { write!(f, " {}x{}", t, n)?; }
        Ok(())
    }
}

struct Recipe<T> { name: &'static str, inputs: Vec<(T, u32)>, output: (T, u32), tool: Option<T> }

#[derive(Debug)]
enum CraftError<T> { Inv(InvError<T>), NeedTool(T), Unknown(String) }
impl<T> From<InvError<T>> for CraftError<T> { fn from(e: InvError<T>) -> Self { CraftError::Inv(e) } }
impl<T: Display> Display for CraftError<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self { CraftError::Inv(e) => write!(f, "{}", e), CraftError::NeedTool(t) => write!(f, "requires tool {}", t), CraftError::Unknown(s) => write!(f, "unknown recipe {:?}", s) }
    }
}

struct Crafter<'r, T> { recipes: BTreeMap<&'static str, &'r Recipe<T>> }

impl<'r, T: Stackable> Crafter<'r, T> {
    fn new(rs: &'r [Recipe<T>]) -> Self { Crafter { recipes: rs.iter().map(|r| (r.name, r)).collect() } }

    fn craft<const S: usize>(&self, inv: &mut Inventory<T, S>, name: &str, times: u32) -> Result<u32, CraftError<T>> {
        let r = self.recipes.get(name).ok_or_else(|| CraftError::Unknown(name.to_string()))?;
        if let Some(tool) = r.tool { if inv.count(tool) == 0 { return Err(CraftError::NeedTool(tool)); } }
        let backup = inv.clone();
        let result = (|| -> Result<u32, CraftError<T>> {
            for &(t, n) in &r.inputs { inv.remove(t, n * times)?; }
            inv.add(r.output.0, r.output.1 * times)?;
            Ok(r.output.1 * times)
        })();
        if result.is_err() { *inv = backup; }
        result
    }

    fn craftable<const S: usize>(&self, inv: &Inventory<T, S>) -> BTreeSet<&'static str> {
        self.recipes.values().filter(|r| r.inputs.iter().all(|&(t, n)| inv.count(t) >= n) && r.tool.map_or(true, |t| inv.count(t) > 0)).map(|r| r.name).collect()
    }
}

fn worth<T: Stackable, const S: usize>(inv: &Inventory<T, S>, price: impl Fn(T) -> Coins) -> Coins {
    inv.summary().into_iter().map(|(t, n)| price(t) * n).sum()
}

fn main() {
    use Material::*;
    let recipes = vec![
        Recipe { name: "planks", inputs: vec![(Wood, 1)], output: (Plank, 4), tool: None },
        Recipe { name: "sticks", inputs: vec![(Plank, 2)], output: (Stick, 4), tool: None },
        Recipe { name: "pickaxe", inputs: vec![(Stick, 2), (Iron, 3)], output: (Pickaxe, 1), tool: None },
        Recipe { name: "ring", inputs: vec![(Gold, 2)], output: (Ring, 1), tool: Some(Pickaxe) },
    ];
    let crafter = Crafter::new(&recipes);
    let mut inv: Inventory<Material, 6> = Inventory::new(Weight(150));
    for (m, n) in [(Wood, 5), (Iron, 4), (Gold, 3), (Stone, 2)] {
        match inv.add(m, n) { Ok(()) => println!("added {} {}", n, m), Err(e) => println!("add {} {}: {}", n, m, e) }
    }
    println!("{}", inv);
    println!("can craft: {:?}", crafter.craftable(&inv));
    let plan = [("ring", 1), ("planks", 2), ("sticks", 1), ("pickaxe", 1), ("ring", 1), ("ring", 1), ("planks", 10), ("anvil", 1)];
    for (name, times) in plan.iter() {
        match crafter.craft(&mut inv, name, *times) {
            Ok(n) => println!("crafted {} x{} -> +{} | {}", name, times, n, inv),
            Err(e) => println!("craft {} x{} failed: {} | {}", name, times, e, inv),
        }
    }
    println!("can craft: {:?}", crafter.craftable(&inv));
    println!("worth {}", worth(&inv, Material::value));
    println!("worth doubled {}", worth(&inv, |m| m.value() * 2));
    let mut small: Inventory<Material, 2> = Inventory::new(Weight(1000));
    println!("{:?}", small.add(Ring, 2));
    println!("{:?}", small.add(Ring, 1));
    println!("{:?}", small.remove(Gold, 1));
    println!("{}", small);
    let heavy: Weight = Material::ALL.iter().map(|m| m.weight()).sum();
    println!("all weights {} max {}", heavy, Material::ALL.iter().map(|m| m.weight()).max().unwrap());
    println!("[{:>8}] [{:<8}]", Gold, Stick);
}
