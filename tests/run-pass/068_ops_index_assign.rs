use std::ops::{AddAssign, Index, IndexMut, MulAssign, SubAssign};

#[derive(Debug, Clone, PartialEq)]
struct Matrix {
    rows: usize,
    cols: usize,
    data: Vec<i64>,
}

impl Matrix {
    fn new(rows: usize, cols: usize) -> Self {
        Matrix { rows, cols, data: vec![0; rows * cols] }
    }
    fn identity(n: usize) -> Self {
        let mut m = Matrix::new(n, n);
        for i in 0..n {
            m[(i, i)] = 1;
        }
        m
    }
    fn print(&self) {
        for r in 0..self.rows {
            let row: Vec<String> = (0..self.cols).map(|c| format!("{:3}", self[(r, c)])).collect();
            println!("[{}]", row.join(""));
        }
    }
}

impl Index<(usize, usize)> for Matrix {
    type Output = i64;
    fn index(&self, (r, c): (usize, usize)) -> &i64 {
        &self.data[r * self.cols + c]
    }
}

impl IndexMut<(usize, usize)> for Matrix {
    fn index_mut(&mut self, (r, c): (usize, usize)) -> &mut i64 {
        &mut self.data[r * self.cols + c]
    }
}

impl AddAssign<&Matrix> for Matrix {
    fn add_assign(&mut self, o: &Matrix) {
        for i in 0..self.data.len() {
            self.data[i] += o.data[i];
        }
    }
}

impl MulAssign<i64> for Matrix {
    fn mul_assign(&mut self, k: i64) {
        for x in self.data.iter_mut() {
            *x *= k;
        }
    }
}

struct Counter {
    value: i32,
}

impl AddAssign<i32> for Counter {
    fn add_assign(&mut self, n: i32) {
        self.value += n;
    }
}

impl SubAssign<i32> for Counter {
    fn sub_assign(&mut self, n: i32) {
        self.value -= n;
    }
}

struct Polynomial {
    coeffs: Vec<i32>,
}

impl Index<usize> for Polynomial {
    type Output = i32;
    fn index(&self, i: usize) -> &i32 {
        if i < self.coeffs.len() { &self.coeffs[i] } else { &0 }
    }
}

fn main() {
    let mut m = Matrix::new(3, 3);
    for r in 0..3 {
        for c in 0..3 {
            m[(r, c)] = (r * 3 + c) as i64;
        }
    }
    m.print();
    println!("m[(1,2)] = {}", m[(1, 2)]);
    let id = Matrix::identity(3);
    m += &id;
    println!("after += identity:");
    m.print();
    m *= -2;
    println!("after *= -2:");
    m.print();
    println!("identity == identity: {}", id == Matrix::identity(3));
    let mut c = Counter { value: 10 };
    c += 5;
    c -= 3;
    c += 100;
    println!("counter = {}", c.value);
    let p = Polynomial { coeffs: vec![1, 0, -3] };
    println!("p[0]={} p[2]={} p[7]={}", p[0], p[2], p[7]);
    let mut v = vec![1, 2, 3];
    v[0] += 10;
    let idx = v.len() - 1;
    v[idx] *= 7;
    println!("v = {:?}", v);
}
