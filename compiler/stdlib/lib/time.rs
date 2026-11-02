//! Temporal quantification.

use crate::fmt;
use crate::hash::{Hash, Hasher};
use crate::intrinsics;
use crate::iter::Iterator;
use crate::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign};

const NANOS_PER_SEC: u32 = 1_000_000_000;
const NANOS_PER_MILLI: u32 = 1_000_000;
const NANOS_PER_MICRO: u32 = 1_000;
const MILLIS_PER_SEC: u64 = 1_000;
const MICROS_PER_SEC: u64 = 1_000_000;

/// A `Duration` type to represent a span of time.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Duration {
    secs: u64,
    nanos: u32,
}

impl Hash for Duration {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.secs.hash(state);
        self.nanos.hash(state);
    }
}

impl Duration {
    /// The duration of one second.
    pub const SECOND: Duration = Duration::from_secs(1);
    /// The duration of one millisecond.
    pub const MILLISECOND: Duration = Duration::from_millis(1);
    /// The duration of one microsecond.
    pub const MICROSECOND: Duration = Duration::from_micros(1);
    /// The duration of one nanosecond.
    pub const NANOSECOND: Duration = Duration::from_nanos(1);
    /// A duration of zero time.
    pub const ZERO: Duration = Duration::from_nanos(0);
    /// The maximum duration.
    pub const MAX: Duration = Duration { secs: u64::MAX, nanos: NANOS_PER_SEC - 1 };

    /// Creates a new `Duration` from the specified number of whole seconds and additional nanoseconds.
    #[inline]
    #[track_caller]
    pub const fn new(secs: u64, nanos: u32) -> Duration {
        if nanos < NANOS_PER_SEC {
            Duration { secs, nanos }
        } else {
            let extra = (nanos / NANOS_PER_SEC) as u64;
            let secs = match secs.checked_add(extra) {
                Some(secs) => secs,
                None => panic!("overflow in Duration::new"),
            };
            Duration { secs, nanos: nanos % NANOS_PER_SEC }
        }
    }

    /// Creates a new `Duration` from the specified number of whole seconds.
    #[inline]
    pub const fn from_secs(secs: u64) -> Duration {
        Duration { secs, nanos: 0 }
    }

    /// Creates a new `Duration` from the specified number of milliseconds.
    #[inline]
    pub const fn from_millis(millis: u64) -> Duration {
        Duration { secs: millis / MILLIS_PER_SEC, nanos: ((millis % MILLIS_PER_SEC) as u32) * NANOS_PER_MILLI }
    }

    /// Creates a new `Duration` from the specified number of microseconds.
    #[inline]
    pub const fn from_micros(micros: u64) -> Duration {
        Duration { secs: micros / MICROS_PER_SEC, nanos: ((micros % MICROS_PER_SEC) as u32) * NANOS_PER_MICRO }
    }

    /// Creates a new `Duration` from the specified number of nanoseconds.
    #[inline]
    pub const fn from_nanos(nanos: u64) -> Duration {
        Duration { secs: nanos / (NANOS_PER_SEC as u64), nanos: (nanos % (NANOS_PER_SEC as u64)) as u32 }
    }

    /// Creates a new `Duration` from the specified number of minutes.
    #[inline]
    #[track_caller]
    pub const fn from_mins(mins: u64) -> Duration {
        match mins.checked_mul(60) {
            Some(secs) => Duration::from_secs(secs),
            None => panic!("overflow in Duration::from_mins"),
        }
    }

    /// Creates a new `Duration` from the specified number of hours.
    #[inline]
    #[track_caller]
    pub const fn from_hours(hours: u64) -> Duration {
        match hours.checked_mul(3600) {
            Some(secs) => Duration::from_secs(secs),
            None => panic!("overflow in Duration::from_hours"),
        }
    }

    /// Returns true if this `Duration` spans no time.
    #[inline]
    pub const fn is_zero(&self) -> bool {
        self.secs == 0 && self.nanos == 0
    }

    /// Returns the number of _whole_ seconds contained by this `Duration`.
    #[inline]
    pub const fn as_secs(&self) -> u64 {
        self.secs
    }

    /// Returns the fractional part of this `Duration`, in whole milliseconds.
    #[inline]
    pub const fn subsec_millis(&self) -> u32 {
        self.nanos / NANOS_PER_MILLI
    }

