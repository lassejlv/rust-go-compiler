//~ ERROR E0382
struct Req { url: String, body: String }
impl Req { fn len(&self) -> usize { self.url.len() + self.body.len() } }
fn main() {
    let r = Req { url: "u".into(), body: "b".into() };
    let body = r.body;
    println!("{} {}", body, r.len());
}
