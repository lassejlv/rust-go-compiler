// supertrait method resolution, same-named methods, fully qualified calls
trait Animal {
    fn name(&self) -> String;
    fn sound(&self) -> String {
        "...".to_string()
    }
    fn intro(&self) -> String {
        format!("{} says {}", self.name(), self.sound())
    }
}
trait Pet: Animal {
    fn owner(&self) -> String;
    fn intro_pet(&self) -> String {
        format!("{} (owned by {})", self.intro(), self.owner())
    }
}
trait Trick: Pet + std::fmt::Debug {
    fn trick(&self) -> &'static str;
    fn show(&self) -> String {
        format!("{} does {} [{:?}]", self.intro_pet(), self.trick(), self)
    }
}

#[derive(Debug)]
struct Dog {
    n: &'static str,
}
impl Animal for Dog {
    fn name(&self) -> String {
        self.n.to_string()
    }
    fn sound(&self) -> String {
        "woof".into()
    }
}
impl Pet for Dog {
    fn owner(&self) -> String {
        "alice".into()
    }
}
impl Trick for Dog {
    fn trick(&self) -> &'static str {
        "roll"
    }
}

#[derive(Debug)]
struct Fish;
impl Animal for Fish {
    fn name(&self) -> String {
        "fish".into()
    }
}
impl Pet for Fish {
    fn owner(&self) -> String {
        "bob".into()
    }
    fn intro_pet(&self) -> String {
        format!("a quiet {}", self.name())
    }
}
impl Trick for Fish {
    fn trick(&self) -> &'static str {
        "swim"
    }
}

trait Pilot {
    fn fly(&self) -> String;
    fn kind() -> &'static str {
        "pilot"
    }
}
trait Wizard {
    fn fly(&self) -> String;
    fn kind() -> &'static str {
        "wizard"
    }
}
struct Human;
impl Pilot for Human {
    fn fly(&self) -> String {
        "captain speaking".into()
    }
}
impl Wizard for Human {
    fn fly(&self) -> String {
        "up!".into()
    }
}
impl Human {
    fn fly(&self) -> String {
        "*waving arms*".into()
    }
    fn kind() -> &'static str {
        "human"
    }
}

fn all_intros(ts: &[Box<dyn Trick>]) {
    for t in ts {
        println!("{}", t.show());
        let p: &dyn Pet = t.as_ref();
        println!("  pet: {}", p.owner());
        let a: &dyn Animal = p;
        println!("  animal: {}", a.intro());
    }
}

fn generic_intro<T: Trick + ?Sized>(t: &T) -> String {
    format!("{} / {}", Animal::name(t), <T as Pet>::owner(t))
}

trait Base {
    fn val(&self) -> i32;
}
trait Mid1: Base {
    fn m1(&self) -> i32 {
        self.val() * 10
    }
}
trait Mid2: Base {
    fn m2(&self) -> i32 {
        self.val() + 1
    }
}
trait Diamond: Mid1 + Mid2 {
    fn both(&self) -> i32 {
        self.m1() + self.m2() + self.val()
    }
}
struct D(i32);
impl Base for D {
    fn val(&self) -> i32 {
        self.0
    }
}
impl Mid1 for D {}
impl Mid2 for D {
    fn m2(&self) -> i32 {
        -self.val()
    }
}
impl Diamond for D {}

fn main() {
    let v: Vec<Box<dyn Trick>> = vec![Box::new(Dog { n: "rex" }), Box::new(Fish)];
    all_intros(&v);
    println!("{}", generic_intro(&Dog { n: "fido" }));
    println!("{}", generic_intro(v[1].as_ref()));
    let h = Human;
    println!("{}", h.fly());
    println!("{}", Pilot::fly(&h));
    println!("{}", <Human as Wizard>::fly(&h));
    println!("{} {} {}", Human::kind(), <Human as Pilot>::kind(), <Human as Wizard>::kind());
    let d = D(4);
    println!("{} {} {} {}", d.val(), d.m1(), d.m2(), d.both());
    let dd: &dyn Diamond = &d;
    println!("{}", dd.both());
    let m: &dyn Mid1 = dd;
    println!("{}", m.m1());
}