    /// Returns the fractional part of this `Duration`, in whole microseconds.
    #[inline]
    pub const fn subsec_micros(&self) -> u32 {
        self.nanos / NANOS_PER_MICRO
    }

    /// Returns the fractional part of this `Duration`, in nanoseconds.
    #[inline]
    pub const fn subsec_nanos(&self) -> u32 {
        self.nanos
    }

    /// Returns the total number of whole milliseconds contained by this `Duration`.
    #[inline]
    pub const fn as_millis(&self) -> u128 {
        self.secs as u128 * MILLIS_PER_SEC as u128 + (self.nanos / NANOS_PER_MILLI) as u128
    }

    /// Returns the total number of whole microseconds contained by this `Duration`.
    #[inline]
    pub const fn as_micros(&self) -> u128 {
        self.secs as u128 * MICROS_PER_SEC as u128 + (self.nanos / NANOS_PER_MICRO) as u128
    }

    /// Returns the total number of nanoseconds contained by this `Duration`.
    #[inline]
    pub const fn as_nanos(&self) -> u128 {
        self.secs as u128 * NANOS_PER_SEC as u128 + self.nanos as u128
    }

    /// Computes the absolute difference between `self` and `other`.
    #[inline]
    pub fn abs_diff(self, other: Duration) -> Duration {
        if let Some(res) = self.checked_sub(other) { res } else { other.checked_sub(self).unwrap() }
    }

    /// Checked `Duration` addition.
    #[inline]
    pub const fn checked_add(self, rhs: Duration) -> Option<Duration> {
        if let Some(mut secs) = self.secs.checked_add(rhs.secs) {
            let mut nanos = self.nanos + rhs.nanos;
            if nanos >= NANOS_PER_SEC {
                nanos -= NANOS_PER_SEC;
                if let Some(new_secs) = secs.checked_add(1) {
                    secs = new_secs;
                } else {
                    return None;
                }
            }
            Some(Duration { secs, nanos })
        } else {
            None
        }
    }

    /// Saturating `Duration` addition.
    #[inline]
    pub const fn saturating_add(self, rhs: Duration) -> Duration {
        match self.checked_add(rhs) {
            Some(res) => res,
            None => Duration::MAX,
        }
    }

    /// Checked `Duration` subtraction.
    #[inline]
    pub const fn checked_sub(self, rhs: Duration) -> Option<Duration> {
        if let Some(mut secs) = self.secs.checked_sub(rhs.secs) {
            let nanos = if self.nanos >= rhs.nanos {
                self.nanos - rhs.nanos
            } else if let Some(sub_secs) = secs.checked_sub(1) {
                secs = sub_secs;
                self.nanos + NANOS_PER_SEC - rhs.nanos
            } else {
                return None;
            };
            Some(Duration { secs, nanos })
        } else {
            None
        }
    }

    /// Saturating `Duration` subtraction.
    #[inline]
    pub const fn saturating_sub(self, rhs: Duration) -> Duration {
        match self.checked_sub(rhs) {
            Some(res) => res,
            None => Duration::ZERO,
        }
    }

    /// Checked `Duration` multiplication.
    #[inline]
    pub const fn checked_mul(self, rhs: u32) -> Option<Duration> {
        let total_nanos = self.nanos as u64 * rhs as u64;
        let extra_secs = total_nanos / (NANOS_PER_SEC as u64);
        let nanos = (total_nanos % (NANOS_PER_SEC as u64)) as u32;
        if let Some(s) = self.secs.checked_mul(rhs as u64) {
            if let Some(secs) = s.checked_add(extra_secs) {
                return Some(Duration { secs, nanos });
            }
        }
        None
    }

    /// Saturating `Duration` multiplication.
    #[inline]
    pub const fn saturating_mul(self, rhs: u32) -> Duration {
        match self.checked_mul(rhs) {
            Some(res) => res,
            None => Duration::MAX,
        }
    }

    /// Checked `Duration` division.
    #[inline]
    pub const fn checked_div(self, rhs: u32) -> Option<Duration> {
        if rhs != 0 {
            let secs = self.secs / (rhs as u64);
            let carry = self.secs - secs * (rhs as u64);
            let extra_nanos = carry * (NANOS_PER_SEC as u64) / (rhs as u64);
            let nanos = self.nanos / rhs + (extra_nanos as u32);
            Some(Duration::new(secs, nanos))
        } else {
            None
        }
    }

