struct Flag<const ON: bool>;
impl Flag<true> { fn state(&self) -> &'static str { "on" } }
impl Flag<false> { fn state(&self) -> &'static str { "off" } }

struct Buffer<T, const N: usize = 4> { items: [T; N], used: usize }
impl<T: Copy + Default + std::fmt::Debug, const N: usize> Buffer<T, N> {
    fn new() -> Self { Buffer { items: [T::default(); N], used: 0 } }
    fn push(&mut self, t: T) -> Result<(), T> {
        if self.used == N { return Err(t); }
        self.items[self.used] = t;
        self.used += 1;
        Ok(())
    }
    fn view(&self) -> &[T] { &self.items[..self.used] }
}

struct Chars<const C: char>;
impl<const C: char> Chars<C> { fn line(n: usize) -> String { std::iter::repeat(C).take(n).collect() } }

trait Dims { const D: usize; }
struct Grid<const W: usize, const H: usize> { cells: [[u8; W]; H] }
impl<const W: usize, const H: usize> Dims for Grid<W, H> { const D: usize = W * H; }
impl<const W: usize, const H: usize> Grid<W, H> {
    fn new() -> Self { Grid { cells: [[0; W]; H] } }
    fn set(&mut self, x: usize, y: usize, v: u8) { self.cells[y][x] = v; }
    fn transpose(&self) -> Grid<H, W> {
        let mut g = Grid::<H, W>::new();
        for y in 0..H { for x in 0..W { g.cells[x][y] = self.cells[y][x]; } }
        g
    }
    fn render(&self) -> String {
        self.cells.iter().map(|r| r.iter().map(|c| if *c > 0 { '#' } else { '.' }).collect::<String>()).collect::<Vec<_>>().join("/")
    }
}

fn concat<const A: usize, const B: usize>(a: [i32; A], b: [i32; B]) -> Vec<i32> { a.iter().chain(b.iter()).copied().collect() }

fn arr_map<T, U, const N: usize>(a: [T; N], f: impl FnMut(T) -> U) -> [U; N] { a.map(f) }

fn main() {
    println!("{} {}", Flag::<true>.state(), Flag::<false>.state());
    let mut b: Buffer<u8> = Buffer::new();
    for i in 0..6 { if let Err(x) = b.push(i) { println!("full, rejected {}", x); } }
    println!("{:?}", b.view());
    let mut b2: Buffer<char, 2> = Buffer::new();
    b2.push('a').unwrap();
    let r = b2.push('b');
    let r2 = b2.push('c');
    println!("{:?} {:?} {:?}", b2.view(), r, r2);
    println!("{} {}", Chars::<'*'>::line(5), Chars::<'-'>::line(3));
    let mut g: Grid<3, 2> = Grid::new();
    g.set(0, 0, 1); g.set(2, 1, 1); g.set(1, 1, 1);
    println!("{} D={}", g.render(), <Grid<3, 2> as Dims>::D);
    let t = g.transpose();
    println!("{} D={}", t.render(), <Grid<2, 3>>::D);
    println!("{:?}", concat([1, 2], [3, 4, 5]));
    println!("{:?}", arr_map([1, 2, 3], |x| x as f32 / 2.0));
    println!("{:?}", arr_map(["a", "bb"], str::len));
}
