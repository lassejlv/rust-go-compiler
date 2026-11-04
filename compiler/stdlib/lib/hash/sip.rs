//! SipHash-1-3 (the algorithm behind `DefaultHasher`) and SipHash-2-4.

use crate::intrinsics;

#[derive(Clone, Copy, Debug)]
struct State {
    v0: u64,
    v2: u64,
    v1: u64,
    v3: u64,
}

#[inline(always)]
fn sip_round(s: &mut State) {
    s.v0 = s.v0.wrapping_add(s.v1);
    s.v1 = s.v1.rotate_left(13);
    s.v1 ^= s.v0;
    s.v0 = s.v0.rotate_left(32);
    s.v2 = s.v2.wrapping_add(s.v3);
    s.v3 = s.v3.rotate_left(16);
    s.v3 ^= s.v2;
    s.v0 = s.v0.wrapping_add(s.v3);
    s.v3 = s.v3.rotate_left(21);
    s.v3 ^= s.v0;
    s.v2 = s.v2.wrapping_add(s.v1);
    s.v1 = s.v1.rotate_left(17);
    s.v1 ^= s.v2;
    s.v2 = s.v2.rotate_left(32);
}

/// Loads a little-endian u64 from 8 bytes at `p` (no alignment needed).
#[inline(always)]
unsafe fn load_u64(p: *const u8) -> u64 {
    let mut x: u64 = 0;
    intrinsics::copy_nonoverlapping(p, &mut x as *mut u64 as *mut u8, 8);
    u64::from_le(x)
}

/// Loads `len` (< 8) bytes at `p` as a little-endian u64.
#[inline(always)]
unsafe fn load_tail(p: *const u8, len: usize) -> u64 {
    let mut out: u64 = 0;
    let mut i = 0;
    if i + 3 < len {
        let mut x: u32 = 0;
        intrinsics::copy_nonoverlapping(p, &mut x as *mut u32 as *mut u8, 4);
        out = u32::from_le(x) as u64;
        i += 4;
    }
    if i + 1 < len {
        let mut x: u16 = 0;
        intrinsics::copy_nonoverlapping(intrinsics::ptr_add(p, i), &mut x as *mut u16 as *mut u8, 2);
        out |= (u16::from_le(x) as u64) << (8 * i as u32);
        i += 2;
    }
    if i < len {
        out |= (*intrinsics::ptr_add(p, i) as u64) << (8 * i as u32);
    }
    out
}

/// The multiply-fold mixing step of the fast (non-SipHash) mode.
#[inline(always)]
fn fold_mul(a: u64, b: u64) -> u64 {
    let r = (a as u128) * (b as u128);
    (r as u64) ^ ((r >> 64) as u64)
}

const FAST_K1: u64 = 0x5851f42d4c957f2d;
const FAST_K2: u64 = 0x9e3779b97f4a7c15;
const FAST_SEED: u64 = 0x243f6a8885a308d3;

/// SipHash with `C` compression rounds and `D` finalization rounds; or,
/// with `c_rounds == 0`, a fast multiply-fold hash (the hasher of
/// `RandomState`, whose keys are random in the real library, so that its
/// hash values are not observable: only `DefaultHasher::new()` and
/// `SipHasher` must compute SipHash).
#[derive(Clone, Copy, Debug)]
pub(crate) struct SipCore {
    k0: u64,
    k1: u64,
    length: usize,
    state: State,
    tail: u64,
    ntail: usize,
    c_rounds: u32,
    d_rounds: u32,
}

impl SipCore {
    #[inline]
    pub(crate) fn new(k0: u64, k1: u64, c_rounds: u32, d_rounds: u32) -> SipCore {
        let mut s = SipCore {
            k0,
            k1,
            length: 0,
            state: State { v0: 0, v1: 0, v2: 0, v3: 0 },
            tail: 0,
            ntail: 0,
            c_rounds,
            d_rounds,
        };
        s.reset();
        s
    }