    /// Returns the number of seconds contained by this `Duration` as `f64`.
    #[inline]
    pub fn as_secs_f64(&self) -> f64 {
        (self.secs as f64) + (self.nanos as f64) / (NANOS_PER_SEC as f64)
    }

    /// Returns the number of seconds contained by this `Duration` as `f32`.
    #[inline]
    pub fn as_secs_f32(&self) -> f32 {
        (self.secs as f32) + (self.nanos as f32) / (NANOS_PER_SEC as f32)
    }

    /// Returns the number of milliseconds contained by this `Duration` as `f64`.
    #[inline]
    pub fn as_millis_f64(&self) -> f64 {
        (self.secs as f64) * (MILLIS_PER_SEC as f64) + (self.nanos as f64) / (NANOS_PER_MILLI as f64)
    }

    /// Returns the number of milliseconds contained by this `Duration` as `f32`.
    #[inline]
    pub fn as_millis_f32(&self) -> f32 {
        self.as_millis_f64() as f32
    }

    /// Creates a new `Duration` from the specified number of seconds represented as `f64`.
    #[track_caller]
    pub fn from_secs_f64(secs: f64) -> Duration {
        match Duration::try_from_secs_f64(secs) {
            Ok(v) => v,
            Err(e) => panic!("{}", e.description()),
        }
    }

    /// Creates a new `Duration` from the specified number of seconds represented as `f32`.
    #[track_caller]
    pub fn from_secs_f32(secs: f32) -> Duration {
        match Duration::try_from_secs_f32(secs) {
            Ok(v) => v,
            Err(e) => panic!("{}", e.description()),
        }
    }

    /// The checked version of `from_secs_f64`.
    pub fn try_from_secs_f64(secs: f64) -> Result<Duration, TryFromFloatSecsError> {
        if secs < 0.0 {
            return Err(TryFromFloatSecsError { kind: TryFromFloatSecsErrorKind::Negative });
        }
        if !(secs < 18446744073709551616.0) {
            return Err(TryFromFloatSecsError { kind: TryFromFloatSecsErrorKind::OverflowOrNan });
        }
        let whole = secs.trunc();
        let mut s = whole as u64;
        let mut n = ((secs - whole) * (NANOS_PER_SEC as f64)).round() as u64;
        if n >= NANOS_PER_SEC as u64 {
            n -= NANOS_PER_SEC as u64;
            s = match s.checked_add(1) {
                Some(s) => s,
                None => return Err(TryFromFloatSecsError { kind: TryFromFloatSecsErrorKind::OverflowOrNan }),
            };
        }
        Ok(Duration { secs: s, nanos: n as u32 })
    }

    /// The checked version of `from_secs_f32`.
    pub fn try_from_secs_f32(secs: f32) -> Result<Duration, TryFromFloatSecsError> {
        if secs.is_nan() {
            return Err(TryFromFloatSecsError { kind: TryFromFloatSecsErrorKind::OverflowOrNan });
        }
        if secs < -0.0 {
            return Err(TryFromFloatSecsError { kind: TryFromFloatSecsErrorKind::Negative });
        }
        Duration::try_from_secs_f64(secs as f64)
    }

    /// Multiplies `Duration` by `f64`.
    #[track_caller]
    pub fn mul_f64(self, rhs: f64) -> Duration {
        Duration::from_secs_f64(rhs * self.as_secs_f64())
    }

    /// Multiplies `Duration` by `f32`.
    #[track_caller]
    pub fn mul_f32(self, rhs: f32) -> Duration {
        Duration::from_secs_f64((rhs as f64) * self.as_secs_f64())
    }

    /// Divides `Duration` by `f64`.
    #[track_caller]
    pub fn div_f64(self, rhs: f64) -> Duration {
        Duration::from_secs_f64(self.as_secs_f64() / rhs)
    }

    /// Divides `Duration` by `f32`.
    #[track_caller]
    pub fn div_f32(self, rhs: f32) -> Duration {
        Duration::from_secs_f64(self.as_secs_f64() / (rhs as f64))
    }

    /// Divides `Duration` by `Duration` and returns `f64`.
    #[inline]
    pub fn div_duration_f64(self, rhs: Duration) -> f64 {
        let self_nanos = (self.secs as f64) * (NANOS_PER_SEC as f64) + (self.nanos as f64);
        let rhs_nanos = (rhs.secs as f64) * (NANOS_PER_SEC as f64) + (rhs.nanos as f64);
        self_nanos / rhs_nanos
    }

