// ManuallyDrop, mem::forget, mem::replace, mem::take, mem::swap, Option::replace,
// and checking exactly which values are dropped and when.
use std::cell::Cell;
use std::mem::{self, ManuallyDrop};

thread_local! {
    static DROPS: Cell<u32> = Cell::new(0);
}

#[derive(Debug, Default)]
struct Tracked {
    name: String,
}
impl Drop for Tracked {
    fn drop(&mut self) {
        DROPS.with(|d| d.set(d.get() + 1));
        println!("  drop {:?}", self.name);
    }
}
fn t(n: &str) -> Tracked {
    Tracked { name: n.to_string() }
}
fn drops() -> u32 {
    DROPS.with(|d| d.get())
}

struct Holder {
    slot: Tracked,
    items: Vec<Tracked>,
}

impl Holder {
    fn swap_slot(&mut self, new: Tracked) -> Tracked {
        mem::replace(&mut self.slot, new)
    }
    fn take_items(&mut self) -> Vec<Tracked> {
        mem::take(&mut self.items)
    }
}

fn main() {
    println!("forget:");
    let a = t("forgotten");
    mem::forget(a);
    println!("  drops={}", drops());

    println!("manually drop:");
    let mut m = ManuallyDrop::new(t("manual"));
    m.name.push_str("-edited");
    println!("  inner name {}", m.name);
    println!("  drops={}", drops());
    unsafe { ManuallyDrop::drop(&mut m) };
    println!("  drops={}", drops());

    let md = ManuallyDrop::new(t("into_inner"));
    let back = ManuallyDrop::into_inner(md);
    println!("  got back {}", back.name);
    drop(back);

    println!("replace/take:");
    let mut h = Holder { slot: t("s1"), items: vec![t("i1"), t("i2")] };
    let old = h.swap_slot(t("s2"));
    println!("  old={} new={}", old.name, h.slot.name);
    drop(old);
    let items = h.take_items();
    println!("  took {} items, left {}", items.len(), h.items.len());
    let defaulted = mem::take(&mut h.slot);
    println!("  defaulted slot name={:?} took={}", h.slot.name, defaulted.name);
    drop(items);
    println!("  drops={}", drops());

    println!("swap:");
    let mut x = t("x");
    let mut y = t("y");
    mem::swap(&mut x, &mut y);
    println!("  x={} y={}", x.name, y.name);
    let mut arr = [t("a0"), t("a1"), t("a2")];
    arr.swap(0, 2);
    let names: Vec<&str> = arr.iter().map(|t| t.name.as_str()).collect();
    println!("  arr={:?}", names);
    let (l, r) = arr.split_at_mut(1);
    mem::swap(&mut l[0], &mut r[1]);
    let names: Vec<&str> = arr.iter().map(|t| t.name.as_str()).collect();
    println!("  arr={:?}", names);

    println!("option replace:");
    let mut o = Some(t("o1"));
    let prev = o.replace(t("o2"));
    println!("  prev={:?}", prev.as_ref().map(|t| &t.name));
    let prev2 = o.take();
    println!("  after take is_none={} prev2={:?}", o.is_none(), prev2.map(|t| t.name.clone()));
    let got = o.get_or_insert_with(|| t("o3"));
    got.name.push('!');
    println!("  o={:?}", o.as_ref().map(|t| t.name.as_str()));

    println!("vec ManuallyDrop:");
    let v: Vec<ManuallyDrop<Tracked>> = vec![ManuallyDrop::new(t("mv0")), ManuallyDrop::new(t("mv1"))];
    let total: usize = v.iter().map(|m| m.name.len()).sum();
    println!("  total name len {}", total);
    drop(v);
    println!("  drops={}", drops());

    println!("end of main, drops={}", drops());
    drop(prev);
}
