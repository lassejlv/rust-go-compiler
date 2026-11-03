// A struct holding &'a mut and methods using it.
struct Writer<'a> {
    out: &'a mut Vec<String>,
    prefix: &'a str,
}

impl<'a> Writer<'a> {
    fn write(&mut self, s: &str) {
        self.out.push(format!("{}{}", self.prefix, s));
    }
    fn last(&self) -> Option<&String> {
        self.out.last()
    }
    fn into_out(self) -> &'a mut Vec<String> {
        self.out
    }
}

fn main() {
    let mut lines = Vec::new();
    let p = String::from("> ");
    {
        let mut w = Writer { out: &mut lines, prefix: &p };
        w.write("a");
        w.write("b");
        println!("{:?}", w.last());
        let o = w.into_out();
        o.push(String::from("raw"));
    }
    lines.push(String::from("after"));
    println!("{:?}", lines);
}