    /// Divides `Duration` by `Duration` and returns `f32`.
    #[inline]
    pub fn div_duration_f32(self, rhs: Duration) -> f32 {
        self.div_duration_f64(rhs) as f32
    }

    pub(crate) fn as_nanos_u64_saturating(&self) -> u64 {
        let n = self.as_nanos();
        if n > (u64::MAX as u128) { u64::MAX } else { n as u64 }
    }
}

impl Add for Duration {
    type Output = Duration;
    #[inline]
    #[track_caller]
    fn add(self, rhs: Duration) -> Duration {
        match self.checked_add(rhs) {
            Some(d) => d,
            None => panic!("overflow when adding durations"),
        }
    }
}

impl AddAssign for Duration {
    #[inline]
    #[track_caller]
    fn add_assign(&mut self, rhs: Duration) {
        *self = *self + rhs;
    }
}

impl Sub for Duration {
    type Output = Duration;
    #[inline]
    #[track_caller]
    fn sub(self, rhs: Duration) -> Duration {
        match self.checked_sub(rhs) {
            Some(d) => d,
            None => panic!("overflow when subtracting durations"),
        }
    }
}

impl SubAssign for Duration {
    #[inline]
    #[track_caller]
    fn sub_assign(&mut self, rhs: Duration) {
        *self = *self - rhs;
    }
}

impl Mul<u32> for Duration {
    type Output = Duration;
    #[inline]
    #[track_caller]
    fn mul(self, rhs: u32) -> Duration {
        match self.checked_mul(rhs) {
            Some(d) => d,
            None => panic!("overflow when multiplying duration by scalar"),
        }
    }
}

impl Mul<Duration> for u32 {
    type Output = Duration;
    #[inline]
    #[track_caller]
    fn mul(self, rhs: Duration) -> Duration {
        rhs * self
    }
}

impl MulAssign<u32> for Duration {
    #[inline]
    #[track_caller]
    fn mul_assign(&mut self, rhs: u32) {
        *self = *self * rhs;
    }
}

impl Div<u32> for Duration {
    type Output = Duration;
    #[inline]
    #[track_caller]
    fn div(self, rhs: u32) -> Duration {
        match self.checked_div(rhs) {
            Some(d) => d,
            None => panic!("divide by zero error when dividing duration by scalar"),
        }
    }
}

impl DivAssign<u32> for Duration {
    #[inline]
    #[track_caller]
    fn div_assign(&mut self, rhs: u32) {
        *self = *self / rhs;
    }
}

impl crate::iter::Sum for Duration {
    fn sum<I: Iterator<Item = Duration>>(iter: I) -> Duration {
        let mut total = Duration::ZERO;
        for d in iter {
            total = match total.checked_add(d) {
                Some(t) => t,
                None => panic!("overflow in iter::sum over durations"),
            };
        }
        total
    }
}

impl<'a> crate::iter::Sum<&'a Duration> for Duration {
    fn sum<I: Iterator<Item = &'a Duration>>(iter: I) -> Duration {
        let mut total = Duration::ZERO;
        for d in iter {
            total = match total.checked_add(*d) {
                Some(t) => t,
                None => panic!("overflow in iter::sum over durations"),
            };
        }
        total
    }
}

// Formats a floating point number in decimal notation: `integer_part` and
// `fractional_part / (10 * fractional_divisor)`, followed by `postfix`.
// (A port of the formatting used by the real `Duration`'s `Debug`.)
fn fmt_decimal(
    f: &mut fmt::Formatter<'_>,
    integer_part: u64,
    fractional_part: u32,
    fractional_divisor: u32,
    prefix: &str,
    postfix: &str,
) -> fmt::Result {
    let mut integer_part = Some(integer_part);
    let mut fractional_part = fractional_part;
    let mut fractional_divisor = fractional_divisor;
    let mut buf = [b'0'; 9];
    let end = match f.precision() {
        Some(p) => if p < 9 { p } else { 9 },
        None => 9,
    };
    let mut pos = 0;
    while fractional_part > 0 && pos < end {
        buf[pos] = b'0' + (fractional_part / fractional_divisor) as u8;
        fractional_part %= fractional_divisor;
        fractional_divisor /= 10;
        pos += 1;
    }
    if fractional_part > 0 && fractional_part >= fractional_divisor * 5 {
        let mut rev_pos = pos;
        let mut carry = true;
        while carry && rev_pos > 0 {
            rev_pos -= 1;
            if buf[rev_pos] < b'9' {
                buf[rev_pos] += 1;
                carry = false;
            } else {
                buf[rev_pos] = b'0';
            }
        }
        if carry {
            integer_part = match integer_part {
                Some(i) => i.checked_add(1),
                None => None,
            };
        }
    }
    let end = match f.precision() {
        Some(p) => if p < 9 { p } else { 9 },
        None => pos,
    };
    let mut s = String::new();
    s.push_str(prefix);
    match integer_part {
        Some(i) => s.push_str(&i.to_string()),
        None => s.push_str("18446744073709551616"),
    }
    if end > 0 {
        s.push('.');
        let mut i = 0;
        while i < end {
            s.push(buf[i] as char);
            i += 1;
        }
        if let Some(p) = f.precision() {
            let mut w = end;
            while w < p {
                s.push('0');
                w += 1;
            }
        }
    }
    s.push_str(postfix);
    f.pad_ignoring_precision(&s)
}

