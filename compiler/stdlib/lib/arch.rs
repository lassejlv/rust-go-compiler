//! SIMD and vendor intrinsics module.
//!
//! rustgo has no SIMD backend: the x86_64 vector types and intrinsics are
//! emulated lane by lane in portable Rust (with the same results as the real
//! instructions). No `target_feature` cfg is set, so crates normally select
//! their scalar fallbacks; these definitions exist so that code which merely
//! mentions the intrinsics compiles (and works if it is called).

/// Platform-specific intrinsics for the `x86_64` platform.
pub mod x86_64 {
    use crate::fmt;
    use crate::intrinsics;

    macro_rules! vector_type {
        ($(#[$m:meta])* $name:ident, $bytes:expr) => {
            $(#[$m])*
            // NOTE: real vector types are 16/32-byte aligned; rustgo does not
            // implement `#[repr(align)]`, so these are byte-aligned.
            #[derive(Copy, Clone)]
            #[repr(C)]
            #[allow(non_camel_case_types)]
            pub struct $name {
                bytes: [u8; $bytes],
            }

            impl $name {
                #[inline]
                fn from_bytes(bytes: [u8; $bytes]) -> $name {
                    $name { bytes }
                }
            }

            impl fmt::Debug for $name {
                fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    f.debug_tuple(stringify!($name)).field(&self.bytes).finish()
                }
            }
        };
    }

    vector_type!(
        /// 128-bit wide integer vector type, x86-specific.
        __m128i, 16
    );
    vector_type!(
        /// 128-bit wide set of four `f32` types, x86-specific.
        __m128, 16
    );
    vector_type!(
        /// 128-bit wide set of two `f64` types, x86-specific.
        __m128d, 16
    );
    vector_type!(
        /// 256-bit wide integer vector type, x86-specific.
        __m256i, 32
    );
    vector_type!(
        /// 256-bit wide set of eight `f32` types, x86-specific.
        __m256, 32
    );
    vector_type!(
        /// 256-bit wide set of four `f64` types, x86-specific.
        __m256d, 32
    );

    /// Shared lane-wise operations of the 128-bit and 256-bit integer vectors.
    macro_rules! int_vector_ops {
        ($v:ident, $n:expr,
         set1_epi8 = $set1_epi8:ident, set1_epi16 = $set1_epi16:ident, set1_epi32 = $set1_epi32:ident,
         set1_epi64x = $set1_epi64x:ident, setzero = $setzero:ident,
         load = $load:ident, loadu = $loadu:ident, store = $store:ident, storeu = $storeu:ident,
         cmpeq_epi8 = $cmpeq_epi8:ident, cmpgt_epi8 = $cmpgt_epi8:ident,
         movemask_epi8 = $movemask_epi8:ident,
         and = $and:ident, or = $or:ident, xor = $xor:ident, andnot = $andnot:ident,
         add_epi8 = $add_epi8:ident, sub_epi8 = $sub_epi8:ident, add_epi64 = $add_epi64:ident,
         min_epu8 = $min_epu8:ident, max_epu8 = $max_epu8:ident,
         adds_epu8 = $adds_epu8:ident, subs_epu8 = $subs_epu8:ident,
         srli_epi16 = $srli_epi16:ident, slli_epi16 = $slli_epi16:ident,
         shuffle_epi8 = $shuffle_epi8:ident, alignr_epi8 = $alignr_epi8:ident,
         unpacklo_epi8 = $unpacklo_epi8:ident, unpackhi_epi8 = $unpackhi_epi8:ident,
         extract_epi8 = $extract_epi8:ident, extract_epi64 = $extract_epi64:ident) => {
            /// Broadcasts 8-bit integer `a` to all elements.
            #[inline]
            pub unsafe fn $set1_epi8(a: i8) -> $v {
                $v::from_bytes([a as u8; $n])
            }

            /// Broadcasts 16-bit integer `a` to all elements.
            #[inline]
            pub unsafe fn $set1_epi16(a: i16) -> $v {
                let mut b = [0u8; $n];
                let le = a.to_le_bytes();
                let mut i = 0;
                while i < $n {
                    b[i] = le[i % 2];
                    i += 1;
                }
                $v::from_bytes(b)
            }

            /// Broadcasts 32-bit integer `a` to all elements.
            #[inline]
            pub unsafe fn $set1_epi32(a: i32) -> $v {
                let mut b = [0u8; $n];
                let le = a.to_le_bytes();
                let mut i = 0;
                while i < $n {
                    b[i] = le[i % 4];
                    i += 1;
                }
                $v::from_bytes(b)
            }

            /// Broadcasts 64-bit integer `a` to all elements.
            #[inline]
            pub unsafe fn $set1_epi64x(a: i64) -> $v {
                let mut b = [0u8; $n];
                let le = a.to_le_bytes();
                let mut i = 0;
                while i < $n {
                    b[i] = le[i % 8];
                    i += 1;
                }
                $v::from_bytes(b)
            }

            /// Returns a vector with all elements set to zero.
            #[inline]
            pub unsafe fn $setzero() -> $v {
                $v::from_bytes([0u8; $n])
            }

            /// Loads integer data from memory (aligned).
            #[inline]
            pub unsafe fn $load(mem_addr: *const $v) -> $v {
                intrinsics::read(mem_addr)
            }

            /// Loads integer data from memory (unaligned).
            #[inline]
            pub unsafe fn $loadu(mem_addr: *const $v) -> $v {
                let mut b = [0u8; $n];
                intrinsics::copy_nonoverlapping(mem_addr as *const u8, b.as_mut_ptr(), $n);
                $v::from_bytes(b)
            }

            /// Stores integer data into memory (aligned).
            #[inline]
            pub unsafe fn $store(mem_addr: *mut $v, a: $v) {
                intrinsics::write(mem_addr, a)
            }

            /// Stores integer data into memory (unaligned).
            #[inline]
            pub unsafe fn $storeu(mem_addr: *mut $v, a: $v) {
                intrinsics::copy_nonoverlapping(a.bytes.as_ptr(), mem_addr as *mut u8, $n);
            }

            /// Compares packed 8-bit integers for equality.
            #[inline]
            pub unsafe fn $cmpeq_epi8(a: $v, b: $v) -> $v {
                let mut r = [0u8; $n];
                let mut i = 0;
                while i < $n {
                    r[i] = if a.bytes[i] == b.bytes[i] { 0xFF } else { 0 };
                    i += 1;
                }
                $v::from_bytes(r)
            }

            /// Compares packed signed 8-bit integers for greater-than.
            #[inline]
            pub unsafe fn $cmpgt_epi8(a: $v, b: $v) -> $v {
                let mut r = [0u8; $n];
                let mut i = 0;
                while i < $n {
                    r[i] = if (a.bytes[i] as i8) > (b.bytes[i] as i8) { 0xFF } else { 0 };
                    i += 1;
                }
                $v::from_bytes(r)
            }

            /// Returns a mask of the most significant bit of each element.
            #[inline]
            pub unsafe fn $movemask_epi8(a: $v) -> i32 {
                let mut m: u32 = 0;
                let mut i = 0;
                while i < $n {
                    m |= ((a.bytes[i] >> 7) as u32) << i;
                    i += 1;
                }
                m as i32
            }

            /// Computes the bitwise AND.
            #[inline]
            pub unsafe fn $and(a: $v, b: $v) -> $v {
                let mut r = [0u8; $n];
                let mut i = 0;
                while i < $n {
                    r[i] = a.bytes[i] & b.bytes[i];
                    i += 1;
                }
                $v::from_bytes(r)
            }

            /// Computes the bitwise OR.
            #[inline]
            pub unsafe fn $or(a: $v, b: $v) -> $v {
                let mut r = [0u8; $n];
                let mut i = 0;
                while i < $n {
                    r[i] = a.bytes[i] | b.bytes[i];
                    i += 1;
                }
                $v::from_bytes(r)
            }

            /// Computes the bitwise XOR.
            #[inline]
            pub unsafe fn $xor(a: $v, b: $v) -> $v {
                let mut r = [0u8; $n];
                let mut i = 0;
                while i < $n {
                    r[i] = a.bytes[i] ^ b.bytes[i];
                    i += 1;
                }
                $v::from_bytes(r)
            }

            /// Computes the bitwise NOT of `a` and then AND with `b`.
            #[inline]
            pub unsafe fn $andnot(a: $v, b: $v) -> $v {
                let mut r = [0u8; $n];
                let mut i = 0;
                while i < $n {
                    r[i] = !a.bytes[i] & b.bytes[i];
                    i += 1;
                }
                $v::from_bytes(r)
            }

            /// Adds packed 8-bit integers (wrapping).
            #[inline]
            pub unsafe fn $add_epi8(a: $v, b: $v) -> $v {
                let mut r = [0u8; $n];
                let mut i = 0;
                while i < $n {
                    r[i] = a.bytes[i].wrapping_add(b.bytes[i]);
                    i += 1;
                }
                $v::from_bytes(r)
            }

            /// Subtracts packed 8-bit integers (wrapping).
            #[inline]
            pub unsafe fn $sub_epi8(a: $v, b: $v) -> $v {
                let mut r = [0u8; $n];
                let mut i = 0;
                while i < $n {
                    r[i] = a.bytes[i].wrapping_sub(b.bytes[i]);
                    i += 1;
                }
                $v::from_bytes(r)
            }

            /// Adds packed 64-bit integers (wrapping).
            #[inline]
            pub unsafe fn $add_epi64(a: $v, b: $v) -> $v {
                let mut r = [0u8; $n];
                let mut i = 0;
                while i < $n {
                    let mut x = [0u8; 8];
                    let mut y = [0u8; 8];
                    x.copy_from_slice(&a.bytes[i..i + 8]);
                    y.copy_from_slice(&b.bytes[i..i + 8]);
                    let s = u64::from_le_bytes(x).wrapping_add(u64::from_le_bytes(y));
                    r[i..i + 8].copy_from_slice(&s.to_le_bytes());
                    i += 8;
                }
                $v::from_bytes(r)
            }

            /// Compares packed unsigned 8-bit integers and returns the minimums.
            #[inline]
            pub unsafe fn $min_epu8(a: $v, b: $v) -> $v {
                let mut r = [0u8; $n];
                let mut i = 0;
                while i < $n {
                    r[i] = if a.bytes[i] < b.bytes[i] { a.bytes[i] } else { b.bytes[i] };
                    i += 1;
                }
                $v::from_bytes(r)
            }

            /// Compares packed unsigned 8-bit integers and returns the maximums.
            #[inline]
            pub unsafe fn $max_epu8(a: $v, b: $v) -> $v {
                let mut r = [0u8; $n];
                let mut i = 0;
                while i < $n {
                    r[i] = if a.bytes[i] > b.bytes[i] { a.bytes[i] } else { b.bytes[i] };
                    i += 1;
                }
                $v::from_bytes(r)
            }

            /// Adds packed unsigned 8-bit integers using saturation.
            #[inline]
            pub unsafe fn $adds_epu8(a: $v, b: $v) -> $v {
                let mut r = [0u8; $n];
                let mut i = 0;
                while i < $n {
                    r[i] = a.bytes[i].saturating_add(b.bytes[i]);
                    i += 1;
                }
                $v::from_bytes(r)
            }

            /// Subtracts packed unsigned 8-bit integers using saturation.
            #[inline]
            pub unsafe fn $subs_epu8(a: $v, b: $v) -> $v {
                let mut r = [0u8; $n];
                let mut i = 0;
                while i < $n {
                    r[i] = a.bytes[i].saturating_sub(b.bytes[i]);
                    i += 1;
                }
                $v::from_bytes(r)
            }

            /// Shifts packed 16-bit integers right by `imm8` while shifting in zeros.
            #[inline]
            pub unsafe fn $srli_epi16(a: $v, imm8: i32) -> $v {
                let mut r = [0u8; $n];
                let mut i = 0;
                while i < $n {
                    let x = u16::from_le_bytes([a.bytes[i], a.bytes[i + 1]]);
                    let y = if imm8 < 0 || imm8 > 15 { 0 } else { x >> (imm8 as u32) };
                    let le = y.to_le_bytes();
                    r[i] = le[0];
                    r[i + 1] = le[1];
                    i += 2;
                }
                $v::from_bytes(r)
            }

            /// Shifts packed 16-bit integers left by `imm8` while shifting in zeros.
            #[inline]
            pub unsafe fn $slli_epi16(a: $v, imm8: i32) -> $v {
                let mut r = [0u8; $n];
                let mut i = 0;
                while i < $n {
                    let x = u16::from_le_bytes([a.bytes[i], a.bytes[i + 1]]);
                    let y = if imm8 < 0 || imm8 > 15 { 0 } else { x << (imm8 as u32) };
                    let le = y.to_le_bytes();
                    r[i] = le[0];
                    r[i + 1] = le[1];
                    i += 2;
                }
                $v::from_bytes(r)
            }

            /// Shuffles bytes from `a` according to the content of `b` (within 128-bit lanes).
            #[inline]
            pub unsafe fn $shuffle_epi8(a: $v, b: $v) -> $v {
                let mut r = [0u8; $n];
                let mut i = 0;
                while i < $n {
                    let lane = i - i % 16;
                    let c = b.bytes[i];
                    r[i] = if c & 0x80 != 0 { 0 } else { a.bytes[lane + (c & 0x0F) as usize] };
                    i += 1;
                }
                $v::from_bytes(r)
            }

            /// Concatenates pairs of 16-byte blocks in `a` and `b` into a 32-byte temporary
            /// result, shifts the result right by `n` bytes, and returns the low 16 bytes
            /// (within 128-bit lanes).
            #[inline]
            pub unsafe fn $alignr_epi8(a: $v, b: $v, n: i32) -> $v {
                let mut r = [0u8; $n];
                let n = n as u32 as usize;
                let mut lane = 0;
                while lane < $n {
                    let mut i = 0;
                    while i < 16 {
                        let k = i + n;
                        r[lane + i] = if k < 16 {
                            b.bytes[lane + k]
                        } else if k < 32 {
                            a.bytes[lane + k - 16]
                        } else {
                            0
                        };
                        i += 1;
                    }
                    lane += 16;
                }
                $v::from_bytes(r)
            }

            /// Unpacks and interleaves 8-bit integers from the low half of each 128-bit lane.
            #[inline]
            pub unsafe fn $unpacklo_epi8(a: $v, b: $v) -> $v {
                let mut r = [0u8; $n];
                let mut lane = 0;
                while lane < $n {
                    let mut i = 0;
                    while i < 8 {
                        r[lane + 2 * i] = a.bytes[lane + i];
                        r[lane + 2 * i + 1] = b.bytes[lane + i];
                        i += 1;
                    }
                    lane += 16;
                }
                $v::from_bytes(r)
            }

            /// Unpacks and interleaves 8-bit integers from the high half of each 128-bit lane.
            #[inline]
            pub unsafe fn $unpackhi_epi8(a: $v, b: $v) -> $v {
                let mut r = [0u8; $n];
                let mut lane = 0;
                while lane < $n {
                    let mut i = 0;
                    while i < 8 {
                        r[lane + 2 * i] = a.bytes[lane + 8 + i];
                        r[lane + 2 * i + 1] = b.bytes[lane + 8 + i];
                        i += 1;
                    }
                    lane += 16;
                }
                $v::from_bytes(r)
            }

            /// Extracts an 8-bit integer (zero-extended) at index `imm8`.
            #[inline]
            pub unsafe fn $extract_epi8(a: $v, imm8: i32) -> i32 {
                a.bytes[(imm8 as u32 as usize) % $n] as i32
            }

            /// Extracts a 64-bit integer at index `imm8`.
            #[inline]
            pub unsafe fn $extract_epi64(a: $v, imm8: i32) -> i64 {
                let i = ((imm8 as u32 as usize) % ($n / 8)) * 8;
                let mut x = [0u8; 8];
                x.copy_from_slice(&a.bytes[i..i + 8]);
                i64::from_le_bytes(x)
            }
        };
    }

    int_vector_ops!(__m128i, 16,
        set1_epi8 = _mm_set1_epi8, set1_epi16 = _mm_set1_epi16, set1_epi32 = _mm_set1_epi32,
        set1_epi64x = _mm_set1_epi64x, setzero = _mm_setzero_si128,
        load = _mm_load_si128, loadu = _mm_loadu_si128, store = _mm_store_si128, storeu = _mm_storeu_si128,
        cmpeq_epi8 = _mm_cmpeq_epi8, cmpgt_epi8 = _mm_cmpgt_epi8,
        movemask_epi8 = _mm_movemask_epi8,
        and = _mm_and_si128, or = _mm_or_si128, xor = _mm_xor_si128, andnot = _mm_andnot_si128,
        add_epi8 = _mm_add_epi8, sub_epi8 = _mm_sub_epi8, add_epi64 = _mm_add_epi64,
        min_epu8 = _mm_min_epu8, max_epu8 = _mm_max_epu8,
        adds_epu8 = _mm_adds_epu8, subs_epu8 = _mm_subs_epu8,
        srli_epi16 = _mm_srli_epi16, slli_epi16 = _mm_slli_epi16,
        shuffle_epi8 = _mm_shuffle_epi8, alignr_epi8 = _mm_alignr_epi8,
        unpacklo_epi8 = _mm_unpacklo_epi8, unpackhi_epi8 = _mm_unpackhi_epi8,
        extract_epi8 = _mm_extract_epi8, extract_epi64 = _mm_extract_epi64);

    int_vector_ops!(__m256i, 32,
        set1_epi8 = _mm256_set1_epi8, set1_epi16 = _mm256_set1_epi16, set1_epi32 = _mm256_set1_epi32,
        set1_epi64x = _mm256_set1_epi64x, setzero = _mm256_setzero_si256,
        load = _mm256_load_si256, loadu = _mm256_loadu_si256, store = _mm256_store_si256, storeu = _mm256_storeu_si256,
        cmpeq_epi8 = _mm256_cmpeq_epi8, cmpgt_epi8 = _mm256_cmpgt_epi8,
        movemask_epi8 = _mm256_movemask_epi8,
        and = _mm256_and_si256, or = _mm256_or_si256, xor = _mm256_xor_si256, andnot = _mm256_andnot_si256,
        add_epi8 = _mm256_add_epi8, sub_epi8 = _mm256_sub_epi8, add_epi64 = _mm256_add_epi64,
        min_epu8 = _mm256_min_epu8, max_epu8 = _mm256_max_epu8,
        adds_epu8 = _mm256_adds_epu8, subs_epu8 = _mm256_subs_epu8,
        srli_epi16 = _mm256_srli_epi16, slli_epi16 = _mm256_slli_epi16,
        shuffle_epi8 = _mm256_shuffle_epi8, alignr_epi8 = _mm256_alignr_epi8,
        unpacklo_epi8 = _mm256_unpacklo_epi8, unpackhi_epi8 = _mm256_unpackhi_epi8,
        extract_epi8 = _mm256_extract_epi8, extract_epi64 = _mm256_extract_epi64);

    // ---- lane helpers (little-endian lanes)
    #[inline]
    fn rd16(b: &[u8], i: usize) -> i16 {
        i16::from_le_bytes([b[2 * i], b[2 * i + 1]])
    }
    #[inline]
    fn wr16(b: &mut [u8], i: usize, v: i16) {
        b[2 * i..2 * i + 2].copy_from_slice(&v.to_le_bytes());
    }
    #[inline]
    fn rd32(b: &[u8], i: usize) -> i32 {
        i32::from_le_bytes([b[4 * i], b[4 * i + 1], b[4 * i + 2], b[4 * i + 3]])
    }
    #[inline]
    fn wr32(b: &mut [u8], i: usize, v: i32) {
        b[4 * i..4 * i + 4].copy_from_slice(&v.to_le_bytes());
    }
    #[inline]
    fn rd64(b: &[u8], i: usize) -> u64 {
        let mut x = [0u8; 8];
        x.copy_from_slice(&b[8 * i..8 * i + 8]);
        u64::from_le_bytes(x)
    }
    #[inline]
    fn wr64(b: &mut [u8], i: usize, v: u64) {
        b[8 * i..8 * i + 8].copy_from_slice(&v.to_le_bytes());
    }

    macro_rules! lane32_ops {
        ($v:ident, $n:expr, add = $add:ident, sub = $sub:ident, slli = $slli:ident, srli = $srli:ident,
         add16 = $add16:ident, madd = $madd:ident, maddubs = $maddubs:ident, sad = $sad:ident,
         set1_64 = $unused:ident) => {
            /// Adds packed 32-bit integers (wrapping).
            #[inline]
            pub unsafe fn $add(a: $v, b: $v) -> $v {
                let mut r = [0u8; $n];
                let mut i = 0;
                while i < $n / 4 {
                    wr32(&mut r, i, rd32(&a.bytes, i).wrapping_add(rd32(&b.bytes, i)));
                    i += 1;
                }
                $v::from_bytes(r)
            }

            /// Subtracts packed 32-bit integers (wrapping).
            #[inline]
            pub unsafe fn $sub(a: $v, b: $v) -> $v {
                let mut r = [0u8; $n];
                let mut i = 0;
                while i < $n / 4 {
                    wr32(&mut r, i, rd32(&a.bytes, i).wrapping_sub(rd32(&b.bytes, i)));
                    i += 1;
                }
                $v::from_bytes(r)
            }

            /// Adds packed 16-bit integers (wrapping).
            #[inline]
            pub unsafe fn $add16(a: $v, b: $v) -> $v {
                let mut r = [0u8; $n];
                let mut i = 0;
                while i < $n / 2 {
                    wr16(&mut r, i, rd16(&a.bytes, i).wrapping_add(rd16(&b.bytes, i)));
                    i += 1;
                }
                $v::from_bytes(r)
            }

            /// Shifts packed 32-bit integers left by `imm8` while shifting in zeros.
            #[inline]
            pub unsafe fn $slli(a: $v, imm8: i32) -> $v {
                let mut r = [0u8; $n];
                let mut i = 0;
                while i < $n / 4 {
                    let x = rd32(&a.bytes, i) as u32;
                    let y = if imm8 < 0 || imm8 > 31 { 0 } else { x << (imm8 as u32) };
                    wr32(&mut r, i, y as i32);
                    i += 1;
                }
                $v::from_bytes(r)
            }

            /// Shifts packed 32-bit integers right by `imm8` while shifting in zeros.
            #[inline]
            pub unsafe fn $srli(a: $v, imm8: i32) -> $v {
                let mut r = [0u8; $n];
                let mut i = 0;
                while i < $n / 4 {
                    let x = rd32(&a.bytes, i) as u32;
                    let y = if imm8 < 0 || imm8 > 31 { 0 } else { x >> (imm8 as u32) };
                    wr32(&mut r, i, y as i32);
                    i += 1;
                }
                $v::from_bytes(r)
            }

            /// Multiplies packed signed 16-bit integers, producing intermediate signed 32-bit
            /// integers, and horizontally adds adjacent pairs.
            #[inline]
            pub unsafe fn $madd(a: $v, b: $v) -> $v {
                let mut r = [0u8; $n];
                let mut i = 0;
                while i < $n / 4 {
                    let p0 = rd16(&a.bytes, 2 * i) as i32 * rd16(&b.bytes, 2 * i) as i32;
                    let p1 = rd16(&a.bytes, 2 * i + 1) as i32 * rd16(&b.bytes, 2 * i + 1) as i32;
                    wr32(&mut r, i, p0.wrapping_add(p1));
                    i += 1;
                }
                $v::from_bytes(r)
            }

            /// Multiplies unsigned 8-bit integers in `a` with signed 8-bit integers in `b`,
            /// and horizontally adds adjacent pairs with signed saturation.
            #[inline]
            pub unsafe fn $maddubs(a: $v, b: $v) -> $v {
                let mut r = [0u8; $n];
                let mut i = 0;
                while i < $n / 2 {
                    let p0 = a.bytes[2 * i] as i32 * (b.bytes[2 * i] as i8) as i32;
                    let p1 = a.bytes[2 * i + 1] as i32 * (b.bytes[2 * i + 1] as i8) as i32;
                    let s = p0 + p1;
                    let s = if s > i16::MAX as i32 { i16::MAX as i32 } else if s < i16::MIN as i32 { i16::MIN as i32 } else { s };
                    wr16(&mut r, i, s as i16);
                    i += 1;
                }
                $v::from_bytes(r)
            }

            /// Computes the absolute differences of packed unsigned 8-bit integers and sums
            /// each consecutive 8 differences into the low 16 bits of a 64-bit element.
            #[inline]
            pub unsafe fn $sad(a: $v, b: $v) -> $v {
                let mut r = [0u8; $n];
                let mut g = 0;
                while g < $n / 8 {
                    let mut sum = 0u64;
                    let mut k = 0;
                    while k < 8 {
                        let x = a.bytes[8 * g + k];
                        let y = b.bytes[8 * g + k];
                        sum += if x > y { (x - y) as u64 } else { (y - x) as u64 };
                        k += 1;
                    }
                    wr64(&mut r, g, sum);
                    g += 1;
                }
                $v::from_bytes(r)
            }
        };
    }

    lane32_ops!(__m128i, 16, add = _mm_add_epi32, sub = _mm_sub_epi32, slli = _mm_slli_epi32, srli = _mm_srli_epi32,
        add16 = _mm_add_epi16, madd = _mm_madd_epi16, maddubs = _mm_maddubs_epi16, sad = _mm_sad_epu8, set1_64 = _x);
    lane32_ops!(__m256i, 32, add = _mm256_add_epi32, sub = _mm256_sub_epi32, slli = _mm256_slli_epi32,
        srli = _mm256_srli_epi32, add16 = _mm256_add_epi16, madd = _mm256_madd_epi16,
        maddubs = _mm256_maddubs_epi16, sad = _mm256_sad_epu8, set1_64 = _x);

    /// Sets packed 8-bit integers with the supplied values in reverse order.
    #[inline]
    #[allow(clippy::too_many_arguments)]
    pub unsafe fn _mm_setr_epi8(e0: i8, e1: i8, e2: i8, e3: i8, e4: i8, e5: i8, e6: i8, e7: i8, e8: i8, e9: i8, e10: i8, e11: i8, e12: i8, e13: i8, e14: i8, e15: i8) -> __m128i {
        __m128i::from_bytes([e0 as u8, e1 as u8, e2 as u8, e3 as u8, e4 as u8, e5 as u8, e6 as u8, e7 as u8, e8 as u8, e9 as u8, e10 as u8, e11 as u8, e12 as u8, e13 as u8, e14 as u8, e15 as u8])
    }

    /// Sets packed 8-bit integers with the supplied values (highest first).
    #[inline]
    #[allow(clippy::too_many_arguments)]
    pub unsafe fn _mm_set_epi8(e15: i8, e14: i8, e13: i8, e12: i8, e11: i8, e10: i8, e9: i8, e8: i8, e7: i8, e6: i8, e5: i8, e4: i8, e3: i8, e2: i8, e1: i8, e0: i8) -> __m128i {
        __m128i::from_bytes([e0 as u8, e1 as u8, e2 as u8, e3 as u8, e4 as u8, e5 as u8, e6 as u8, e7 as u8, e8 as u8, e9 as u8, e10 as u8, e11 as u8, e12 as u8, e13 as u8, e14 as u8, e15 as u8])
    }

    /// Sets packed 8-bit integers with the supplied values in reverse order.
    #[inline]
    #[allow(clippy::too_many_arguments)]
    pub unsafe fn _mm256_setr_epi8(e0: i8, e1: i8, e2: i8, e3: i8, e4: i8, e5: i8, e6: i8, e7: i8, e8: i8, e9: i8, e10: i8, e11: i8, e12: i8, e13: i8, e14: i8, e15: i8, e16: i8, e17: i8, e18: i8, e19: i8, e20: i8, e21: i8, e22: i8, e23: i8, e24: i8, e25: i8, e26: i8, e27: i8, e28: i8, e29: i8, e30: i8, e31: i8) -> __m256i {
        __m256i::from_bytes([e0 as u8, e1 as u8, e2 as u8, e3 as u8, e4 as u8, e5 as u8, e6 as u8, e7 as u8, e8 as u8, e9 as u8, e10 as u8, e11 as u8, e12 as u8, e13 as u8, e14 as u8, e15 as u8, e16 as u8, e17 as u8, e18 as u8, e19 as u8, e20 as u8, e21 as u8, e22 as u8, e23 as u8, e24 as u8, e25 as u8, e26 as u8, e27 as u8, e28 as u8, e29 as u8, e30 as u8, e31 as u8])
    }

    /// Sets packed 8-bit integers with the supplied values (highest first).
    #[inline]
    #[allow(clippy::too_many_arguments)]
    pub unsafe fn _mm256_set_epi8(e31: i8, e30: i8, e29: i8, e28: i8, e27: i8, e26: i8, e25: i8, e24: i8, e23: i8, e22: i8, e21: i8, e20: i8, e19: i8, e18: i8, e17: i8, e16: i8, e15: i8, e14: i8, e13: i8, e12: i8, e11: i8, e10: i8, e9: i8, e8: i8, e7: i8, e6: i8, e5: i8, e4: i8, e3: i8, e2: i8, e1: i8, e0: i8) -> __m256i {
        __m256i::from_bytes([e0 as u8, e1 as u8, e2 as u8, e3 as u8, e4 as u8, e5 as u8, e6 as u8, e7 as u8, e8 as u8, e9 as u8, e10 as u8, e11 as u8, e12 as u8, e13 as u8, e14 as u8, e15 as u8, e16 as u8, e17 as u8, e18 as u8, e19 as u8, e20 as u8, e21 as u8, e22 as u8, e23 as u8, e24 as u8, e25 as u8, e26 as u8, e27 as u8, e28 as u8, e29 as u8, e30 as u8, e31 as u8])
    }

    /// Sets packed 32-bit integers with the supplied values in reverse order.
    #[inline]
    pub unsafe fn _mm_setr_epi32(e0: i32, e1: i32, e2: i32, e3: i32) -> __m128i {
        let mut r = [0u8; 16];
        wr32(&mut r, 0, e0);
        wr32(&mut r, 1, e1);
        wr32(&mut r, 2, e2);
        wr32(&mut r, 3, e3);
        __m128i::from_bytes(r)
    }

    /// Sets packed 32-bit integers with the supplied values (highest first).
    #[inline]
    pub unsafe fn _mm_set_epi32(e3: i32, e2: i32, e1: i32, e0: i32) -> __m128i {
        _mm_setr_epi32(e0, e1, e2, e3)
    }

    /// Sets packed 64-bit integers with the supplied values (highest first).
    #[inline]
    pub unsafe fn _mm_set_epi64x(e1: i64, e0: i64) -> __m128i {
        let mut r = [0u8; 16];
        wr64(&mut r, 0, e0 as u64);
        wr64(&mut r, 1, e1 as u64);
        __m128i::from_bytes(r)
    }

    /// Extracts a 32-bit integer at index `imm8`.
    #[inline]
    pub unsafe fn _mm_extract_epi32(a: __m128i, imm8: i32) -> i32 {
        rd32(&a.bytes, (imm8 & 3) as usize)
    }

    /// Extracts a 16-bit integer (zero-extended) at index `imm8`.
    #[inline]
    pub unsafe fn _mm_extract_epi16(a: __m128i, imm8: i32) -> i32 {
        rd16(&a.bytes, (imm8 & 7) as usize) as u16 as i32
    }

    /// Shuffles 32-bit integers in `a` using the control in `imm8`.
    #[inline]
    pub unsafe fn _mm_shuffle_epi32(a: __m128i, imm8: i32) -> __m128i {
        let mut r = [0u8; 16];
        let mut i = 0;
        while i < 4 {
            wr32(&mut r, i, rd32(&a.bytes, ((imm8 >> (2 * i)) & 3) as usize));
            i += 1;
        }
        __m128i::from_bytes(r)
    }

    /// Shifts `a` left by `imm8` bytes while shifting in zeros.
    #[inline]
    pub unsafe fn _mm_slli_si128(a: __m128i, imm8: i32) -> __m128i {
        let n = (imm8 & 0xFF) as usize;
        let mut r = [0u8; 16];
        let mut i = n;
        while i < 16 {
            r[i] = a.bytes[i - n];
            i += 1;
        }
        __m128i::from_bytes(r)
    }

    /// Shifts `a` right by `imm8` bytes while shifting in zeros.
    #[inline]
    pub unsafe fn _mm_srli_si128(a: __m128i, imm8: i32) -> __m128i {
        let n = (imm8 & 0xFF) as usize;
        let mut r = [0u8; 16];
        let mut i = 0;
        while i + n < 16 {
            r[i] = a.bytes[i + n];
            i += 1;
        }
        __m128i::from_bytes(r)
    }

    /// Unpacks and interleaves 64-bit integers from the high half of `a` and `b`.
    #[inline]
    pub unsafe fn _mm_unpackhi_epi64(a: __m128i, b: __m128i) -> __m128i {
        let mut r = [0u8; 16];
        wr64(&mut r, 0, rd64(&a.bytes, 1));
        wr64(&mut r, 1, rd64(&b.bytes, 1));
        __m128i::from_bytes(r)
    }

    /// Unpacks and interleaves 64-bit integers from the low half of `a` and `b`.
    #[inline]
    pub unsafe fn _mm_unpacklo_epi64(a: __m128i, b: __m128i) -> __m128i {
        let mut r = [0u8; 16];
        wr64(&mut r, 0, rd64(&a.bytes, 0));
        wr64(&mut r, 1, rd64(&b.bytes, 0));
        __m128i::from_bytes(r)
    }

    /// Performs a carry-less multiplication of two 64-bit integers, selected from `a`
    /// and `b` according to `imm8`.
    #[inline]
    pub unsafe fn _mm_clmulepi64_si128(a: __m128i, b: __m128i, imm8: i32) -> __m128i {
        let x = rd64(&a.bytes, (imm8 & 1) as usize);
        let y = rd64(&b.bytes, ((imm8 >> 4) & 1) as usize);
        let mut lo = 0u64;
        let mut hi = 0u64;
        let mut i = 0;
        while i < 64 {
            if (y >> i) & 1 != 0 {
                lo ^= x << i;
                if i > 0 {
                    hi ^= x >> (64 - i);
                }
            }
            i += 1;
        }
        let mut r = [0u8; 16];
        wr64(&mut r, 0, lo);
        wr64(&mut r, 1, hi);
        __m128i::from_bytes(r)
    }

    /// Permutes packed 32-bit integers in `a` using the corresponding index in `idx`.
    #[inline]
    pub unsafe fn _mm256_permutevar8x32_epi32(a: __m256i, idx: __m256i) -> __m256i {
        let mut r = [0u8; 32];
        let mut i = 0;
        while i < 8 {
            wr32(&mut r, i, rd32(&a.bytes, (rd32(&idx.bytes, i) & 7) as usize));
            i += 1;
        }
        __m256i::from_bytes(r)
    }

    /// Zero-extends a 128-bit vector to 256 bits.
    #[inline]
    pub unsafe fn _mm256_zextsi128_si256(a: __m128i) -> __m256i {
        _mm256_castsi128_si256(a)
    }

    /// Returns a vector whose lowest element is `a` and all higher elements are zero.
    #[inline]
    pub unsafe fn _mm_cvtsi64_si128(a: i64) -> __m128i {
        let mut b = [0u8; 16];
        b[..8].copy_from_slice(&a.to_le_bytes());
        __m128i::from_bytes(b)
    }

    /// Returns the lowest element of `a`.
    #[inline]
    pub unsafe fn _mm_cvtsi128_si64(a: __m128i) -> i64 {
        let mut x = [0u8; 8];
        x.copy_from_slice(&a.bytes[..8]);
        i64::from_le_bytes(x)
    }

    /// Returns the lowest 32-bit element of `a`.
    #[inline]
    pub unsafe fn _mm_cvtsi128_si32(a: __m128i) -> i32 {
        i32::from_le_bytes([a.bytes[0], a.bytes[1], a.bytes[2], a.bytes[3]])
    }

    /// Returns a vector whose lowest 32-bit element is `a` and all higher elements are zero.
    #[inline]
    pub unsafe fn _mm_cvtsi32_si128(a: i32) -> __m128i {
        let mut b = [0u8; 16];
        b[..4].copy_from_slice(&a.to_le_bytes());
        __m128i::from_bytes(b)
    }

    /// Computes the bitwise AND of `a` and `b`, and returns 1 if the result is zero.
    #[inline]
    pub unsafe fn _mm_testz_si128(a: __m128i, b: __m128i) -> i32 {
        let mut i = 0;
        while i < 16 {
            if a.bytes[i] & b.bytes[i] != 0 {
                return 0;
            }
            i += 1;
        }
        1
    }

    /// Broadcasts 128 bits of integer data from `a` to all 128-bit lanes.
    #[inline]
    pub unsafe fn _mm256_broadcastsi128_si256(a: __m128i) -> __m256i {
        let mut b = [0u8; 32];
        b[..16].copy_from_slice(&a.bytes);
        b[16..].copy_from_slice(&a.bytes);
        __m256i::from_bytes(b)
    }

    /// Casts a 128-bit vector to a 256-bit vector (upper half zeroed here).
    #[inline]
    pub unsafe fn _mm256_castsi128_si256(a: __m128i) -> __m256i {
        let mut b = [0u8; 32];
        b[..16].copy_from_slice(&a.bytes);
        __m256i::from_bytes(b)
    }

    /// Casts a 256-bit vector to its lower 128 bits.
    #[inline]
    pub unsafe fn _mm256_castsi256_si128(a: __m256i) -> __m128i {
        let mut b = [0u8; 16];
        b.copy_from_slice(&a.bytes[..16]);
        __m128i::from_bytes(b)
    }

    /// Extracts 128 bits (selected by `imm8`) from `a`.
    #[inline]
    pub unsafe fn _mm256_extracti128_si256(a: __m256i, imm8: i32) -> __m128i {
        let o = if imm8 & 1 != 0 { 16 } else { 0 };
        let mut b = [0u8; 16];
        b.copy_from_slice(&a.bytes[o..o + 16]);
        __m128i::from_bytes(b)
    }

    /// Shuffles 128-bit lanes selected by `imm8` from `a` and `b`.
    #[inline]
    pub unsafe fn _mm256_permute2x128_si256(a: __m256i, b: __m256i, imm8: i32) -> __m256i {
        let mut r = [0u8; 32];
        let mut half = 0;
        while half < 2 {
            let ctl = (imm8 >> (4 * half)) & 0xF;
            if ctl & 0x8 == 0 {
                let src = if ctl & 0x2 == 0 { &a.bytes } else { &b.bytes };
                let o = if ctl & 0x1 == 0 { 0 } else { 16 };
                r[half * 16..half * 16 + 16].copy_from_slice(&src[o..o + 16]);
            }
            half += 1;
        }
        __m256i::from_bytes(r)
    }

    /// Permutes 64-bit integers from `a` using control mask `imm8`.
    #[inline]
    pub unsafe fn _mm256_permute4x64_epi64(a: __m256i, imm8: i32) -> __m256i {
        let mut r = [0u8; 32];
        let mut i = 0;
        while i < 4 {
            let s = ((imm8 >> (2 * i)) & 3) as usize;
            r[i * 8..i * 8 + 8].copy_from_slice(&a.bytes[s * 8..s * 8 + 8]);
            i += 1;
        }
        __m256i::from_bytes(r)
    }

    /// Returns `1` if all bits of `a & b` are zero.
    #[inline]
    pub unsafe fn _mm256_testz_si256(a: __m256i, b: __m256i) -> i32 {
        let mut i = 0;
        while i < 32 {
            if a.bytes[i] & b.bytes[i] != 0 {
                return 0;
            }
            i += 1;
        }
        1
    }

    /// Performs one round of an AES encryption flow.
    pub unsafe fn _mm_aesenc_si128(a: __m128i, round_key: __m128i) -> __m128i {
        let _ = (a, round_key);
        crate::rt::panic_str("AES-NI intrinsics are not supported by rustgo", file!(), line!(), column!())
    }

    /// Performs one round of an AES decryption flow.
    pub unsafe fn _mm_aesdec_si128(a: __m128i, round_key: __m128i) -> __m128i {
        let _ = (a, round_key);
        crate::rt::panic_str("AES-NI intrinsics are not supported by rustgo", file!(), line!(), column!())
    }

    /// Fetch the line of data from memory that contains `p` (no-op).
    #[inline]
    pub unsafe fn _mm_prefetch(p: *const i8, strategy: i32) {
        let _ = (p, strategy);
    }

    /// Blends packed 16-bit integers from `a` and `b` using control mask `imm8`.
    #[inline]
    pub unsafe fn _mm_blend_epi16(a: __m128i, b: __m128i, imm8: i32) -> __m128i {
        let mut r = a.bytes;
        let mut i = 0;
        while i < 8 {
            if (imm8 >> i) & 1 != 0 {
                r[2 * i] = b.bytes[2 * i];
                r[2 * i + 1] = b.bytes[2 * i + 1];
            }
            i += 1;
        }
        __m128i::from_bytes(r)
    }

    macro_rules! shift64_ops {
        ($v:ident, $n:expr, slli = $slli:ident, srli = $srli:ident) => {
            /// Shifts packed 64-bit integers left by `imm8` while shifting in zeros.
            #[inline]
            pub unsafe fn $slli(a: $v, imm8: i32) -> $v {
                let mut r = [0u8; $n];
                let sh = imm8 as u32;
                if sh < 64 {
                    let mut i = 0;
                    while i < $n / 8 {
                        wr64(&mut r, i, rd64(&a.bytes, i) << sh);
                        i += 1;
                    }
                }
                $v::from_bytes(r)
            }

            /// Shifts packed 64-bit integers right by `imm8` while shifting in zeros.
            #[inline]
            pub unsafe fn $srli(a: $v, imm8: i32) -> $v {
                let mut r = [0u8; $n];
                let sh = imm8 as u32;
                if sh < 64 {
                    let mut i = 0;
                    while i < $n / 8 {
                        wr64(&mut r, i, rd64(&a.bytes, i) >> sh);
                        i += 1;
                    }
                }
                $v::from_bytes(r)
            }
        };
    }
    shift64_ops!(__m128i, 16, slli = _mm_slli_epi64, srli = _mm_srli_epi64);
    shift64_ops!(__m256i, 32, slli = _mm256_slli_epi64, srli = _mm256_srli_epi64);

    /// Copies `a`, then inserts 128 bits from `b` at the half selected by `imm8`.
    #[inline]
    pub unsafe fn _mm256_insertf128_si256(a: __m256i, b: __m128i, imm8: i32) -> __m256i {
        let mut r = a.bytes;
        let o = if imm8 & 1 != 0 { 16 } else { 0 };
        let mut i = 0;
        while i < 16 {
            r[o + i] = b.bytes[i];
            i += 1;
        }
        __m256i::from_bytes(r)
    }

    /// Copies `a`, then inserts 128 bits from `b` at the half selected by `imm8`.
    #[inline]
    pub unsafe fn _mm256_inserti128_si256(a: __m256i, b: __m128i, imm8: i32) -> __m256i {
        _mm256_insertf128_si256(a, b, imm8)
    }

    /// Sets packed `__m256i` from two 128-bit halves (`hi`, `lo`).
    #[inline]
    pub unsafe fn _mm256_set_m128i(hi: __m128i, lo: __m128i) -> __m256i {
        let mut r = [0u8; 32];
        let mut i = 0;
        while i < 16 {
            r[i] = lo.bytes[i];
            r[16 + i] = hi.bytes[i];
            i += 1;
        }
        __m256i::from_bytes(r)
    }

    /// Sets packed 64-bit integers, highest element first.
    #[inline]
    pub unsafe fn _mm256_set_epi64x(e3: i64, e2: i64, e1: i64, e0: i64) -> __m256i {
        let mut r = [0u8; 32];
        wr64(&mut r, 0, e0 as u64);
        wr64(&mut r, 1, e1 as u64);
        wr64(&mut r, 2, e2 as u64);
        wr64(&mut r, 3, e3 as u64);
        __m256i::from_bytes(r)
    }

    #[inline]
    fn sha_lanes(v: __m128i) -> [u32; 4] {
        [rd32(&v.bytes, 0) as u32, rd32(&v.bytes, 1) as u32, rd32(&v.bytes, 2) as u32, rd32(&v.bytes, 3) as u32]
    }

    #[inline]
    fn sha_from_lanes(l: [u32; 4]) -> __m128i {
        let mut r = [0u8; 16];
        let mut i = 0;
        while i < 4 {
            wr32(&mut r, i, l[i] as i32);
            i += 1;
        }
        __m128i::from_bytes(r)
    }

    /// Intermediate calculation for the next four SHA256 message dwords.
    #[inline]
    pub unsafe fn _mm_sha256msg1_epu32(a: __m128i, b: __m128i) -> __m128i {
        fn s0(x: u32) -> u32 {
            x.rotate_right(7) ^ x.rotate_right(18) ^ (x >> 3)
        }
        let w = sha_lanes(a);
        let w4 = sha_lanes(b)[0];
        sha_from_lanes([
            w[0].wrapping_add(s0(w[1])),
            w[1].wrapping_add(s0(w[2])),
            w[2].wrapping_add(s0(w[3])),
            w[3].wrapping_add(s0(w4)),
        ])
    }

    /// Final calculation for the next four SHA256 message dwords.
    #[inline]
    pub unsafe fn _mm_sha256msg2_epu32(a: __m128i, b: __m128i) -> __m128i {
        fn s1(x: u32) -> u32 {
            x.rotate_right(17) ^ x.rotate_right(19) ^ (x >> 10)
        }
        let x = sha_lanes(a);
        let y = sha_lanes(b);
        let w16 = x[0].wrapping_add(s1(y[2]));
        let w17 = x[1].wrapping_add(s1(y[3]));
        let w18 = x[2].wrapping_add(s1(w16));
        let w19 = x[3].wrapping_add(s1(w17));
        sha_from_lanes([w16, w17, w18, w19])
    }

    /// Performs two rounds of SHA256 operation.
    #[inline]
    pub unsafe fn _mm_sha256rnds2_epu32(a: __m128i, b: __m128i, k: __m128i) -> __m128i {
        let x = sha_lanes(a);
        let y = sha_lanes(b);
        let wk = sha_lanes(k);
        let (mut ra, mut rb, mut rc, mut rd) = (y[3], y[2], x[3], x[2]);
        let (mut re, mut rf, mut rg, mut rh) = (y[1], y[0], x[1], x[0]);
        let mut i = 0;
        while i < 2 {
            let ch = (re & rf) ^ (!re & rg);
            let maj = (ra & rb) ^ (ra & rc) ^ (rb & rc);
            let sum0 = ra.rotate_right(2) ^ ra.rotate_right(13) ^ ra.rotate_right(22);
            let sum1 = re.rotate_right(6) ^ re.rotate_right(11) ^ re.rotate_right(25);
            let t = ch.wrapping_add(sum1).wrapping_add(wk[i]).wrapping_add(rh);
            let na = t.wrapping_add(maj).wrapping_add(sum0);
            let ne = t.wrapping_add(rd);
            rh = rg;
            rg = rf;
            rf = re;
            re = ne;
            rd = rc;
            rc = rb;
            rb = ra;
            ra = na;
            i += 1;
        }
        sha_from_lanes([rf, re, rb, ra])
    }

    /// `XFEATURE_ENABLED_MASK` for `XCR`.
    pub const _XCR_XFEATURE_ENABLED_MASK: u32 = 0;

    /// Reads the contents of the extended control register `XCR` specified in `xcr_no`.
    ///
    /// NOTE: consistent with the masked `cpuid` (baseline x86_64, no AVX state),
    /// `XCR0` reports only the x87 and SSE state components.
    #[inline]
    pub unsafe fn _xgetbv(xcr_no: u32) -> u64 {
        if xcr_no == 0 { 0b11 } else { 0 }
    }

    /// Result of the `cpuid` instruction.
    #[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
    pub struct CpuidResult {
        /// EAX register.
        pub eax: u32,
        /// EBX register.
        pub ebx: u32,
        /// ECX register.
        pub ecx: u32,
        /// EDX register.
        pub edx: u32,
    }

    extern "C" {
        fn rustgo_cpuid(leaf: u32, sub: u32, out: *mut u32);
    }

    /// Returns the result of the `cpuid` instruction for a given `leaf` (`EAX`)
    /// and `sub_leaf` (`ECX`).
    ///
    /// NOTE: rustgo emulates SIMD intrinsics lane by lane and reports only the
    /// baseline x86_64 feature set (see `is_x86_feature_detected!`). The raw
    /// `cpuid` result is masked to match, so crates that detect features via
    /// `cpuid` (cpufeatures, raw-cpuid, ...) select their portable code paths:
    /// leaf 1 ECX loses SSE3..AVX/AES/RDRAND (and XSAVE/OSXSAVE), leaf 7
    /// reports no extended features, and leaf 0x8000_0001 ECX loses
    /// LZCNT/SSE4A/XOP/FMA4/TBM. Vendor, brand, cache and topology leaves are
    /// passed through unchanged.
    #[inline]
    pub fn __cpuid_count(leaf: u32, sub_leaf: u32) -> CpuidResult {
        let mut out = [0u32; 4];
        unsafe { rustgo_cpuid(leaf, sub_leaf, out.as_mut_ptr()) };
        let mut r = CpuidResult { eax: out[0], ebx: out[1], ecx: out[2], edx: out[3] };
        match leaf {
            1 => {
                // sse3, pclmulqdq, ssse3, fma, cx16, sse4.1, sse4.2, movbe,
                // popcnt, aes, xsave, osxsave, avx, f16c, rdrand
                const CLEAR: u32 = (1 << 0) | (1 << 1) | (1 << 9) | (1 << 12) | (1 << 13) | (1 << 19) | (1 << 20)
                    | (1 << 22) | (1 << 23) | (1 << 25) | (1 << 26) | (1 << 27) | (1 << 28) | (1 << 29) | (1 << 30);
                r.ecx &= !CLEAR;
            }
            7 => {
                r.ebx = 0;
                r.ecx = 0;
                r.edx = 0;
                if sub_leaf != 0 {
                    r.eax = 0;
                }
            }
            0x8000_0001 => {
                const CLEAR: u32 = (1 << 5) | (1 << 6) | (1 << 11) | (1 << 16) | (1 << 21);
                r.ecx &= !CLEAR;
            }
            _ => {}
        }
        r
    }

    /// See [`__cpuid_count`].
    #[inline]
    pub fn __cpuid(leaf: u32) -> CpuidResult {
        __cpuid_count(leaf, 0)
    }

    /// Returns the highest-supported `leaf` (`EAX`) and sub-leaf (`ECX`) `cpuid` values.
    #[inline]
    pub fn __get_cpuid_max(leaf: u32) -> (u32, u32) {
        let CpuidResult { eax, ebx, .. } = __cpuid(leaf);
        (eax, ebx)
    }

    /// Hint to the processor that the code sequence is a spin-wait loop.
    #[inline]
    pub unsafe fn _mm_pause() {}

    /// Constant for `_mm_prefetch`.
    pub const _MM_HINT_T0: i32 = 3;
    /// Constant for `_mm_prefetch`.
    pub const _MM_HINT_T1: i32 = 2;
    /// Constant for `_mm_prefetch`.
    pub const _MM_HINT_T2: i32 = 1;
    /// Constant for `_mm_prefetch`.
    pub const _MM_HINT_NTA: i32 = 0;
}

#[doc(hidden)]
pub fn __is_x86_feature_detected(feature: &str) -> bool {
    // Baseline x86_64 features; the emulated intrinsics give correct results
    // for these. Everything else reports as unavailable.
    matches!(feature, "sse" | "sse2" | "fxsr")
}

/// A macro to test at runtime whether a CPU feature is available on x86/x86-64 platforms.
#[macro_export]
macro_rules! is_x86_feature_detected {
    ($t:tt) => {
        $crate::arch::__is_x86_feature_detected($t)
    };
}
