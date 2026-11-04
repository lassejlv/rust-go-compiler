//~ ERROR lint
// dangerous_implicit_autorefs inside an impl method: indexing a Vec field
// of `*self.buf`.
struct Buf {
    data: Vec<f32>,
}

struct Cursor {
    buf: *mut Buf,
    pos: usize,
}

impl Cursor {
    fn write(&mut self, x: f32) {
        unsafe {
            (*self.buf).data[self.pos] = x;
        }
        self.pos += 1;
    }
}

fn main() {
    let mut b = Buf { data: vec![0.0; 2] };
    let mut c = Cursor { buf: &mut b, pos: 0 };
    c.write(1.5);
    println!("{:?}", b.data);
}
