// GPUI-style actions and keymaps: an `actions!` macro defining unit-struct actions with names,
// a trait Action with as_any/boxed_clone/name, keystroke parsing, multi-key bindings with context
// predicates, and dispatch to handlers registered by TypeId.
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt::Debug;

trait Action: Any + Debug {
    fn name(&self) -> &'static str;
    fn boxed_clone(&self) -> Box<dyn Action>;
    fn as_any(&self) -> &dyn Any;
    fn partial_eq(&self, other: &dyn Action) -> bool;
}

macro_rules! actions {
    ($namespace:ident, [$($name:ident),* $(,)?]) => {
        $(
            #[derive(Debug, Clone, PartialEq, Default)]
            struct $name;
            impl Action for $name {
                fn name(&self) -> &'static str { concat!(stringify!($namespace), "::", stringify!($name)) }
                fn boxed_clone(&self) -> Box<dyn Action> { Box::new(self.clone()) }
                fn as_any(&self) -> &dyn Any { self }
                fn partial_eq(&self, o: &dyn Action) -> bool { o.as_any().downcast_ref::<Self>().map_or(false, |o| o == self) }
            }
        )*
    };
}

actions!(editor, [Copy, Paste, Undo, SelectAll]);
actions!(workspace, [Save, CloseTab, ToggleSidebar]);

// Action with data
#[derive(Debug, Clone, PartialEq)]
struct GoToLine(u32);
impl Action for GoToLine {
    fn name(&self) -> &'static str {
        "editor::GoToLine"
    }
    fn boxed_clone(&self) -> Box<dyn Action> {
        Box::new(self.clone())
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn partial_eq(&self, o: &dyn Action) -> bool {
        o.as_any().downcast_ref::<Self>() == Some(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Keystroke {
    ctrl: bool,
    shift: bool,
    key: String,
}

impl Keystroke {
    fn parse(s: &str) -> Result<Keystroke, String> {
        let mut k = Keystroke { ctrl: false, shift: false, key: String::new() };
        for part in s.split('-') {
            match part {
                "ctrl" => k.ctrl = true,
                "shift" => k.shift = true,
                "" => return Err(format!("empty component in {:?}", s)),
                p if k.key.is_empty() => k.key = p.to_string(),
                p => return Err(format!("unexpected {:?} in {:?}", p, s)),
            }
        }
        if k.key.is_empty() {
            return Err(format!("no key in {:?}", s));
        }
        Ok(k)
    }
}

struct Binding {
    keys: Vec<Keystroke>,
    action: Box<dyn Action>,
    context: Option<&'static str>,
}

#[derive(Debug, PartialEq)]
enum Match {
    None,
    Pending,
    Complete(String),
}

struct Keymap {
    bindings: Vec<Binding>,
    pending: Vec<Keystroke>,
}

impl Keymap {
    fn bind(&mut self, keys: &str, action: impl Action, context: Option<&'static str>) -> Result<(), String> {
        let ks = keys.split(' ').map(Keystroke::parse).collect::<Result<Vec<_>, _>>()?;
        self.bindings.push(Binding { keys: ks, action: Box::new(action), context });
        Ok(())
    }

    fn push(&mut self, ks: Keystroke, contexts: &[&str]) -> (Match, Option<Box<dyn Action>>) {
        self.pending.push(ks);
        let mut pending = false;
        // later bindings take precedence
        for b in self.bindings.iter().rev() {
            if let Some(c) = b.context {
                if !contexts.contains(&c) {
                    continue;
                }
            }
            if b.keys == self.pending {
                self.pending.clear();
                return (Match::Complete(b.action.name().to_string()), Some(b.action.boxed_clone()));
            }
            if b.keys.starts_with(&self.pending) {
                pending = true;
            }
        }
        if pending {
            (Match::Pending, None)
        } else {
            self.pending.clear();
            (Match::None, None)
        }
    }
}

type Handler = Box<dyn Fn(&dyn Action, &mut Vec<String>)>;

struct Dispatcher {
    handlers: HashMap<TypeId, Handler>,
}

impl Dispatcher {
    fn on_action<A: Action + 'static>(&mut self, f: impl Fn(&A, &mut Vec<String>) + 'static) {
        self.handlers.insert(
            TypeId::of::<A>(),
            Box::new(move |a, log| f(a.as_any().downcast_ref::<A>().unwrap(), log)),
        );
    }
    fn dispatch(&self, a: &dyn Action, log: &mut Vec<String>) -> bool {
        match self.handlers.get(&a.as_any().type_id()) {
            Some(h) => {
                h(a, log);
                true
            }
            None => false,
        }
    }
}

fn main() {
    let mut km = Keymap { bindings: vec![], pending: vec![] };
    km.bind("ctrl-c", Copy, Some("Editor")).unwrap();
    km.bind("ctrl-v", Paste, Some("Editor")).unwrap();
    km.bind("ctrl-z", Undo, None).unwrap();
    km.bind("ctrl-a", SelectAll, Some("Editor")).unwrap();
    km.bind("ctrl-s", Save, None).unwrap();
    km.bind("ctrl-k ctrl-w", CloseTab, None).unwrap();
    km.bind("ctrl-k ctrl-b", ToggleSidebar, None).unwrap();
    km.bind("ctrl-g", GoToLine(1), Some("Editor")).unwrap();
    km.bind("ctrl-shift-g", GoToLine(999), Some("Editor")).unwrap();
    println!("{:?}", km.bind("ctrl--x", Copy, None));
    println!("{:?}", km.bind("ctrl", Copy, None));
    println!("{:?}", km.bind("a-b", Copy, None));

    let mut d = Dispatcher { handlers: HashMap::new() };
    d.on_action(|_: &Copy, log| log.push("copied selection".into()));
    d.on_action(|_: &Save, log| log.push("saved file".into()));
    d.on_action(|g: &GoToLine, log| log.push(format!("jumped to line {}", g.0)));
    d.on_action(|_: &CloseTab, log| log.push("closed tab".into()));

    let mut log = vec![];
    let editor_ctx = ["Workspace", "Editor"];
    let ws_ctx = ["Workspace"];
    let inputs: [(&str, &[&str]); 9] = [
        ("ctrl-c", &editor_ctx),
        ("ctrl-c", &ws_ctx),
        ("ctrl-s", &ws_ctx),
        ("ctrl-k", &ws_ctx),
        ("ctrl-w", &ws_ctx),
        ("ctrl-k", &ws_ctx),
        ("x", &ws_ctx),
        ("ctrl-shift-g", &editor_ctx),
        ("ctrl-z", &editor_ctx),
    ];
    for (k, ctx) in inputs {
        let ks = Keystroke::parse(k).unwrap();
        let (m, action) = km.push(ks, ctx);
        let handled = action.as_ref().map(|a| d.dispatch(a.as_ref(), &mut log));
        println!("{:<13} {:<28} handled={:?}", k, format!("{:?}", m), handled);
    }
    println!("{:?}", log);
    let a: Box<dyn Action> = Box::new(GoToLine(5));
    println!("{} {} {}", a.partial_eq(&GoToLine(5)), a.partial_eq(&GoToLine(6)), a.partial_eq(&Copy));
    println!("{:?} {}", a.boxed_clone(), ToggleSidebar.name());
}
