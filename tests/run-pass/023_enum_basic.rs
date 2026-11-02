#[derive(Debug, Clone, Copy, PartialEq)]
enum Direction {
    North,
    East,
    South,
    West,
}

impl Direction {
    fn turn_right(self) -> Direction {
        match self {
            Direction::North => Direction::East,
            Direction::East => Direction::South,
            Direction::South => Direction::West,
            Direction::West => Direction::North,
        }
    }

    fn delta(self) -> (i32, i32) {
        match self {
            Direction::North => (0, 1),
            Direction::East => (1, 0),
            Direction::South => (0, -1),
            Direction::West => (-1, 0),
        }
    }
}

#[derive(Debug)]
enum Status {
    Active = 1,
    Inactive = 0,
    Banned = 99,
}

fn main() {
    let mut d = Direction::North;
    let mut pos = (0, 0);
    let moves = [3, 2, 1, 4, 2];
    for steps in moves {
        let (dx, dy) = d.delta();
        pos.0 += dx * steps;
        pos.1 += dy * steps;
        println!("moved {} {:?} -> ({}, {})", steps, d, pos.0, pos.1);
        d = d.turn_right();
    }
    println!("final direction: {:?}", d);
    println!("is west: {}", d == Direction::West);
    println!("status values: {} {} {}", Status::Active as i32, Status::Inactive as i32, Status::Banned as i32);
    println!("status debug: {:?}", Status::Banned);
    use Direction::*;
    let all = [North, East, South, West];
    for x in all.iter() {
        let name = match x {
            North | South => "vertical",
            East | West => "horizontal",
        };
        println!("{:?} is {}", x, name);
    }
}