    /// The fast mode (see above).
    #[inline]
    pub(crate) fn new_fast(k0: u64, k1: u64) -> SipCore {
        SipCore {
            k0,
            k1,
            length: 0,
            state: State { v0: k0 ^ FAST_SEED, v1: k1, v2: 0, v3: 0 },
            tail: 0,
            ntail: 0,
            c_rounds: 0,
            d_rounds: 0,
        }
    }

    #[inline]
    fn reset(&mut self) {
        self.length = 0;
        self.state.v0 = self.k0 ^ 0x736f6d6570736575;
        self.state.v1 = self.k1 ^ 0x646f72616e646f6d;
        self.state.v2 = self.k0 ^ 0x6c7967656e657261;
        self.state.v3 = self.k1 ^ 0x7465646279746573;
        self.ntail = 0;
    }

    /// Absorbs one message word.
    #[inline(always)]
    fn compress(&mut self, m: u64) {
        self.state.v3 ^= m;
        sip_round(&mut self.state);
        if self.c_rounds != 1 {
            let mut i = 1;
            while i < self.c_rounds {
                sip_round(&mut self.state);
                i += 1;
            }
        }
        self.state.v0 ^= m;
    }

    /// Writes the `size` (<= 8) low-order bytes of `x` (zero-extended),
    /// exactly like `write` of their little-endian bytes.
    #[inline(always)]
    pub(crate) fn short_write(&mut self, x: u64, size: usize) {
        if self.c_rounds == 0 {
            self.state.v0 = fold_mul(self.state.v0 ^ x, FAST_K1);
            return;
        }
        let nbuf = self.ntail;
        self.length += size;
        self.tail |= x << (8 * nbuf as u32);
        if nbuf + size < 8 {
            self.ntail = nbuf + size;
            return;
        }
        let m = self.tail;
        self.compress(m);
        self.ntail = nbuf + size - 8;
        self.tail = if self.ntail > 0 { x >> (8 * (size - self.ntail) as u32) } else { 0 };
    }

    pub(crate) fn write(&mut self, msg: &[u8]) {
        let length = msg.len();
        let p = msg.as_ptr();
        if self.c_rounds == 0 {
            let mut acc = fold_mul(self.state.v0 ^ length as u64, FAST_K2);
            let mut i = 0;
            while i + 8 <= length {
                acc = fold_mul(acc ^ unsafe { load_u64(intrinsics::ptr_add(p, i)) }, FAST_K1);
                i += 8;
            }
            if i < length {
                acc = fold_mul(acc ^ unsafe { load_tail(intrinsics::ptr_add(p, i), length - i) }, FAST_K1);
            }
            self.state.v0 = acc;
            return;
        }
        self.length += length;
        let mut needed = 0;
        if self.ntail != 0 {
            needed = 8 - self.ntail;
            let take = if length < needed { length } else { needed };
            self.tail |= unsafe { load_tail(p, take) } << (8 * self.ntail as u32);
            if length < needed {
                self.ntail += length;
                return;
            } else {
                let m = self.tail;
                self.compress(m);
                self.ntail = 0;
            }
        }
        let len = length - needed;
        let left = len & 0x7;
        let mut i = needed;
        while i < len - left {
            let mi = unsafe { load_u64(intrinsics::ptr_add(p, i)) };
            self.compress(mi);
            i += 8;
        }
        self.tail = unsafe { load_tail(intrinsics::ptr_add(p, i), left) };
        self.ntail = left;
    }

    #[inline]
    pub(crate) fn finish(&self) -> u64 {
        if self.c_rounds == 0 {
            return fold_mul(self.state.v0, FAST_K2);
        }
        let mut state = self.state;
        let b: u64 = (((self.length as u64) & 0xff) << 56) | self.tail;
        state.v3 ^= b;
        let mut i = 0;
        while i < self.c_rounds {
            sip_round(&mut state);
            i += 1;
        }
        state.v0 ^= b;
        state.v2 ^= 0xff;
        sip_round(&mut state);
        sip_round(&mut state);
        sip_round(&mut state);
        let mut i = 3;
        while i < self.d_rounds {
            sip_round(&mut state);
            i += 1;
        }
        state.v0 ^ state.v1 ^ state.v2 ^ state.v3
    }
}
