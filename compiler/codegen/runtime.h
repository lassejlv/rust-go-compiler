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

#include <pthread.h>
#include <setjmp.h>
#include <unistd.h>
#include <time.h>
#include <errno.h>
#include <sched.h>
#include <sys/syscall.h>
#include <linux/futex.h>
#include <signal.h>
#include <stdio_ext.h>
#include <cpuid.h>

extern char** environ;

/* Process-wide runtime state is declared with RG_GLOBAL (initializers with
   RG_INIT): the program may be compiled as several translation units
   (RG_MULTI_TU), of which exactly one (RG_DEFINE_GLOBALS) defines each
   global while the others declare it extern. Runtime functions are static
   (a copy per unit) and must not keep state of their own: no `static`
   variables, at file or function scope (codegen.RuntimeSplittable checks). */
#if !defined(RG_MULTI_TU)
#define RG_GLOBAL static
#define RG_INIT(...) = __VA_ARGS__
#elif defined(RG_DEFINE_GLOBALS)
#define RG_GLOBAL
#define RG_INIT(...) = __VA_ARGS__
#else
#define RG_GLOBAL extern
#define RG_INIT(...)
#endif

RG_GLOBAL char rg_outbuf[1 << 16];
RG_GLOBAL int rg_argc;
RG_GLOBAL char** rg_argv;

/* ---- per-thread runtime state ---- */
/* cleanup stack for unwinding on panic (grows on demand) */
typedef struct { void* p; void (*f)(void*); uint8_t* flag; } RgCleanup;
RG_GLOBAL __thread RgCleanup* rg_cs;
RG_GLOBAL __thread size_t rg_csn, rg_cscap;
RG_GLOBAL __thread int rg_panicking;
RG_GLOBAL __thread int rg_is_main;
RG_GLOBAL __thread char* rg_tname;          /* thread name (NULL: "main" / "<unnamed>") */
RG_GLOBAL __thread size_t rg_tname_len;
/* the payload message of the last caught panic of this thread */
RG_GLOBAL __thread char* rg_pmsg;
RG_GLOBAL __thread size_t rg_pmsg_len;
RG_GLOBAL __thread int rg_pmsg_static;
typedef struct RgTry { jmp_buf jb; size_t csbase; struct RgTry* prev; } RgTry;
RG_GLOBAL __thread RgTry* rg_try_top;
/* Cleanup-stack switching (for coroutines): each execution context owns a
   cleanup stack (array + count + capacity) and its innermost catch frame.
   rg_cs_swap installs `s` as the current thread's state and returns the
   previous one; a fresh context starts from (RgCsState){0}. */
typedef struct { RgCleanup* cs; size_t n, cap; struct RgTry* try_top; } RgCsState;
/* Simpler form: the current stack pointer/count (capacity kept internally). */
static RgCleanup* rg_cs_save(size_t* n) { *n = rg_csn; return rg_cs; }

RG_GLOBAL int rg_first_panic RG_INIT(1);

__attribute__((noinline)) static void rg_cs_grow(void) {
  size_t n = rg_cscap ? rg_cscap * 2 : 256;
  RgCleanup* p = (RgCleanup*)realloc(rg_cs, n * sizeof(RgCleanup));
  if (!p) { fprintf(stderr, "rustgo: out of memory (cleanup stack)\n"); abort(); }
  rg_cs = p; rg_cscap = n;
}
#define RG_CS_PUSH(P, F, FL) do { if (__builtin_expect(rg_csn >= rg_cscap, 0)) rg_cs_grow(); rg_cs[rg_csn] = (RgCleanup){ (void*)(P), (void (*)(void*))(F), (FL) }; rg_csn++; } while (0)
#define RG_CS_POP(N) (rg_csn -= (N))
static RgCsState rg_cs_swap(RgCsState s) {
  RgCsState old = { rg_cs, rg_csn, rg_cscap, rg_try_top };
  rg_cs = s.cs; rg_csn = s.n; rg_cscap = s.cap; rg_try_top = s.try_top;
  return old;
}
/* Restores a stack obtained from rg_cs_save on this thread (capacity must not
   have changed meanwhile: use rg_cs_swap for independent stacks). */