impl fmt::Debug for Duration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let prefix = if f.sign_plus() { "+" } else { "" };
        if self.secs > 0 {
            fmt_decimal(f, self.secs, self.nanos, NANOS_PER_SEC / 10, prefix, "s")
        } else if self.nanos >= NANOS_PER_MILLI {
            fmt_decimal(f, (self.nanos / NANOS_PER_MILLI) as u64, self.nanos % NANOS_PER_MILLI, NANOS_PER_MILLI / 10, prefix, "ms")
        } else if self.nanos >= NANOS_PER_MICRO {
            fmt_decimal(f, (self.nanos / NANOS_PER_MICRO) as u64, self.nanos % NANOS_PER_MICRO, NANOS_PER_MICRO / 10, prefix, "µs")
        } else {
            fmt_decimal(f, self.nanos as u64, 0, 1, prefix, "ns")
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TryFromFloatSecsErrorKind {
    Negative,
    OverflowOrNan,
}

/// An error which can be returned when converting a floating-point value of seconds into a `Duration`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TryFromFloatSecsError {
    kind: TryFromFloatSecsErrorKind,
}

impl TryFromFloatSecsError {
    fn description(&self) -> &'static str {
        match self.kind {
            TryFromFloatSecsErrorKind::Negative => "cannot convert float seconds to Duration: value is negative",
            TryFromFloatSecsErrorKind::OverflowOrNan => "cannot convert float seconds to Duration: value is either too big or NaN",
        }
    }
}

impl fmt::Display for TryFromFloatSecsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.description())
    }
}

impl crate::error::Error for TryFromFloatSecsError {}

// ---------------------------------------------------------------------------
// Instant

/// A measurement of a monotonically nondecreasing clock.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Instant {
    t: Duration,
}

impl Instant {
    /// Returns an instant corresponding to "now".
    #[inline]
    pub fn now() -> Instant {
        Instant { t: Duration::from_nanos(unsafe { intrinsics::time_monotonic_ns() }) }
    }

    /// Returns the amount of time elapsed from another instant to this one, or zero duration if that instant is later than this one.
    #[inline]
    pub fn duration_since(&self, earlier: Instant) -> Duration {
        self.checked_duration_since(earlier).unwrap_or_default()
    }

    /// Returns the amount of time elapsed from another instant to this one, or None if that instant is later than this one.
    #[inline]
    pub fn checked_duration_since(&self, earlier: Instant) -> Option<Duration> {
        self.t.checked_sub(earlier.t)
    }

    /// Returns the amount of time elapsed from another instant to this one, or zero duration if that instant is later than this one.
    #[inline]
    pub fn saturating_duration_since(&self, earlier: Instant) -> Duration {
        self.checked_duration_since(earlier).unwrap_or_default()
    }

    /// Returns the amount of time elapsed since this instant.
    #[inline]
    pub fn elapsed(&self) -> Duration {
        Instant::now().duration_since(*self)
    }

    /// Returns `Some(t)` where `t` is the time `self + duration` if `t` can be represented.
    #[inline]
    pub fn checked_add(&self, duration: Duration) -> Option<Instant> {
        match self.t.checked_add(duration) {
            Some(t) => Some(Instant { t }),
            None => None,
        }
    }

    /// Returns `Some(t)` where `t` is the time `self - duration` if `t` can be represented.
    #[inline]
    pub fn checked_sub(&self, duration: Duration) -> Option<Instant> {
        match self.t.checked_sub(duration) {
            Some(t) => Some(Instant { t }),
            None => None,
        }
    }
}

