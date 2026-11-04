// Task scheduler simulation with Rc<RefCell<>> task graph, Weak parent and
// dependent links, Drop logging, priority aging, a round-robin CPU model,
// cancellation cascades, and cycle detection.
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::fmt;
use std::rc::{Rc, Weak};

thread_local! {
    static DROPS: RefCell<Vec<String>> = RefCell::new(Vec::new());
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Status { Pending, Ready, Running, Done, Cancelled, Failed }

struct Task {
    id: u32,
    name: String,
    work: Cell<u32>,
    priority: Cell<i32>,
    status: Cell<Status>,
    deps: RefCell<Vec<Rc<RefCell<Task>>>>,
    dependents: RefCell<Vec<Weak<RefCell<Task>>>>,
    parent: RefCell<Weak<RefCell<Task>>>,
    children: RefCell<Vec<Rc<RefCell<Task>>>>,
    log: RefCell<Vec<(u32, &'static str)>>,
    fail_at: Option<u32>,
}

impl Drop for Task {
    fn drop(&mut self) {
        let msg = format!("{}#{}({:?})", self.name, self.id, self.status.get());
        DROPS.with(|d| d.borrow_mut().push(msg));
    }
}

impl fmt::Debug for Task {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Task")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("status", &self.status.get())
            .field("work", &self.work.get())
            .field("deps", &self.deps.borrow().iter().map(|d| d.borrow().id).collect::<Vec<_>>())
            .finish()
    }
}

type TaskRef = Rc<RefCell<Task>>;

struct Scheduler {
    tasks: BTreeMap<u32, TaskRef>,
    ready: VecDeque<TaskRef>,
    clock: u32,
    quantum: u32,
    next_id: u32,
    timeline: Vec<String>,
}

impl Scheduler {
    fn new(quantum: u32) -> Self {
        Scheduler { tasks: BTreeMap::new(), ready: VecDeque::new(), clock: 0, quantum, next_id: 1, timeline: vec![] }
    }

    fn spawn(&mut self, name: &str, work: u32, prio: i32, parent: Option<&TaskRef>, fail_at: Option<u32>) -> TaskRef {
        let t = Rc::new(RefCell::new(Task {
            id: self.next_id,
            name: name.to_string(),
            work: Cell::new(work),
            priority: Cell::new(prio),
            status: Cell::new(Status::Pending),
            deps: RefCell::new(vec![]),
            dependents: RefCell::new(vec![]),
            parent: RefCell::new(Weak::new()),
            children: RefCell::new(vec![]),
            log: RefCell::new(vec![]),
            fail_at,
        }));
        self.next_id += 1;
        if let Some(p) = parent {
            *t.borrow().parent.borrow_mut() = Rc::downgrade(p);
            p.borrow().children.borrow_mut().push(t.clone());
        }
        self.tasks.insert(t.borrow().id, t.clone());
        t
    }

    fn depend(&self, t: &TaskRef, on: &TaskRef) {
        t.borrow().deps.borrow_mut().push(on.clone());
        on.borrow().dependents.borrow_mut().push(Rc::downgrade(t));
    }

    fn has_cycle(&self) -> Option<Vec<u32>> {
        #[derive(Clone, Copy, PartialEq)]
        enum Mark { White, Grey, Black }
        let mut marks: HashMap<u32, Mark> = self.tasks.keys().map(|&k| (k, Mark::White)).collect();
        fn visit(t: &TaskRef, marks: &mut HashMap<u32, Mark>, path: &mut Vec<u32>) -> bool {
            let id = t.borrow().id;
            match marks[&id] {
                Mark::Grey => {
                    path.push(id);
                    return true;
                }
                Mark::Black => return false,
                Mark::White => {}
            }
            marks.insert(id, Mark::Grey);
            path.push(id);
            let deps: Vec<TaskRef> = t.borrow().deps.borrow().clone();
            for d in &deps {
                if visit(d, marks, path) {
                    return true;
                }
            }
            path.pop();
            marks.insert(id, Mark::Black);
            false
        }
        for t in self.tasks.values() {
            let mut path = vec![];
            if visit(t, &mut marks, &mut path) {
                let last = *path.last().unwrap();
                let start = path.iter().position(|&x| x == last).unwrap();
                return Some(path[start..].to_vec());
            }
        }
        None
    }

    fn refresh_ready(&mut self) {
        for t in self.tasks.values() {
            let tb = t.borrow();
            if tb.status.get() != Status::Pending {
                continue;
            }
            let deps_done = tb.deps.borrow().iter().all(|d| d.borrow().status.get() == Status::Done);
            if deps_done {
                tb.status.set(Status::Ready);
                tb.log.borrow_mut().push((self.clock, "ready"));
                drop(tb);
                self.ready.push_back(t.clone());
            }
        }
        // stable sort by priority (higher first)
        let mut v: Vec<TaskRef> = self.ready.drain(..).collect();
        v.sort_by_key(|t| -t.borrow().priority.get());
        self.ready.extend(v);
    }

    fn cancel(&mut self, t: &TaskRef, reason: &'static str) -> usize {
        let mut count = 0;
        let mut stack = vec![t.clone()];
        while let Some(cur) = stack.pop() {
            let c = cur.borrow();
            if matches!(c.status.get(), Status::Done | Status::Cancelled | Status::Failed) {
                continue;
            }
            c.status.set(Status::Cancelled);
            c.log.borrow_mut().push((self.clock, reason));
            count += 1;
            for w in c.dependents.borrow().iter() {
                if let Some(d) = w.upgrade() {
                    stack.push(d);
                }
            }
            for ch in c.children.borrow().iter() {
                stack.push(ch.clone());
            }
        }
        self.ready.retain(|r| r.borrow().status.get() == Status::Ready);
        count
    }

    fn run(&mut self, max_ticks: u32) {
        self.refresh_ready();
        while self.clock < max_ticks {
            let Some(t) = self.ready.pop_front() else {
                if self.tasks.values().all(|t| t.borrow().status.get() != Status::Pending) {
                    break;
                }
                self.clock += 1;
                self.refresh_ready();
                if self.ready.is_empty() {
                    break;
                }
                continue;
            };
            let (id, name, slice, failed) = {
                let tb = t.borrow();
                tb.status.set(Status::Running);
                let slice = tb.work.get().min(self.quantum);
                let before = tb.work.get();
                tb.work.set(before - slice);
                let failed = tb.fail_at.map_or(false, |f| before > f && before - slice <= f);
                (tb.id, tb.name.clone(), slice, failed)
            };
            self.timeline.push(format!("{:>3}-{:<3} {}#{}", self.clock, self.clock + slice, name, id));
            self.clock += slice;
            // age waiting tasks
            for w in self.ready.iter() {
                let p = &w.borrow().priority;
                p.set(p.get() + 1);
            }
            if failed {
                t.borrow().status.set(Status::Failed);
                t.borrow().log.borrow_mut().push((self.clock, "failed"));
                let deps: Vec<TaskRef> = t.borrow().dependents.borrow().iter().filter_map(|w| w.upgrade()).collect();
                let mut n = 0;
                for d in deps {
                    n += self.cancel(&d, "dep failed");
                }
                self.timeline.push(format!("    !! {}#{} failed, cancelled {}", name, id, n));
            } else if t.borrow().work.get() == 0 {
                t.borrow().status.set(Status::Done);
                t.borrow().log.borrow_mut().push((self.clock, "done"));
                if let Some(p) = t.borrow().parent.borrow().upgrade() {
                    let pb = p.borrow();
                    let all = pb.children.borrow().iter().all(|c| c.borrow().status.get() == Status::Done);
                    if all {
                        pb.log.borrow_mut().push((self.clock, "children done"));
                    }
                }
                self.refresh_ready();
            } else {
                t.borrow().status.set(Status::Ready);
                t.borrow().priority.set(t.borrow().priority.get() - 2);
                self.ready.push_back(t);
                self.refresh_ready();
            }
        }
    }
}

fn main() {
    let mut s = Scheduler::new(3);
    let build = s.spawn("build", 2, 5, None, None);
    let compile_a = s.spawn("compile_a", 7, 3, Some(&build), None);
    let compile_b = s.spawn("compile_b", 4, 3, Some(&build), None);
    let link = s.spawn("link", 3, 4, Some(&build), None);
    let test = s.spawn("test", 6, 2, None, None);
    let lint = s.spawn("lint", 2, 1, None, None);
    let docs = s.spawn("docs", 5, 0, None, Some(2));
    let publish = s.spawn("publish", 1, 9, None, None);
    let flaky = s.spawn("flaky_cleanup", 2, 0, Some(&docs), None);
    s.depend(&link, &compile_a);
    s.depend(&link, &compile_b);
    s.depend(&test, &link);
    s.depend(&publish, &test);
    s.depend(&publish, &docs);
    s.depend(&publish, &lint);
    s.depend(&build, &link);
    println!("cycle: {:?}", s.has_cycle());
    println!("{:?}", compile_a.borrow());
    println!("strong(build)={} weak(build)={} strong(link)={} weak(link)={}",
        Rc::strong_count(&build), Rc::weak_count(&build), Rc::strong_count(&link), Rc::weak_count(&link));
    let parent_name = compile_b.borrow().parent.borrow().upgrade().map(|p| p.borrow().name.clone());
    println!("compile_b parent: {:?}", parent_name);
    drop(flaky);

    s.run(100);
    for l in &s.timeline {
        println!("{}", l);
    }
    println!("clock = {}", s.clock);
    for (id, t) in &s.tasks {
        let tb = t.borrow();
        let log: Vec<String> = tb.log.borrow().iter().map(|(c, e)| format!("{}@{}", e, c)).collect();
        println!("{:>2} {:<14} {:<10} prio {:>3}  {}", id, tb.name, format!("{:?}", tb.status.get()), tb.priority.get(), log.join(", "));
    }
    let by_status = s.tasks.values().fold(BTreeMap::new(), |mut m: BTreeMap<String, u32>, t| {
        *m.entry(format!("{:?}", t.borrow().status.get())).or_default() += 1;
        m
    });
    println!("{:?}", by_status);

    // cycles
    let mut s2 = Scheduler::new(2);
    let a = s2.spawn("a", 1, 0, None, None);
    let b = s2.spawn("b", 1, 0, None, None);
    let c = s2.spawn("c", 1, 0, None, None);
    s2.depend(&a, &b);
    s2.depend(&b, &c);
    s2.depend(&c, &a);
    println!("cycle: {:?}", s2.has_cycle());
    s2.run(10);
    println!("cyclic run clock {} timeline {:?}", s2.clock, s2.timeline);
    // break the cycle so things can be dropped
    c.borrow().deps.borrow_mut().clear();
    println!("after break: {:?}", s2.has_cycle());
    s2.run(10);
    println!("timeline {:?}", s2.timeline);
    drop((a, b, c));

    let try_borrow = {
        let t = s.tasks.values().next().unwrap().clone();
        let guard = t.borrow_mut();
        let r = t.try_borrow().is_err();
        drop(guard);
        let ok = t.try_borrow_mut().is_ok();
        (r, ok)
    };
    println!("try_borrow while mut-borrowed fails: {:?}", try_borrow);

    DROPS.with(|d| println!("drops before scheduler drop: {:?}", d.borrow()));
    drop(s2);
    DROPS.with(|d| println!("after s2: {:?}", d.borrow().len()));
    let weak_build = Rc::downgrade(&build);
    drop((build, compile_a, compile_b, link, test, lint, docs, publish));
    println!("build alive via scheduler: {}", weak_build.upgrade().is_some());
    drop(s);
    println!("build alive after scheduler drop: {}", weak_build.upgrade().is_some());
    DROPS.with(|d| {
        let d = d.borrow();
        println!("total drops {}: {:?}", d.len(), &d[3..]);
    });
}
