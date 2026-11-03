// iter_mut with mutation of disjoint data while iterating.
struct Particle {
    pos: i32,
    vel: i32,
    hist: Vec<i32>,
}

fn main() {
    let mut ps: Vec<Particle> = (0..3)
        .map(|i| Particle { pos: i, vel: i + 1, hist: Vec::new() })
        .collect();
    let mut total = 0;
    for step in 0..3 {
        for p in ps.iter_mut() {
            p.pos += p.vel;
            p.hist.push(p.pos);
            if step == 1 {
                p.vel = -p.vel;
            }
            total += p.pos;
        }
    }
    for (i, p) in ps.iter_mut().enumerate() {
        for h in p.hist.iter_mut() {
            *h += i as i32;
        }
    }
    for p in &ps {
        println!("{} {} {:?}", p.pos, p.vel, p.hist);
    }
    println!("total {}", total);
}
