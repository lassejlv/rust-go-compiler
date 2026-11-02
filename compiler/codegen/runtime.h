/* rustgo runtime (generated code relies on GNU C extensions) */
#include <stdint.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>
#include <strings.h>

typedef struct {} Unit;
typedef struct { void* ptr; uint64_t meta; } Fat;

#define RG_I128_MAX ((__int128)(((unsigned __int128)1 << 127) - 1))
#define RG_I128_MIN (-RG_I128_MAX - 1)
#define RG_U128_MAX (~(unsigned __int128)0)

static char rg_outbuf[1 << 16];

static void rg_init(int argc, char** argv) {
  (void)argc; (void)argv;
  setvbuf(stdout, rg_outbuf, _IOFBF, sizeof rg_outbuf);
}

static void rg_flush_stdout(void) { fflush(stdout); }

static void rg_write_stdout(const uint8_t* p, uint64_t n) { if (n) fwrite(p, 1, n, stdout); }
static void rg_write_stderr(const uint8_t* p, uint64_t n) { fflush(stdout); if (n) fwrite(p, 1, n, stderr); }

__attribute__((noreturn)) static void rg_exit(int32_t code) { fflush(stdout); exit(code); }
__attribute__((noreturn)) static void rg_abort(void) { fflush(stdout); abort(); }

__attribute__((noreturn, cold)) static void rg_panic_raw(const char* msg, uint64_t mlen, const char* file, uint64_t flen, uint32_t line, uint32_t col) {
  fflush(stdout);
  fprintf(stderr, "\nthread 'main' panicked at %.*s:%u:%u:\n%.*s\nnote: run with `RUST_BACKTRACE=1` environment variable to display a backtrace\n",
          (int)flen, file, line, col, (int)mlen, msg);
  fflush(stderr);
  exit(101);
}

__attribute__((noreturn, cold)) static void rg_panic_impl(Fat msg, Fat file, uint32_t line, uint32_t col) {
  rg_panic_raw((const char*)msg.ptr, msg.meta, (const char*)file.ptr, file.meta, line, col);
}

__attribute__((noreturn, cold)) static void rg_panic_str(const char* msg, const char* file, uint32_t line, uint32_t col) {
  rg_panic_raw(msg, strlen(msg), file, strlen(file), line, col);
}

__attribute__((noreturn, cold)) static void rg_panic_bounds(uint64_t idx, uint64_t len, const char* file, uint32_t line, uint32_t col) {
  char buf[128];
  int n = snprintf(buf, sizeof buf, "index out of bounds: the len is %llu but the index is %llu", (unsigned long long)len, (unsigned long long)idx);
  rg_panic_raw(buf, n, file, strlen(file), line, col);
}

#define rg_unreachable() do { fflush(stdout); fprintf(stderr, "internal error: entered unreachable code (rustgo)\n"); abort(); } while (0)

static void rg_drop_noop(void* p) { (void)p; }

/* ---- checked arithmetic (debug-mode semantics) ---- */
#define RG_ADD(T, a, b, F, L, C) ({ T _r; if (__builtin_add_overflow((T)(a), (T)(b), &_r)) rg_panic_str("attempt to add with overflow", F, L, C); _r; })
#define RG_SUB(T, a, b, F, L, C) ({ T _r; if (__builtin_sub_overflow((T)(a), (T)(b), &_r)) rg_panic_str("attempt to subtract with overflow", F, L, C); _r; })
#define RG_MUL(T, a, b, F, L, C) ({ T _r; if (__builtin_mul_overflow((T)(a), (T)(b), &_r)) rg_panic_str("attempt to multiply with overflow", F, L, C); _r; })
#define RG_TMIN(T) ((T)((unsigned __int128)1 << (sizeof(T) * 8 - 1)))
#define RG_DIV(T, a, b, F, L, C) ({ T _a = (a), _b = (b); \
  if (_b == 0) rg_panic_str("attempt to divide by zero", F, L, C); \
  if ((T)-1 < 0 && _b == (T)-1 && _a == RG_TMIN(T)) rg_panic_str("attempt to divide with overflow", F, L, C); \
  (T)(_a / _b); })
#define RG_REM(T, a, b, F, L, C) ({ T _a = (a), _b = (b); \
  if (_b == 0) rg_panic_str("attempt to calculate the remainder with a divisor of zero", F, L, C); \
  if ((T)-1 < 0 && _b == (T)-1 && _a == RG_TMIN(T)) rg_panic_str("attempt to calculate the remainder with overflow", F, L, C); \
  (T)(_a % _b); })
