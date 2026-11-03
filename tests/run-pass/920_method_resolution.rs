trait Speak {
    fn speak(&self) -> String;
    fn id(&self) -> u32 { 0 }
}

struct Dog;
impl Dog {
    // inherent method shadows trait method
    fn speak(&self) -> String { "inherent woof".into() }
}
impl Speak for Dog {
    fn speak(&self) -> String { "trait woof".into() }
    fn id(&self) -> u32 { 1 }
}

impl<T: Speak> Speak for Vec<T> {
    fn speak(&self) -> String { self.iter().map(|x| x.speak()).collect::<Vec<_>>().join("+") }
}
impl<'a, T: Speak + ?Sized> Speak for &'a T {
    fn speak(&self) -> String { format!("ref({})", (**self).speak()) }
    fn id(&self) -> u32 { 100 + (**self).id() }
}
impl<T: Speak> Speak for Box<T> {
    fn speak(&self) -> String { format!("box({})", (**self).speak()) }
}

struct Cat;
impl Speak for Cat { fn speak(&self) -> String { "meow".into() } }

trait A { fn name(&self) -> &'static str { "A" } }
trait B { fn name(&self) -> &'static str { "B" } }
struct Both;
impl A for Both {}
impl B for Both {}

fn generic_speak<T: Speak>(t: T) -> String { format!("{}#{}", t.speak(), t.id()) }
fn generic_ref<T: Speak + ?Sized>(t: &T) -> String { t.speak() }

trait Value { fn value(self) -> i32; }
impl Value for i32 { fn value(self) -> i32 { self } }
impl Value for &i32 { fn value(self) -> i32 { *self * 10 } }
impl Value for &&i32 { fn value(self) -> i32 { **self * 100 } }

fn main() {
    let d = Dog;
    println!("{}", d.speak());
    println!("{}", Speak::speak(&d));
    println!("{}", <Dog as Speak>::speak(&d));
    println!("{}", generic_speak(Dog));
    println!("{}", generic_speak(&Dog));
    println!("{}", generic_speak(&&Cat));
    println!("{}", generic_speak(Box::new(Cat)));
    println!("{}", generic_speak(vec![Cat, Cat]));
    println!("{}", generic_speak(vec![&Dog as &dyn Speak, &Cat]));
    println!("{}", generic_ref(&Dog));
    println!("{}", generic_ref::<dyn Speak>(&Cat));
    let b = Both;
    println!("{} {}", A::name(&b), <Both as B>::name(&b));
    let x = 7i32;
    let rx = &x;
    println!("{} {} {}", x.value(), rx.value(), (&rx).value());
    println!("{} {}", Value::value(&x), <&&i32 as Value>::value(&rx));
}