static void rg_cs_restore(RgCleanup* base, size_t n) { if (base != rg_cs) { rg_cs = base; rg_cscap = n; } rg_csn = n; }

static void rg_tls_run_dtors(void);
static void rg_atexit(void) { rg_tls_run_dtors(); }

static void rg_init(int argc, char** argv) {
  rg_argc = argc; rg_argv = argv;
  rg_is_main = 1;
  setvbuf(stdout, rg_outbuf, _IOFBF, sizeof rg_outbuf);
  atexit(rg_atexit);
  /* like Rust's runtime: writes to closed pipes fail with EPIPE instead of killing the process */
  signal(SIGPIPE, SIG_IGN);
}

static void rg_flush_stdout(void) { fflush(stdout); }

__attribute__((noreturn, cold)) static void rg_panic_str(const char* msg, const char* file, uint32_t line, uint32_t col);
__attribute__((noinline, cold)) static void rg_stdout_failed(void) {
  int e = errno;
  clearerr(stdout);
  if (e == EPIPE) {
    /* drop what is still buffered so that the exit-time flush stays quiet */
    __fpurge(stdout);
    rg_panic_str("failed printing to stdout: Broken pipe (os error 32)", "library/std/src/io/stdio.rs", 1166, 9);
  }
}
static void rg_write_stdout(const uint8_t* p, uint64_t n) { if (n && fwrite(p, 1, n, stdout) != n) rg_stdout_failed(); }
static void rg_write_stderr(const uint8_t* p, uint64_t n) { fflush(stdout); if (n) fwrite(p, 1, n, stderr); }

__attribute__((noreturn)) static void rg_exit(int32_t code) { fflush(stdout); exit(code); }
__attribute__((noreturn)) static void rg_abort(void) { fflush(stdout); abort(); }

static void rg_thread_name_get(const char** p, int* n) {
  if (rg_tname) { *p = rg_tname; *n = (int)rg_tname_len; }
  else if (rg_is_main) { *p = "main"; *n = 4; }
  else { *p = "<unnamed>"; *n = 9; }
}

/* Unwinds the current thread: runs the pending destructors down to the
   innermost catch frame and jumps there, or exits the process (main thread)
   / the thread (other threads) when there is none. */
__attribute__((noreturn)) static void rg_unwind(const char* msg, uint64_t mlen, int is_static) {
  size_t base = rg_try_top ? rg_try_top->csbase : 0;
  rg_panicking = 1;
  if (rg_try_top) {
    /* copy the message first: the destructors may free it */
    char* m = (char*)malloc(mlen + 1);
    if (mlen) memcpy(m, msg, mlen);
    m[mlen] = 0;
    free(rg_pmsg);
    rg_pmsg = m; rg_pmsg_len = mlen; rg_pmsg_static = is_static;
  }
  while (rg_csn > base) {
    rg_csn--;
    RgCleanup c = rg_cs[rg_csn];
    if (*c.flag) { *c.flag = 0; c.f(c.p); }
  }
  if (rg_try_top) longjmp(rg_try_top->jb, 1);
  fflush(stdout);
  if (!rg_is_main) pthread_exit(NULL);
  exit(101);
}

__attribute__((noreturn, cold)) static void rg_panic_full(const char* msg, uint64_t mlen, const char* file, uint64_t flen, uint32_t line, uint32_t col, int is_static) {
  const char* tn; int tl;
  rg_thread_name_get(&tn, &tl);
  fflush(stdout);
  flockfile(stderr);
  fprintf(stderr, "\nthread '%.*s' (%ld) panicked at %.*s:%u:%u:\n%.*s\n", tl, tn, (long)syscall(SYS_gettid), (int)flen, file, line, col, (int)mlen, msg);
  if (__atomic_exchange_n(&rg_first_panic, 0, __ATOMIC_SEQ_CST))
    fprintf(stderr, "note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace\n");
  funlockfile(stderr);
  fflush(stderr);
  if (rg_panicking) { fprintf(stderr, "thread '%.*s' panicked while processing panic. aborting.\n", tl, tn); fflush(stdout); abort(); }
  rg_unwind(msg, mlen, is_static);
}

