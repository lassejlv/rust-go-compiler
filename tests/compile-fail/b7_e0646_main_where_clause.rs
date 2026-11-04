//~ ERROR E0646
// `main` cannot have a where clause.
fn main()
where
    i32: Copy,
{
}
