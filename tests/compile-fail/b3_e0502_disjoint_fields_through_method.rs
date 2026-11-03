//~ ERROR E0502
struct Game { players: Vec<String>, log: Vec<String> }
impl Game {
    fn log(&mut self, s: &str) { self.log.push(s.to_string()); }
    fn tick(&mut self) {
        for p in &self.players {
            self.log(p);
        }
    }
}
fn main() { Game { players: vec![], log: vec![] }.tick(); }