__attribute__((noreturn, cold)) static void rg_panic_raw(const char* msg, uint64_t mlen, const char* file, uint64_t flen, uint32_t line, uint32_t col) {
  rg_panic_full(msg, mlen, file, flen, line, col, 0);
}

__attribute__((noreturn, cold)) static void rg_panic_impl(Fat msg, Fat file, uint32_t line, uint32_t col) {
  rg_panic_full((const char*)msg.ptr, msg.meta, (const char*)file.ptr, file.meta, line, col, 0);
}

__attribute__((noreturn, cold)) static void rg_panic_impl_static(Fat msg, Fat file, uint32_t line, uint32_t col) {
  rg_panic_full((const char*)msg.ptr, msg.meta, (const char*)file.ptr, file.meta, line, col, 1);
}

/* resume_unwind: unwinds without printing a message */
__attribute__((noreturn, cold)) static void rg_panic_resume(Fat msg, uint8_t is_static) {
  if (rg_panicking) { fflush(stdout); abort(); }
  rg_unwind((const char*)msg.ptr, msg.meta, is_static);
}

__attribute__((noreturn, cold)) static void rg_panic_str(const char* msg, const char* file, uint32_t line, uint32_t col) {
  rg_panic_full(msg, strlen(msg), file, strlen(file), line, col, 1);
}

__attribute__((noreturn, cold)) static void rg_panic_bounds(uint64_t idx, uint64_t len, const char* file, uint32_t line, uint32_t col) {
  char buf[128];
  int n = snprintf(buf, sizeof buf, "index out of bounds: the len is %llu but the index is %llu", (unsigned long long)len, (unsigned long long)idx);
  rg_panic_raw(buf, n, file, strlen(file), line, col);
}

/* Calls f(data), catching a panic: returns 1 if f panicked (the message is
   then available through rg_pmsg). */
__attribute__((noinline)) static uint8_t rg_try(Unit (*f)(uint8_t*), uint8_t* data) {
  RgTry t;
  t.csbase = rg_csn;
  t.prev = rg_try_top;
  rg_try_top = &t;
  if (setjmp(t.jb) == 0) {
    f(data);
    rg_try_top = t.prev;
    return 0;
  }
  rg_try_top = t.prev;
  rg_panicking = 0;
  return 1;
}
static Fat rg_panic_msg(void) { return (Fat){ rg_pmsg ? rg_pmsg : (char*)"", rg_pmsg ? rg_pmsg_len : 0 }; }

/* ---- futex ---- */
static uint8_t rg_futex_wait(uint32_t* p, uint32_t expected, uint64_t timeout_ns) {
  struct timespec ts, *tsp = NULL;
  if (timeout_ns != UINT64_MAX) {
    ts.tv_sec = (time_t)(timeout_ns / 1000000000ull);
    ts.tv_nsec = (long)(timeout_ns % 1000000000ull);
    tsp = &ts;
  }
  long r = syscall(SYS_futex, p, FUTEX_WAIT_PRIVATE, expected, tsp, NULL, 0);
  return !(r < 0 && errno == ETIMEDOUT);
}
static uint8_t rg_futex_wake(uint32_t* p) { return syscall(SYS_futex, p, FUTEX_WAKE_PRIVATE, 1, NULL, NULL, 0) > 0; }
static void rg_futex_wake_all(uint32_t* p) { syscall(SYS_futex, p, FUTEX_WAKE_PRIVATE, 0x7fffffff, NULL, NULL, 0); }

