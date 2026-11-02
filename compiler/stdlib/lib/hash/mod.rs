//! Generic hashing support.

mod sip;

use crate::fmt;
use crate::marker::PhantomData;

/// A hashable type.
pub trait Hash {
    /// Feeds this value into the given `Hasher`.
    fn hash<H: Hasher>(&self, state: &mut H);

    /// Feeds a slice of this type into the given `Hasher`.
    fn hash_slice<H: Hasher>(data: &[Self], state: &mut H)
    where
        Self: Sized,
    {
        let n = data.len();
        let mut i = 0;
        while i < n {
            data[i].hash(state);
            i += 1;
        }
    }
}

/// A trait for hashing an arbitrary stream of bytes.
pub trait Hasher {
    /// Returns the hash value for the values written so far.
    fn finish(&self) -> u64;

    /// Writes some data into this `Hasher`.
    fn write(&mut self, bytes: &[u8]);

    /// Writes a single `u8` into this hasher.
    #[inline]
    fn write_u8(&mut self, i: u8) {
        self.write(&[i])
    }
    /// Writes a single `u16` into this hasher.
    #[inline]
    fn write_u16(&mut self, i: u16) {
        self.write(&i.to_ne_bytes())
    }
    /// Writes a single `u32` into this hasher.
    #[inline]
    fn write_u32(&mut self, i: u32) {
        self.write(&i.to_ne_bytes())
    }
    /// Writes a single `u64` into this hasher.
    #[inline]
    fn write_u64(&mut self, i: u64) {
        self.write(&i.to_ne_bytes())
    }
    /// Writes a single `u128` into this hasher.
    #[inline]
    fn write_u128(&mut self, i: u128) {
        self.write(&i.to_ne_bytes())
    }
    /// Writes a single `usize` into this hasher.
    #[inline]
    fn write_usize(&mut self, i: usize) {
        self.write(&i.to_ne_bytes())
    }
    /// Writes a single `i8` into this hasher.
    #[inline]
    fn write_i8(&mut self, i: i8) {
        self.write_u8(i as u8)
    }
    /// Writes a single `i16` into this hasher.
    #[inline]
    fn write_i16(&mut self, i: i16) {
        self.write_u16(i as u16)
    }
    /// Writes a single `i32` into this hasher.
    #[inline]
    fn write_i32(&mut self, i: i32) {
        self.write_u32(i as u32)
    }
    /// Writes a single `i64` into this hasher.
    #[inline]
    fn write_i64(&mut self, i: i64) {
        self.write_u64(i as u64)
    }
    /// Writes a single `i128` into this hasher.
    #[inline]
    fn write_i128(&mut self, i: i128) {
        self.write_u128(i as u128)
    }
    /// Writes a single `isize` into this hasher.
    #[inline]
    fn write_isize(&mut self, i: isize) {
        self.write_usize(i as usize)
    }
    /// Writes a length prefix into this hasher.
    #[inline]
    fn write_length_prefix(&mut self, len: usize) {
        self.write_usize(len);
    }
    /// Writes a single `str` into this hasher.
    #[inline]
    fn write_str(&mut self, s: &str) {
        self.write(s.as_bytes());
        self.write_u8(0xff);
    }
}

impl<H: Hasher + ?Sized> Hasher for &mut H {
    fn finish(&self) -> u64 {
        (**self).finish()
    }
    fn write(&mut self, bytes: &[u8]) {
        (**self).write(bytes)
    }
    fn write_u8(&mut self, i: u8) {
        (**self).write_u8(i)
    }
    fn write_u16(&mut self, i: u16) {
        (**self).write_u16(i)
    }
    fn write_u32(&mut self, i: u32) {
        (**self).write_u32(i)
    }
    fn write_u64(&mut self, i: u64) {
        (**self).write_u64(i)
    }
    fn write_u128(&mut self, i: u128) {
        (**self).write_u128(i)
    }
    fn write_usize(&mut self, i: usize) {
        (**self).write_usize(i)
    }
    fn write_i8(&mut self, i: i8) {
        (**self).write_i8(i)
    }
    fn write_i16(&mut self, i: i16) {
        (**self).write_i16(i)
    }
    fn write_i32(&mut self, i: i32) {
        (**self).write_i32(i)
    }
    fn write_i64(&mut self, i: i64) {
        (**self).write_i64(i)
    }
    fn write_i128(&mut self, i: i128) {
        (**self).write_i128(i)
    }
    fn write_isize(&mut self, i: isize) {
        (**self).write_isize(i)
    }
    fn write_length_prefix(&mut self, len: usize) {
        (**self).write_length_prefix(len)
    }
    fn write_str(&mut self, s: &str) {
        (**self).write_str(s)
    }
}

/// A trait for creating instances of `Hasher`.
pub trait BuildHasher {
    /// Type of the hasher that will be created.
    type Hasher: Hasher;

    /// Creates a new hasher.
    fn build_hasher(&self) -> Self::Hasher;

    /// Calculates the hash of a single value.
    fn hash_one<T: Hash>(&self, x: T) -> u64
    where
        Self: Sized,
    {
        let mut hasher = self.build_hasher();
        x.hash(&mut hasher);
        hasher.finish()
    }
}