#define RG_SHL(T, BITS, a, b, F, L, C) ({ T _a = (a); unsigned __int128 _s = (unsigned __int128)(b); \
  if ((b) < 0 || _s >= (BITS)) rg_panic_str("attempt to shift left with overflow", F, L, C); \
  (T)((unsigned __int128)_a << _s); })
#define RG_SHR(T, BITS, a, b, F, L, C) ({ T _a = (a); unsigned __int128 _s = (unsigned __int128)(b); \
  if ((b) < 0 || _s >= (BITS)) rg_panic_str("attempt to shift right with overflow", F, L, C); \
  (T)(_a >> _s); })
#define RG_NEG(T, a, F, L, C) ({ T _a = (a); if (_a == RG_TMIN(T)) rg_panic_str("attempt to negate with overflow", F, L, C); (T)(-_a); })
#define RG_F2I(T, v, MN, MX) ({ double _v = (double)(v); \
  (_v != _v) ? (T)0 : (_v <= (double)(MN)) ? (T)(MN) : (_v >= (double)(MX)) ? (T)(MX) : (T)_v; })

/* ---- memory ---- */
static uint8_t* rg_alloc(uint64_t size, uint64_t align) {
  void* p;
  if (size == 0) size = 1;
  if (align <= 16) p = malloc(size);
  else { size = (size + align - 1) / align * align; p = aligned_alloc(align, size); }
  if (!p) { fprintf(stderr, "memory allocation of %llu bytes failed\n", (unsigned long long)size); abort(); }
  return (uint8_t*)p;
}
static uint8_t* rg_realloc(uint8_t* p, uint64_t old, uint64_t align, uint64_t size) {
  if (align <= 16) {
    void* q = realloc(p, size ? size : 1);
    if (!q) { fprintf(stderr, "memory allocation failed\n"); abort(); }
    return (uint8_t*)q;
  }
  uint8_t* q = rg_alloc(size, align);
  memcpy(q, p, old < size ? old : size);
  free(p);
  return q;
}
static void rg_dealloc(uint8_t* p, uint64_t size, uint64_t align) { (void)size; (void)align; free(p); }
static int rg_memcmp(const uint8_t* a, const uint8_t* b, uint64_t n) { return n ? memcmp(a, b, n) : 0; }

static int rg_str_eq(Fat a, Fat b) { return a.meta == b.meta && (a.meta == 0 || memcmp(a.ptr, b.ptr, a.meta) == 0); }

/* ---- 128-bit helpers ---- */
static unsigned __int128 rg_u128_from_str(const char* s) {
  unsigned __int128 v = 0;
  for (; *s; s++) if (*s >= '0' && *s <= '9') v = v * 10 + (unsigned)(*s - '0');
  return v;
}
static uint32_t rg_clz128(unsigned __int128 x) {
  uint64_t hi = (uint64_t)(x >> 64), lo = (uint64_t)x;
  if (hi) return __builtin_clzll(hi);
  if (lo) return 64 + __builtin_clzll(lo);
  return 128;
}
static uint32_t rg_ctz128(unsigned __int128 x) {
  uint64_t hi = (uint64_t)(x >> 64), lo = (uint64_t)x;
  if (lo) return __builtin_ctzll(lo);
  if (hi) return 64 + __builtin_ctzll(hi);
  return 128;
}
static unsigned __int128 rg_bswap128(unsigned __int128 x) {
  return ((unsigned __int128)__builtin_bswap64((uint64_t)x) << 64) | __builtin_bswap64((uint64_t)(x >> 64));
}

/* ---- floats ---- */
static double rg_powi(double a, int32_t b) {
  int recip = b < 0; double r = 1;
  while (1) { if (b & 1) r *= a; b /= 2; if (b == 0) break; a *= a; }
  return recip ? 1 / r : r;
}
static float rg_powif(float a, int32_t b) {
  int recip = b < 0; float r = 1;
  while (1) { if (b & 1) r *= a; b /= 2; if (b == 0) break; a *= a; }
  return recip ? 1 / r : r;
}