/* ---- thread-local storage (thread_local!) ---- */
typedef struct { uint8_t* p; Unit (*dtor)(uint8_t*); } RgTlsSlot;
RG_GLOBAL __thread RgTlsSlot* rg_tls;
RG_GLOBAL __thread size_t rg_tls_cap;
RG_GLOBAL __thread uint64_t* rg_tls_order;
RG_GLOBAL __thread size_t rg_tls_n, rg_tls_ocap;
RG_GLOBAL uint64_t rg_tls_next_key RG_INIT(1);
static uint64_t rg_tls_new_key(void) { return __atomic_fetch_add(&rg_tls_next_key, 1, __ATOMIC_SEQ_CST); }
static uint8_t* rg_tls_get(uint64_t key) { return key < rg_tls_cap ? rg_tls[key].p : NULL; }
static void rg_tls_set(uint64_t key, uint8_t* p, Unit (*dtor)(uint8_t*)) {
  if (key >= rg_tls_cap) {
    size_t n = rg_tls_cap ? rg_tls_cap : 16;
    while (n <= key) n *= 2;
    RgTlsSlot* q = (RgTlsSlot*)realloc(rg_tls, n * sizeof(RgTlsSlot));
    if (!q) abort();
    memset(q + rg_tls_cap, 0, (n - rg_tls_cap) * sizeof(RgTlsSlot));
    rg_tls = q; rg_tls_cap = n;
  }
  if (dtor && !rg_tls[key].dtor) {
    if (rg_tls_n == rg_tls_ocap) {
      rg_tls_ocap = rg_tls_ocap ? rg_tls_ocap * 2 : 16;
      rg_tls_order = (uint64_t*)realloc(rg_tls_order, rg_tls_ocap * sizeof(uint64_t));
      if (!rg_tls_order) abort();
    }
    rg_tls_order[rg_tls_n++] = key;
  }
  rg_tls[key].p = p;
  if (dtor) rg_tls[key].dtor = dtor;
}
/* Runs the destructors of this thread's thread-locals (most recently
   registered first); a destroyed slot holds the sentinel pointer 1. */
static void rg_tls_run_dtors(void) {
  while (rg_tls_n > 0) {
    uint64_t key = rg_tls_order[--rg_tls_n];
    RgTlsSlot* s = &rg_tls[key];
    uint8_t* p = s->p;
    Unit (*d)(uint8_t*) = s->dtor;
    s->dtor = NULL;
    if (p && p != (uint8_t*)1 && d) { s->p = (uint8_t*)1; d(p); }
  }
}

/* ---- threads ---- */
typedef struct RgThread {
  Unit (*entry)(uint8_t*);
  uint8_t* data;
  uint32_t finished; /* futex word */
  int refs;
} RgThread;

static void rg_thread_release(uint64_t h) {
  RgThread* th = (RgThread*)(uintptr_t)h;
  if (th && __atomic_sub_fetch(&th->refs, 1, __ATOMIC_ACQ_REL) == 0) free(th);
}
static void rg_thread_retain(uint64_t h) {
  RgThread* th = (RgThread*)(uintptr_t)h;
  __atomic_add_fetch(&th->refs, 1, __ATOMIC_ACQ_REL);
}

static void rg_thread_cleanup(void* arg) {
  RgThread* th = (RgThread*)arg;
  free(rg_cs); rg_cs = NULL; rg_csn = rg_cscap = 0;
  free(rg_tls); rg_tls = NULL; rg_tls_cap = 0;
  free(rg_tls_order); rg_tls_order = NULL; rg_tls_n = rg_tls_ocap = 0;
  free(rg_pmsg); rg_pmsg = NULL;
  free(rg_tname); rg_tname = NULL;
  __atomic_store_n(&th->finished, 1, __ATOMIC_SEQ_CST);
  rg_futex_wake_all(&th->finished);
  rg_thread_release((uint64_t)(uintptr_t)th);
}

static void* rg_thread_start(void* arg) {
  RgThread* th = (RgThread*)arg;
  pthread_cleanup_push(rg_thread_cleanup, th);
  th->entry(th->data);
  rg_tls_run_dtors();
  pthread_cleanup_pop(1);
  return NULL;
}

