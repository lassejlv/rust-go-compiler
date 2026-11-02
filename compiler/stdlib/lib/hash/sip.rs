//! SipHash-1-3 (the algorithm behind `DefaultHasher`) and SipHash-2-4.

#[derive(Clone, Copy, Debug)]
struct State {
    v0: u64,
    v2: u64,
    v1: u64,
    v3: u64,
}

#[inline]
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

/// Loads up to 7 bytes of `buf[start..start + len]` as a little-endian u64.
#[inline]
fn u8to64_le(buf: &[u8], start: usize, len: usize) -> u64 {
    let mut out: u64 = 0;
    let mut i = 0;
    while i < len {
        out |= (buf[start + i] as u64) << (8 * i as u32);
        i += 1;
    }
    out
}

/// SipHash with `C` compression rounds and `D` finalization rounds.
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

    #[inline]
    fn reset(&mut self) {
        self.length = 0;
        self.state.v0 = self.k0 ^ 0x736f6d6570736575;
        self.state.v1 = self.k1 ^ 0x646f72616e646f6d;
        self.state.v2 = self.k0 ^ 0x6c7967656e657261;
        self.state.v3 = self.k1 ^ 0x7465646279746573;
        self.ntail = 0;
    }

    #[inline]
    fn c_rounds(&mut self) {
        let mut i = 0;
        while i < self.c_rounds {
            sip_round(&mut self.state);
            i += 1;
        }
    }

    pub(crate) fn write(&mut self, msg: &[u8]) {
        let length = msg.len();
        self.length += length;
        let mut needed = 0;
        if self.ntail != 0 {
            needed = 8 - self.ntail;
            let take = if length < needed { length } else { needed };
            self.tail |= u8to64_le(msg, 0, take) << (8 * self.ntail as u32);
            if length < needed {
                self.ntail += length;
                return;
            } else {
                self.state.v3 ^= self.tail;
                self.c_rounds();
                self.state.v0 ^= self.tail;
                self.ntail = 0;
            }
        }
        let len = length - needed;
        let left = len & 0x7;
        let mut i = needed;
        while i < len - left {
            let mi = u8to64_le(msg, i, 4) | (u8to64_le(msg, i + 4, 4) << 32);
            self.state.v3 ^= mi;
            self.c_rounds();
            self.state.v0 ^= mi;
            i += 8;
        }
        self.tail = u8to64_le(msg, i, left);
        self.ntail = left;
    }

    pub(crate) fn finish(&self) -> u64 {
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
        let mut i = 0;
        while i < self.d_rounds {
            sip_round(&mut state);
            i += 1;
        }
        state.v0 ^ state.v1 ^ state.v2 ^ state.v3
    }
}
