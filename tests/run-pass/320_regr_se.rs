#[derive(Clone, Copy, Debug)]
struct Tag(u8);
impl Tag { const EMPTY: Tag = Tag(0xFF); }
struct Group(u64);
impl Group {
    const WIDTH: usize = std::mem::size_of::<Self>();
    const fn static_empty() -> &'static [Tag; Group::WIDTH] {
        #[repr(C)]
        struct AlignedTags { _align: [Group; 0], tags: [Tag; Group::WIDTH] }
        const ALIGNED_TAGS: AlignedTags = AlignedTags { _align: [], tags: [Tag::EMPTY; Group::WIDTH] };
        &ALIGNED_TAGS.tags
    }
}
fn main() {
    let e = Group::static_empty();
    println!("{:?}", e);
    let p = e.as_ptr() as usize;
    println!("{} {:?}", p % 8, unsafe { std::ptr::read_unaligned(e.as_ptr() as *const u64) });
}
