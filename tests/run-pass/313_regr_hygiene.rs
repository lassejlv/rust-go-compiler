macro_rules! clos {
    ( @closure $p:pat => $tup:expr ) => { |$p| $tup };
    ( @closure $p:pat => ( $($tup:tt)* ) , $_iter:expr $( , $tail:expr )* ) => {
        clos!(@closure ($p, b) => ( $($tup)*, b ) $( , $tail )*)
    };
}
fn main() {
    let f = clos!(@closure a => (a), 0, 0);
    println!("{:?}", f(((1, 'x'), true)));
}