/* Starts a detached thread running entry(data); returns its handle (with
   two references: the thread's and the caller's), or 0 on failure. */
static uint64_t rg_thread_spawn(Unit (*entry)(uint8_t*), uint8_t* data, uint64_t stack) {
  RgThread* th = (RgThread*)calloc(1, sizeof(RgThread));
  if (!th) return 0;
  th->entry = entry; th->data = data; th->refs = 2;
  pthread_attr_t attr;
  pthread_attr_init(&attr);
  pthread_attr_setdetachstate(&attr, PTHREAD_CREATE_DETACHED);
  if (stack == 0) stack = 8u << 20;
  if (stack < 65536) stack = 65536;
  stack = (stack + 4095) & ~(uint64_t)4095;
  pthread_attr_setstacksize(&attr, stack);
  pthread_t t;
  int r = pthread_create(&t, &attr, rg_thread_start, th);
  pthread_attr_destroy(&attr);
  if (r != 0) { free(th); return 0; }
  return (uint64_t)(uintptr_t)th;
}
static void rg_thread_join(uint64_t h) {
  RgThread* th = (RgThread*)(uintptr_t)h;
  while (__atomic_load_n(&th->finished, __ATOMIC_SEQ_CST) == 0) rg_futex_wait(&th->finished, 0, UINT64_MAX);
}
static uint8_t rg_thread_is_finished(uint64_t h) {
  return __atomic_load_n(&((RgThread*)(uintptr_t)h)->finished, __ATOMIC_SEQ_CST) != 0;
}
static void rg_thread_set_name(const uint8_t* p, uint64_t n) {
  free(rg_tname);
  rg_tname = (char*)malloc(n + 1);
  if (n) memcpy(rg_tname, p, n);
  rg_tname[n] = 0;
  rg_tname_len = n;
}
static void rg_sleep_ns(uint64_t ns) {
  struct timespec ts = { (time_t)(ns / 1000000000ull), (long)(ns % 1000000000ull) };
  while (nanosleep(&ts, &ts) != 0 && errno == EINTR) {}
}
static uint64_t rg_clock_ns(clockid_t c) {
  struct timespec ts;
  clock_gettime(c, &ts);
  return (uint64_t)ts.tv_sec * 1000000000ull + (uint64_t)ts.tv_nsec;
}
static uint64_t rg_available_parallelism(void) {
  long n = sysconf(_SC_NPROCESSORS_ONLN);
  return n > 0 ? (uint64_t)n : 1;
}

/* ---- atomics ---- */
#define RG_ATOMIC_MAX(P, V) ({ __typeof__(*(P)) _o = __atomic_load_n((P), __ATOMIC_SEQ_CST), _v = (V); \
  while (_o < _v && !__atomic_compare_exchange_n((P), &_o, _v, 1, __ATOMIC_SEQ_CST, __ATOMIC_SEQ_CST)) {} _o; })
#define RG_ATOMIC_MIN(P, V) ({ __typeof__(*(P)) _o = __atomic_load_n((P), __ATOMIC_SEQ_CST), _v = (V); \
  while (_o > _v && !__atomic_compare_exchange_n((P), &_o, _v, 1, __ATOMIC_SEQ_CST, __ATOMIC_SEQ_CST)) {} _o; })

