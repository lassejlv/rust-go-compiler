#[derive(Debug, Clone, PartialEq)]
struct Matrix {
    n: usize,
    m: usize,
    d: Vec<Vec<f64>>,
}

impl Matrix {
    fn zeros(n: usize, m: usize) -> Self {
        Matrix { n, m, d: vec![vec![0.0; m]; n] }
    }
    fn identity(n: usize) -> Self {
        let mut r = Self::zeros(n, n);
        for i in 0..n {
            r.d[i][i] = 1.0;
        }
        r
    }
    fn from(rows: Vec<Vec<f64>>) -> Self {
        let n = rows.len();
        let m = rows[0].len();
        Matrix { n, m, d: rows }
    }
    fn mul(&self, o: &Matrix) -> Option<Matrix> {
        if self.m != o.n {
            return None;
        }
        let mut r = Matrix::zeros(self.n, o.m);
        for i in 0..self.n {
            for j in 0..o.m {
                let mut s = 0.0;
                for k in 0..self.m {
                    s += self.d[i][k] * o.d[k][j];
                }
                r.d[i][j] = s;
            }
        }
        Some(r)
    }
    fn transpose(&self) -> Matrix {
        let mut r = Matrix::zeros(self.m, self.n);
        for i in 0..self.n {
            for j in 0..self.m {
                r.d[j][i] = self.d[i][j];
            }
        }
        r
    }
    fn pow(&self, mut e: u32) -> Matrix {
        let mut base = self.clone();
        let mut acc = Matrix::identity(self.n);
        while e > 0 {
            if e & 1 == 1 {
                acc = acc.mul(&base).unwrap();
            }
            base = base.mul(&base).unwrap();
            e >>= 1;
        }
        acc
    }
    fn determinant(&self) -> f64 {
        let mut a = self.d.clone();
        let n = self.n;
        let mut det = 1.0;
        for col in 0..n {
            let mut pivot = col;
            for r in col + 1..n {
                if a[r][col].abs() > a[pivot][col].abs() {
                    pivot = r;
                }
            }
            if a[pivot][col] == 0.0 {
                return 0.0;
            }
            if pivot != col {
                a.swap(pivot, col);
                det = -det;
            }
            det *= a[col][col];
            for r in col + 1..n {
                let f = a[r][col] / a[col][col];
                for c in col..n {
                    a[r][c] -= f * a[col][c];
                }
            }
        }
        det
    }
    fn print(&self, name: &str) {
        println!("{} ({}x{}):", name, self.n, self.m);
        for row in &self.d {
            let cells: Vec<String> = row.iter().map(|x| format!("{:8.2}", x)).collect();
            println!("  [{}]", cells.join(""));
        }
    }
}

fn main() {
    let a = Matrix::from(vec![vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0]]);
    let b = Matrix::from(vec![vec![7.0, 8.0], vec![9.0, 10.0], vec![11.0, 12.0]]);
    a.print("A");
    b.print("B");
    a.mul(&b).unwrap().print("A*B");
    b.mul(&a).unwrap().print("B*A");
    println!("A*A defined: {}", a.mul(&a).is_some());
    a.transpose().print("A^T");
    let fib = Matrix::from(vec![vec![1.0, 1.0], vec![1.0, 0.0]]);
    for e in [1, 5, 10, 30] {
        println!("fib via matrix power {}: {}", e, fib.pow(e).d[0][1]);
    }
    let sq = Matrix::from(vec![vec![2.0, -1.0, 0.0], vec![-1.0, 2.0, -1.0], vec![0.0, -1.0, 2.0]]);
    println!("det = {:.4}", sq.determinant());
    println!("det identity = {}", Matrix::identity(4).determinant());
    let singular = Matrix::from(vec![vec![1.0, 2.0], vec![2.0, 4.0]]);
    println!("det singular = {}", singular.determinant());
    let i3 = Matrix::identity(3);
    println!("sq * I == sq: {}", sq.mul(&i3).unwrap() == sq);
    let rot = Matrix::from(vec![vec![0.0, -1.0], vec![1.0, 0.0]]);
    rot.pow(4).print("rot^4");
    let mut int_m = [[0i64; 4]; 4];
    for i in 0..4 {
        for j in 0..4 {
            int_m[i][j] = (i as i64 + 1) * (j as i64 + 2) % 7;
        }
    }
    let mut prod = [[0i64; 4]; 4];
    for i in 0..4 {
        for j in 0..4 {
            for k in 0..4 {
                prod[i][j] += int_m[i][k] * int_m[k][j];
            }
        }
    }
    for row in prod.iter() {
        println!("{:?}", row);
    }
    let trace: i64 = (0..4).map(|i| prod[i][i]).sum();
    println!("trace = {}", trace);
}