/* shortest round-trip digits: returns number of digits, sets *exp10 (scientific exponent) */
static int rg_shortest(double a, int is_f32, char* digits, int* exp10) {
  char buf[64];
  if (a == 0) { digits[0] = '0'; digits[1] = 0; *exp10 = 0; return 1; }
  for (int p = 1; p <= 17; p++) {
    snprintf(buf, sizeof buf, "%.*e", p - 1, a);
    int ok;
    if (is_f32) ok = strtof(buf, NULL) == (float)a;
    else ok = strtod(buf, NULL) == a;
    if (ok) break;
  }
  int n = 0; char* q = buf;
  for (; *q && *q != 'e'; q++) if (*q >= '0' && *q <= '9') digits[n++] = *q;
  digits[n] = 0;
  *exp10 = atoi(q + 1);
  while (n > 1 && digits[n - 1] == '0') digits[--n] = 0;
  return n;
}

static uint64_t rg_fmt_f64(double v, uint8_t is_f32, uint8_t mode, int64_t prec, uint8_t* out) {
  char* o = (char*)out; int n = 0;
  if (v != v) { memcpy(o, "NaN", 3); return 3; }
  if (signbit(v)) { o[n++] = '-'; v = -v; }
  if (isinf(v)) { memcpy(o + n, "inf", 3); return n + 3; }
  if (mode == 2 || mode == 3) {
    char e = mode == 3 ? 'E' : 'e';
    char digits[40]; int ex, nd;
    if (prec >= 0) {
      char buf[1200];
      snprintf(buf, sizeof buf, "%.*e", (int)prec, v);
      char* q = strchr(buf, 'e');
      int mlen = (int)(q - buf);
      memcpy(o + n, buf, mlen); n += mlen;
      n += sprintf(o + n, "%c%d", e, atoi(q + 1));
      return n;
    }
    nd = rg_shortest(v, is_f32, digits, &ex);
    o[n++] = digits[0];
    if (nd > 1) { o[n++] = '.'; memcpy(o + n, digits + 1, nd - 1); n += nd - 1; }
    n += sprintf(o + n, "%c%d", e, ex);
    return n;
  }
  if (prec >= 0) {
    n += snprintf(o + n, 1190, "%.*f", (int)prec, v);
    return n;
  }
  char digits[40]; int ex;
  int nd = rg_shortest(v, is_f32, digits, &ex);
  if (mode == 1 && v != 0 && (ex < -4 || ex >= 16)) {
    o[n++] = digits[0];
    if (nd > 1) { o[n++] = '.'; memcpy(o + n, digits + 1, nd - 1); n += nd - 1; }
    n += sprintf(o + n, "e%d", ex);
    return n;
  }
  if (ex >= 0) {
    for (int i = 0; i <= ex; i++) o[n++] = i < nd ? digits[i] : '0';
    if (nd > ex + 1) {
      o[n++] = '.';
      for (int i = ex + 1; i < nd; i++) o[n++] = digits[i];
    } else if (mode == 1) {
      o[n++] = '.'; o[n++] = '0';
    }
  } else {
    o[n++] = '0'; o[n++] = '.';
    for (int i = 0; i < -ex - 1; i++) o[n++] = '0';
    for (int i = 0; i < nd; i++) o[n++] = digits[i];
  }
  return n;
}

static uint8_t rg_parse_f64(const uint8_t* p, uint64_t len, double* out) {
  char buf[512];
  if (len == 0 || len >= sizeof buf) return 0;
  memcpy(buf, p, len); buf[len] = 0;
  const char* s = buf;
  int i = 0;
  if (s[i] == '+' || s[i] == '-') i++;
  const char* rest = s + i;
  if (!strcasecmp(rest, "inf") || !strcasecmp(rest, "infinity") || !strcasecmp(rest, "nan")) {
    *out = strtod(s, NULL); return 1;
  }
  int digits = 0;
  while (s[i] >= '0' && s[i] <= '9') { i++; digits++; }
  if (s[i] == '.') { i++; while (s[i] >= '0' && s[i] <= '9') { i++; digits++; } }
  if (!digits) return 0;
  if (s[i] == 'e' || s[i] == 'E') {
    i++;
    if (s[i] == '+' || s[i] == '-') i++;
    int ed = 0;
    while (s[i] >= '0' && s[i] <= '9') { i++; ed++; }
    if (!ed) return 0;
  }
  if (s[i] != 0) return 0;
  *out = strtod(s, NULL);
  return 1;
}