/* ---- environment / stdin ---- */
static const uint8_t* rg_arg_ptr(uint64_t i) { return i < (uint64_t)rg_argc ? (const uint8_t*)rg_argv[i] : (const uint8_t*)""; }
static uint64_t rg_arg_len(uint64_t i) { return i < (uint64_t)rg_argc ? strlen(rg_argv[i]) : 0; }
static const uint8_t* rg_env_get(const uint8_t* p, uint64_t n) {
  char buf[512];
  char* k = n < sizeof buf ? buf : (char*)malloc(n + 1);
  memcpy(k, p, n); k[n] = 0;
  const uint8_t* r = (const uint8_t*)getenv(k);
  if (k != buf) free(k);
  return r;
}
static uint64_t rg_env_count(void) { uint64_t n = 0; while (environ && environ[n]) n++; return n; }
static const uint8_t* rg_env_entry(uint64_t i) { return (const uint8_t*)environ[i]; }
RG_GLOBAL uint8_t rg_stdin_buf[1 << 16];
RG_GLOBAL size_t rg_stdin_pos, rg_stdin_end;
/* Makes buffered stdin data available; returns its length (0 at EOF, -1 on error). */
static int64_t rg_stdin_fill(void) {
  if (rg_stdin_pos < rg_stdin_end) return (int64_t)(rg_stdin_end - rg_stdin_pos);
  ssize_t n;
  do { n = read(0, rg_stdin_buf, sizeof rg_stdin_buf); } while (n < 0 && errno == EINTR);
  rg_stdin_pos = 0;
  rg_stdin_end = n > 0 ? (size_t)n : 0;
  return n;
}
static const uint8_t* rg_stdin_ptr(void) { return rg_stdin_buf + rg_stdin_pos; }
static void rg_stdin_consume(uint64_t n) { rg_stdin_pos += n; if (rg_stdin_pos > rg_stdin_end) rg_stdin_pos = rg_stdin_end; }

#define rg_unreachable() do { fflush(stdout); fprintf(stderr, "internal error: entered unreachable code (rustgo)\n"); abort(); } while (0)

static void rg_drop_noop(void* p) { (void)p; }

/* ---- checked arithmetic (debug-mode semantics) ---- */
#ifdef RG_NO_OVERFLOW_CHECKS
/* release profile: integer arithmetic wraps, shift amounts are masked
 * (division by zero and MIN/-1 still panic, as in rustc) */
#define RG_ADD(T, a, b, F, L, C) ((T)((T)(a) + (T)(b)))
#define RG_SUB(T, a, b, F, L, C) ((T)((T)(a) - (T)(b)))
#define RG_MUL(T, a, b, F, L, C) ((T)((T)(a) * (T)(b)))
#define RG_TMIN(T) ((T)((unsigned __int128)1 << (sizeof(T) * 8 - 1)))
#define RG_DIV(T, a, b, F, L, C) ({ T _a = (a), _b = (b); \
  if (_b == 0) rg_panic_str("attempt to divide by zero", F, L, C); \
  if ((T)-1 < 0 && _b == (T)-1 && _a == RG_TMIN(T)) rg_panic_str("attempt to divide with overflow", F, L, C); \
  (T)(_a / _b); })
#define RG_REM(T, a, b, F, L, C) ({ T _a = (a), _b = (b); \
  if (_b == 0) rg_panic_str("attempt to calculate the remainder with a divisor of zero", F, L, C); \
  if ((T)-1 < 0 && _b == (T)-1 && _a == RG_TMIN(T)) rg_panic_str("attempt to calculate the remainder with overflow", F, L, C); \
  (T)(_a % _b); })
#define RG_SHL(T, BITS, a, b, F, L, C) ((T)((unsigned __int128)(T)(a) << ((unsigned __int128)(b) & ((BITS) - 1))))
#define RG_SHR(T, BITS, a, b, F, L, C) ((T)((T)(a) >> ((unsigned __int128)(b) & ((BITS) - 1))))
#define RG_NEG(T, a, F, L, C) ((T)(-(T)(a)))
#else
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
#endif
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

/* CPUID for core::arch::x86_64::__cpuid_count (declared in std as an extern
 * "C" fn). Weak so every translation unit may carry a copy. */
__attribute__((weak, used)) void rustgo_cpuid(uint32_t leaf, uint32_t sub, uint32_t* out) {
  unsigned int a, b, c, d;
  __cpuid_count(leaf, sub, a, b, c, d);
  out[0] = a; out[1] = b; out[2] = c; out[3] = d;
}
