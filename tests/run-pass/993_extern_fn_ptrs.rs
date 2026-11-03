use std::os::raw::c_int;

type BinOp = extern "C" fn(c_int, c_int) -> c_int;

extern "C" fn add(a: c_int, b: c_int) -> c_int { a + b }
extern "C" fn mul(a: c_int, b: c_int) -> c_int { a * b }
extern "C" fn sub(a: c_int, b: c_int) -> c_int { a - b }

#[repr(C)]
struct VTable {
    name: &'static str,
    op: BinOp,
    unary: Option<extern "C" fn(c_int) -> c_int>,
}

extern "C" fn neg(x: c_int) -> c_int { -x }

fn fold(v: &[c_int], init: c_int, f: BinOp) -> c_int {
    v.iter().fold(init, |acc, &x| f(acc, x))
}

unsafe extern "C" fn unsafe_deref(p: *const c_int) -> c_int { *p }

struct Ctx { total: i64 }

extern "C" fn visit(ctx: *mut std::os::raw::c_void, val: c_int) {
    let ctx = unsafe { &mut *(ctx as *mut Ctx) };
    ctx.total += val as i64 * 10;
}

fn for_each_c(vals: &[c_int], cb: extern "C" fn(*mut std::os::raw::c_void, c_int), data: *mut std::os::raw::c_void) {
    for &v in vals { cb(data, v); }
}

fn main() {
    let tables = [
        VTable { name: "add", op: add, unary: None },
        VTable { name: "mul", op: mul, unary: Some(neg) },
        VTable { name: "sub", op: sub, unary: Some(neg) },
    ];
    let data = [1, 2, 3, 4, 5];
    for t in &tables {
        let r = fold(&data, if t.name == "mul" { 1 } else { 0 }, t.op);
        let u = t.unary.map(|f| f(r));
        println!("{}: {} {:?}", t.name, r, u);
    }
    let x = 77;
    println!("{}", unsafe { unsafe_deref(&x) });
    let mut ctx = Ctx { total: 0 };
    for_each_c(&[1, 2, 3], visit, &mut ctx as *mut Ctx as *mut _);
    println!("total {}", ctx.total);
    let f1: BinOp = add;
    let f2: BinOp = add;
    println!("same fn eq: {}", f1 as usize == f2 as usize);
    let rust_fn: fn(i32) -> i32 = |x| x + 1;
    let fns: Vec<fn(i32) -> i32> = vec![rust_fn, |x| x * x, i32::abs];
    println!("{:?}", fns.iter().map(|f| f(-6)).collect::<Vec<_>>());
}