impl Add<Duration> for Instant {
    type Output = Instant;
    #[track_caller]
    fn add(self, other: Duration) -> Instant {
        self.checked_add(other).expect("overflow when adding duration to instant")
    }
}

impl AddAssign<Duration> for Instant {
    #[track_caller]
    fn add_assign(&mut self, other: Duration) {
        *self = *self + other;
    }
}

impl Sub<Duration> for Instant {
    type Output = Instant;
    #[track_caller]
    fn sub(self, other: Duration) -> Instant {
        self.checked_sub(other).expect("overflow when subtracting duration from instant")
    }
}

impl SubAssign<Duration> for Instant {
    #[track_caller]
    fn sub_assign(&mut self, other: Duration) {
        *self = *self - other;
    }
}

impl Sub<Instant> for Instant {
    type Output = Duration;
    fn sub(self, other: Instant) -> Duration {
        self.duration_since(other)
    }
}

impl fmt::Debug for Instant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Instant").field("tv_sec", &self.t.secs).field("tv_nsec", &self.t.nanos).finish()
    }
}

// ---------------------------------------------------------------------------
// SystemTime

/// A measurement of the system clock.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SystemTime {
    // time since the epoch (negative times are not supported)
    t: Duration,
}

/// An anchor in time which can be used to create new `SystemTime` instances.
pub const UNIX_EPOCH: SystemTime = SystemTime { t: Duration::ZERO };

/// An error returned from the `duration_since` and `elapsed` methods on `SystemTime`.
#[derive(Clone, Debug)]
pub struct SystemTimeError(Duration);

impl SystemTimeError {
    /// Returns the positive duration which represents how far forward the second system time was from the first.
    pub fn duration(&self) -> Duration {
        self.0
    }
}

impl fmt::Display for SystemTimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("second time provided was later than self")
    }
}

impl crate::error::Error for SystemTimeError {}

impl SystemTime {
    /// An anchor in time which can be used to create new `SystemTime` instances.
    pub const UNIX_EPOCH: SystemTime = UNIX_EPOCH;

    /// Returns the system time corresponding to "now".
    pub fn now() -> SystemTime {
        SystemTime { t: Duration::from_nanos(unsafe { intrinsics::time_realtime_ns() }) }
    }

    /// Returns the amount of time elapsed from an earlier point in time.
    pub fn duration_since(&self, earlier: SystemTime) -> Result<Duration, SystemTimeError> {
        match self.t.checked_sub(earlier.t) {
            Some(d) => Ok(d),
            None => Err(SystemTimeError(earlier.t - self.t)),
        }
    }

    /// Returns the difference from this system time to the current system time.
    pub fn elapsed(&self) -> Result<Duration, SystemTimeError> {
        SystemTime::now().duration_since(*self)
    }

    /// Returns `Some(t)` where `t` is the time `self + duration` if `t` can be represented.
    pub fn checked_add(&self, duration: Duration) -> Option<SystemTime> {
        match self.t.checked_add(duration) {
            Some(t) => Some(SystemTime { t }),
            None => None,
        }
    }

    /// Returns `Some(t)` where `t` is the time `self - duration` if `t` can be represented.
    pub fn checked_sub(&self, duration: Duration) -> Option<SystemTime> {
        match self.t.checked_sub(duration) {
            Some(t) => Some(SystemTime { t }),
            None => None,
        }
    }
}

impl Add<Duration> for SystemTime {
    type Output = SystemTime;
    #[track_caller]
    fn add(self, dur: Duration) -> SystemTime {
        self.checked_add(dur).expect("overflow when adding duration to instant")
    }
}

impl AddAssign<Duration> for SystemTime {
    fn add_assign(&mut self, other: Duration) {
        *self = *self + other;
    }
}

impl Sub<Duration> for SystemTime {
    type Output = SystemTime;
    #[track_caller]
    fn sub(self, dur: Duration) -> SystemTime {
        self.checked_sub(dur).expect("overflow when subtracting duration from instant")
    }
}

impl SubAssign<Duration> for SystemTime {
    fn sub_assign(&mut self, other: Duration) {
        *self = *self - other;
    }
}

impl fmt::Debug for SystemTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SystemTime").field("tv_sec", &self.t.secs).field("tv_nsec", &self.t.nanos).finish()
    }
}