/// Used to create a default `BuildHasher` instance for types that implement `Hasher` and `Default`.
pub struct BuildHasherDefault<H>(PhantomData<H>);

impl<H> BuildHasherDefault<H> {
    /// Creates a new BuildHasherDefault for Hasher `H`.
    pub const fn new() -> Self {
        BuildHasherDefault(PhantomData)
    }
}

impl<H: Default + Hasher> BuildHasher for BuildHasherDefault<H> {
    type Hasher = H;
    fn build_hasher(&self) -> H {
        H::default()
    }
}

impl<H> Clone for BuildHasherDefault<H> {
    fn clone(&self) -> BuildHasherDefault<H> {
        BuildHasherDefault(PhantomData)
    }
}

impl<H> Default for BuildHasherDefault<H> {
    fn default() -> BuildHasherDefault<H> {
        BuildHasherDefault(PhantomData)
    }
}

impl<H> fmt::Debug for BuildHasherDefault<H> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BuildHasherDefault").finish()
    }
}

/// The default `Hasher` used by `RandomState` (SipHash-1-3).
#[derive(Clone, Debug)]
pub struct DefaultHasher(sip::SipCore);

impl DefaultHasher {
    /// Creates a new `DefaultHasher`.
    #[inline]
    pub fn new() -> DefaultHasher {
        DefaultHasher(sip::SipCore::new(0, 0, 1, 3))
    }
}

impl Default for DefaultHasher {
    #[inline]
    fn default() -> DefaultHasher {
        DefaultHasher::new()
    }
}

impl Hasher for DefaultHasher {
    #[inline]
    fn write(&mut self, msg: &[u8]) {
        self.0.write(msg)
    }
    #[inline]
    fn write_str(&mut self, s: &str) {
        self.0.write(s.as_bytes());
        self.0.write(&[0xff]);
    }
    #[inline]
    fn finish(&self) -> u64 {
        self.0.finish()
    }
}

/// An implementation of SipHash 2-4.
#[derive(Debug, Clone)]
pub struct SipHasher(sip::SipCore);

impl SipHasher {
    /// Creates a new `SipHasher` with the two initial keys set to 0.
    #[inline]
    pub fn new() -> SipHasher {
        SipHasher::new_with_keys(0, 0)
    }

    /// Creates a `SipHasher` that is keyed off the provided keys.
    #[inline]
    pub fn new_with_keys(key0: u64, key1: u64) -> SipHasher {
        SipHasher(sip::SipCore::new(key0, key1, 2, 4))
    }
}

impl Default for SipHasher {
    fn default() -> SipHasher {
        SipHasher::new()
    }
}

impl Hasher for SipHasher {
    #[inline]
    fn write(&mut self, msg: &[u8]) {
        self.0.write(msg)
    }
    #[inline]
    fn finish(&self) -> u64 {
        self.0.finish()
    }
}

/// `RandomState` is the default state for `HashMap` types (deterministic keys here).
#[derive(Clone)]
pub struct RandomState {
    k0: u64,
    k1: u64,
}

impl RandomState {
    /// Constructs a new `RandomState`.
    #[inline]
    pub fn new() -> RandomState {
        RandomState { k0: 0, k1: 0 }
    }
}

impl BuildHasher for RandomState {
    type Hasher = DefaultHasher;
    #[inline]
    fn build_hasher(&self) -> DefaultHasher {
        DefaultHasher(sip::SipCore::new(self.k0, self.k1, 1, 3))
    }
}

impl Default for RandomState {
    #[inline]
    fn default() -> RandomState {
        RandomState::new()
    }
}

impl fmt::Debug for RandomState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RandomState").finish_non_exhaustive()
    }
}

// ---------------------------------------------------------------------------
// Implementations for primitive types

macro_rules! __std_hash_int {
    ($($t:ty => $meth:ident),* $(,)?) => {
        $(
            impl Hash for $t {
                #[inline]
                fn hash<H: Hasher>(&self, state: &mut H) {
                    state.$meth(*self)
                }
                #[inline]
                fn hash_slice<H: Hasher>(data: &[$t], state: &mut H) {
                    let n = data.len();
                    let mut i = 0;
                    while i < n {
                        state.$meth(data[i]);
                        i += 1;
                    }
                }
            }
        )*
    };
}

__std_hash_int! {
    u8 => write_u8, u16 => write_u16, u32 => write_u32, u64 => write_u64, u128 => write_u128, usize => write_usize,
    i8 => write_i8, i16 => write_i16, i32 => write_i32, i64 => write_i64, i128 => write_i128, isize => write_isize,
}

impl Hash for bool {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u8(*self as u8)
    }
}

impl Hash for char {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u32(*self as u32)
    }
}

impl Hash for str {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_str(self);
    }
}

impl Hash for () {
    #[inline]
    fn hash<H: Hasher>(&self, _state: &mut H) {}
}

impl<T: Hash> Hash for [T] {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_length_prefix(self.len());
        Hash::hash_slice(self, state)
    }
}

impl<T: ?Sized + Hash> Hash for &T {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        (**self).hash(state);
    }
}

impl<T: ?Sized + Hash> Hash for &mut T {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        (**self).hash(state);
    }
}
