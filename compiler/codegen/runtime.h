/* rustgo runtime (generated code relies on GNU C extensions) */
#include <stdint.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>
#include <strings.h>

typedef struct {} Unit;

/* #[inline] functions too large to be worth inlining in debug builds
   (see inlineSmallBytes in codegen/gen.go) */
#ifdef RG_NO_OVERFLOW_CHECKS
#define RG_INLINE_BIG inline
#else
#define RG_INLINE_BIG
#endif
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
#include <sys/resource.h>
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
/* #[track_caller]: the caller stores the call's location (a pointer to a
 * core::panic::Location) right before the call; `tracked` says it was
 * propagated from a track_caller caller rather than being the call site. */
typedef struct { Fat file; uint32_t line, col; } RgLoc;
/* std's entry point for runtime-raised panics (installed by main) */
RG_GLOBAL void (*rg_std_panic)(Fat msg, Fat file, uint32_t line, uint32_t col, uint8_t is_static);
RG_GLOBAL __thread const RgLoc* rg_caller_loc;
RG_GLOBAL __thread int rg_caller_tracked;
RG_GLOBAL __thread int rg_is_main;
RG_GLOBAL __thread char* rg_tname;          /* thread name (NULL: "main" / "<unnamed>") */
RG_GLOBAL __thread size_t rg_tname_len;
RG_GLOBAL __thread void* rg_altstack;     /* sigaltstack for stack overflow reports */
/* the stack limit the process started with, when rg_init raised it */
RG_GLOBAL struct rlimit rg_stack_rlim0;
RG_GLOBAL int rg_stack_rlim_raised;
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
/* pushes an entry below the top K ones (a temporary that outlives them) */
#define RG_CS_INS(K, P, F, FL) do { if (__builtin_expect(rg_csn >= rg_cscap, 0)) rg_cs_grow(); memmove(&rg_cs[rg_csn - (K) + 1], &rg_cs[rg_csn - (K)], (K) * sizeof(RgCleanup)); rg_cs[rg_csn - (K)] = (RgCleanup){ (void*)(P), (void (*)(void*))(F), (FL) }; rg_csn++; } while (0)
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

/* Stack overflow detection (like Rust's runtime): a SIGSEGV/SIGBUS whose
 * address lies in the guard area below the current thread's stack prints
 * "thread '<name>' (<tid>) has overflowed its stack" and aborts. */
static void rg_stack_overflow_handler(int sig, siginfo_t* si, void* uc) {
  (void)uc;
  uintptr_t addr = (uintptr_t)si->si_addr;
  pthread_attr_t a;
  if (pthread_getattr_np(pthread_self(), &a) == 0) {
    void* lo = 0; size_t sz = 0, guard = 0;
    pthread_attr_getstack(&a, &lo, &sz);
    pthread_attr_getguardsize(&a, &guard);
    pthread_attr_destroy(&a);
    uintptr_t l = (uintptr_t)lo;
    if (guard < 4096) guard = 4096;
    if (addr + guard + (1u << 20) >= l && addr < l + 4096) {
      char buf[256];
      const char* name = rg_tname ? rg_tname : (rg_is_main ? "main" : "<unnamed>");
      int nl = rg_tname ? (int)rg_tname_len : (int)strlen(name);
      int n = snprintf(buf, sizeof buf, "\nthread '%.*s' (%ld) has overflowed its stack\nfatal runtime error: stack overflow, aborting\n", nl, name, (long)syscall(SYS_gettid));
      if (n > 0) { ssize_t w = write(2, buf, (size_t)n); (void)w; }
      signal(SIGABRT, SIG_DFL);
      abort();
    }
  }
  signal(sig, SIG_DFL);
  raise(sig);
}

static void rg_install_stack_guard(void) {
  stack_t ss;
  ss.ss_size = 64 * 1024;
  ss.ss_sp = malloc(ss.ss_size);
  ss.ss_flags = 0;
  rg_altstack = ss.ss_sp;
  if (ss.ss_sp && sigaltstack(&ss, NULL) == 0) {
    struct sigaction sa;
    memset(&sa, 0, sizeof sa);
    sa.sa_sigaction = rg_stack_overflow_handler;
    sa.sa_flags = SA_SIGINFO | SA_ONSTACK;
    sigaction(SIGSEGV, &sa, NULL);
    sigaction(SIGBUS, &sa, NULL);
  }
}

/* The generated C keeps more of a function's temporaries in its frame than
   rustc does (drop flags, values registered for unwinding), so recursion
   reaches the end of the stack 2-3x sooner. Like the 8MB default for
   spawned threads (rustc: 2MB), give the main thread 4x the stack it was
   started with (it grows on demand; at most 64MB, within the hard limit).
   Child processes get the original limit back (rg_restore_stack_limit). */
static void rg_raise_main_stack(void) {
  struct rlimit rl;
  if (getrlimit(RLIMIT_STACK, &rl) != 0 || rl.rlim_cur == RLIM_INFINITY) return;
  rlim_t want = rl.rlim_cur * 4;
  if (want > ((rlim_t)64 << 20)) want = (rlim_t)64 << 20;
  if (rl.rlim_max != RLIM_INFINITY && want > rl.rlim_max) want = rl.rlim_max;
  if (want <= rl.rlim_cur) return;
  struct rlimit raised = rl;
  raised.rlim_cur = want;
  if (setrlimit(RLIMIT_STACK, &raised) == 0) { rg_stack_rlim0 = rl; rg_stack_rlim_raised = 1; }
}
static void rg_restore_stack_limit(void) {
  if (rg_stack_rlim_raised) setrlimit(RLIMIT_STACK, &rg_stack_rlim0);
}

static void rg_init(int argc, char** argv) {
  rg_argc = argc; rg_argv = argv;
  rg_is_main = 1;
  rg_raise_main_stack();
  setvbuf(stdout, rg_outbuf, _IOFBF, sizeof rg_outbuf);
  atexit(rg_atexit);
  /* like Rust's runtime: writes to closed pipes fail with EPIPE instead of killing the process */
  signal(SIGPIPE, SIG_IGN);
  rg_install_stack_guard();
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
/* stdout is line-buffered like std's LineWriter (also when it is not a
   terminal): text up to the last newline of a write is flushed, the rest
   stays buffered. */
static void rg_write_stdout(const uint8_t* p, uint64_t n) {
  if (!n) return;
  const uint8_t* nl = NULL;
  for (uint64_t i = n; i > 0; i--) if (p[i - 1] == '\n') { nl = p + i - 1; break; }
  if (!nl) { if (fwrite(p, 1, n, stdout) != n) rg_stdout_failed(); return; }
  uint64_t k = (uint64_t)(nl - p) + 1;
  if (fwrite(p, 1, k, stdout) != k || fflush(stdout) != 0) rg_stdout_failed();
  if (k < n && fwrite(p + k, 1, n - k, stdout) != n - k) rg_stdout_failed();
}
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
    /* the flag is left set: several entries (the parts of a partially moved
       value) may share it */
    if (*c.flag) c.f(c.p);
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
  if (rg_std_panic && !rg_panicking) {
    /* through std (panic hooks, payload types) */
    rg_std_panic((Fat){ (void*)msg, strlen(msg) }, (Fat){ (void*)file, strlen(file) }, line, col, 1);
    abort();
  }
  rg_panic_full(msg, strlen(msg), file, strlen(file), line, col, 1);
}

__attribute__((noreturn, cold)) static void rg_panic_bounds(uint64_t idx, uint64_t len, const char* file, uint32_t line, uint32_t col) {
  char buf[128];
  int n = snprintf(buf, sizeof buf, "index out of bounds: the len is %llu but the index is %llu", (unsigned long long)len, (unsigned long long)idx);
  if (rg_std_panic && !rg_panicking) {
    rg_std_panic((Fat){ (void*)buf, (uint64_t)n }, (Fat){ (void*)file, strlen(file) }, line, col, 0);
    abort();
  }
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
typedef struct { uint8_t* p; Unit (*dtor)(uint8_t*); uint8_t late; } RgTlsSlot;
RG_GLOBAL __thread RgTlsSlot* rg_tls;
RG_GLOBAL __thread size_t rg_tls_cap;
RG_GLOBAL __thread uint64_t* rg_tls_order;
RG_GLOBAL __thread size_t rg_tls_n, rg_tls_ocap;
RG_GLOBAL uint64_t rg_tls_next_key RG_INIT(1);
static uint64_t rg_tls_new_key(void) { return __atomic_fetch_add(&rg_tls_next_key, 1, __ATOMIC_SEQ_CST); }
static uint8_t* rg_tls_get(uint64_t key) { return key < rg_tls_cap ? rg_tls[key].p : NULL; }
static void rg_tls_set(uint64_t key, uint8_t* p, Unit (*dtor)(uint8_t*), uint8_t late) {
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
  if (dtor) { rg_tls[key].dtor = dtor; rg_tls[key].late = late; }
}
/* Runs the destructors of this thread's thread-locals (most recently
   registered first); a destroyed slot holds the sentinel pointer
   UINTPTR_MAX (1 is the address of a boxed zero-sized value). Values
   without drop glue ("late" slots) stay accessible, like in rustc, until
   every other destructor has run; then their storage is freed. */
static void rg_tls_run_dtors(void) {
  size_t nlate = 0;
  while (rg_tls_n > nlate) {
    uint64_t key = rg_tls_order[--rg_tls_n];
    RgTlsSlot* s = &rg_tls[key];
    if (s->late) {
      /* keep it: move it below the slots still to be destroyed */
      memmove(rg_tls_order + 1, rg_tls_order, rg_tls_n * sizeof(uint64_t));
      rg_tls_order[0] = key;
      rg_tls_n++;
      nlate++;
      continue;
    }
    uint8_t* p = s->p;
    Unit (*d)(uint8_t*) = s->dtor;
    s->dtor = NULL;
    if (p && p != (uint8_t*)UINTPTR_MAX && d) { s->p = (uint8_t*)UINTPTR_MAX; d(p); }
  }
  while (rg_tls_n > 0) {
    uint64_t key = rg_tls_order[--rg_tls_n];
    RgTlsSlot* s = &rg_tls[key];
    uint8_t* p = s->p;
    Unit (*d)(uint8_t*) = s->dtor;
    s->dtor = NULL;
    s->late = 0;
    if (p && p != (uint8_t*)UINTPTR_MAX && d) { s->p = (uint8_t*)UINTPTR_MAX; d(p); }
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
  rg_install_stack_guard(); /* per-thread alternate signal stack */
  pthread_cleanup_push(rg_thread_cleanup, th);
  th->entry(th->data);
  rg_tls_run_dtors();
  pthread_cleanup_pop(1);
  if (rg_altstack) {
    stack_t ss; memset(&ss, 0, sizeof ss); ss.ss_flags = SS_DISABLE;
    sigaltstack(&ss, NULL);
    free(rg_altstack); rg_altstack = NULL;
  }
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
#define RG_OVERFLOW_CHECKS 0
#else
#define RG_OVERFLOW_CHECKS 1
#endif
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
/* the `#[global_allocator]`, when installed (rg_sys_* is the system one) */
RG_GLOBAL uint8_t* (*rg_ga_alloc)(uint64_t, uint64_t);
RG_GLOBAL void (*rg_ga_dealloc)(uint8_t*, uint64_t, uint64_t);
RG_GLOBAL uint8_t* (*rg_ga_realloc)(uint8_t*, uint64_t, uint64_t, uint64_t);
static uint8_t* rg_sys_alloc(uint64_t size, uint64_t align) {
  void* p;
  if (size == 0) size = 1;
  if (align <= 16) p = malloc(size);
  else { size = (size + align - 1) / align * align; p = aligned_alloc(align, size); }
  if (!p) { fprintf(stderr, "memory allocation of %llu bytes failed\n", (unsigned long long)size); abort(); }
  return (uint8_t*)p;
}
static uint8_t* rg_sys_realloc(uint8_t* p, uint64_t old, uint64_t align, uint64_t size) {
  if (align <= 16) {
    void* q = realloc(p, size ? size : 1);
    if (!q) { fprintf(stderr, "memory allocation failed\n"); abort(); }
    return (uint8_t*)q;
  }
  uint8_t* q = rg_sys_alloc(size, align);
  memcpy(q, p, old < size ? old : size);
  free(p);
  return q;
}
static void rg_sys_dealloc(uint8_t* p, uint64_t size, uint64_t align) { (void)size; (void)align; free(p); }
static uint8_t* rg_alloc(uint64_t size, uint64_t align) { return rg_ga_alloc ? rg_ga_alloc(size, align) : rg_sys_alloc(size, align); }
static uint8_t* rg_realloc(uint8_t* p, uint64_t old, uint64_t align, uint64_t size) { return rg_ga_realloc ? rg_ga_realloc(p, old, align, size) : rg_sys_realloc(p, old, align, size); }
static void rg_dealloc(uint8_t* p, uint64_t size, uint64_t align) { if (rg_ga_dealloc) rg_ga_dealloc(p, size, align); else rg_sys_dealloc(p, size, align); }
/* memcmp, inline for short strings (most comparisons; a call of the C
   library's costs more than the comparison) */
static inline int rg_memcmp(const uint8_t* a, const uint8_t* b, uint64_t n) {
  if (n <= 16) {
    if (n >= 8) {
      uint64_t x, y;
      memcpy(&x, a, 8); memcpy(&y, b, 8);
      if (x == y) { memcpy(&x, a + n - 8, 8); memcpy(&y, b + n - 8, 8); }
      if (x == y) return 0;
      x = __builtin_bswap64(x); y = __builtin_bswap64(y);
      return x < y ? -1 : 1;
    }
    if (n >= 4) {
      uint32_t x, y;
      memcpy(&x, a, 4); memcpy(&y, b, 4);
      if (x == y) { memcpy(&x, a + n - 4, 4); memcpy(&y, b + n - 4, 4); }
      if (x == y) return 0;
      x = __builtin_bswap32(x); y = __builtin_bswap32(y);
      return x < y ? -1 : 1;
    }
    for (uint64_t i = 0; i < n; i++) if (a[i] != b[i]) return a[i] < b[i] ? -1 : 1;
    return 0;
  }
  return memcmp(a, b, n);
}
/* count_ones: without -mpopcnt, __builtin_popcountll is a libgcc call */
static inline uint64_t rg_popcount64(uint64_t x) {
#ifdef __POPCNT__
  return (uint64_t)__builtin_popcountll(x);
#else
  x = x - ((x >> 1) & 0x5555555555555555ULL);
  x = (x & 0x3333333333333333ULL) + ((x >> 2) & 0x3333333333333333ULL);
  x = (x + (x >> 4)) & 0x0f0f0f0f0f0f0f0fULL;
  return (x * 0x0101010101010101ULL) >> 56;
#endif
}
/* (GNU extensions, declared here as <string.h> only does with _GNU_SOURCE) */
extern void* memmem(const void* h, size_t hn, const void* n, size_t nn);
extern void* memrchr(const void* s, int c, size_t n);
static uint64_t rg_memchr(const uint8_t* p, uint8_t c, uint64_t n) {
  const uint8_t* r = n ? (const uint8_t*)memchr(p, c, n) : 0;
  return r ? (uint64_t)(r - p) : n;
}
static uint64_t rg_memrchr(const uint8_t* p, uint8_t c, uint64_t n) {
  const uint8_t* r = n ? (const uint8_t*)memrchr(p, c, n) : 0;
  return r ? (uint64_t)(r - p) : n;
}
static uint64_t rg_memmem(const uint8_t* h, uint64_t hn, const uint8_t* nd, uint64_t nn) {
  const uint8_t* r = (const uint8_t*)memmem(h, hn, nd, nn);
  return r ? (uint64_t)(r - h) : UINT64_MAX;
}

static inline int rg_str_eq(Fat a, Fat b) { return a.meta == b.meta && rg_memcmp((const uint8_t*)a.ptr, (const uint8_t*)b.ptr, a.meta) == 0; }

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

/* Exact arithmetic on unsigned big integers (enough for any double:
 * the shortest-digits algorithm below needs about 1140 bits). */
typedef struct { int n; uint32_t w[44]; } RgBig;
static void rgb_set(RgBig* a, uint64_t v) { a->n = 0; while (v) { a->w[a->n++] = (uint32_t)v; v >>= 32; } }
static inline void rgb_mul_small(RgBig* a, uint32_t m) {
  uint64_t c = 0;
  for (int i = 0; i < a->n; i++) { uint64_t t = (uint64_t)a->w[i] * m + c; a->w[i] = (uint32_t)t; c = t >> 32; }
  if (c) a->w[a->n++] = (uint32_t)c;
}
static void rgb_mul_pow10(RgBig* a, int k) {
  static const uint32_t p10[9] = { 1, 10, 100, 1000, 10000, 100000, 1000000, 10000000, 100000000 };
  while (k >= 9) { rgb_mul_small(a, 1000000000u); k -= 9; }
  if (k > 0) rgb_mul_small(a, p10[k]);
}
static void rgb_shl(RgBig* a, int k) {
  if (a->n == 0 || k == 0) return;
  int ws = k >> 5, bs = k & 31;
  if (bs) {
    uint32_t c = 0;
    for (int i = 0; i < a->n; i++) { uint32_t w = a->w[i]; a->w[i] = (w << bs) | c; c = w >> (32 - bs); }
    if (c) a->w[a->n++] = c;
  }
  if (ws) { memmove(a->w + ws, a->w, (size_t)a->n * 4); memset(a->w, 0, (size_t)ws * 4); a->n += ws; }
}
static inline int rgb_cmp(const RgBig* a, const RgBig* b) {
  if (a->n != b->n) return a->n < b->n ? -1 : 1;
  for (int i = a->n - 1; i >= 0; i--) if (a->w[i] != b->w[i]) return a->w[i] < b->w[i] ? -1 : 1;
  return 0;
}
/* r = a + b */
static inline void rgb_add(RgBig* r, const RgBig* a, const RgBig* b) {
  int n = a->n > b->n ? a->n : b->n; uint64_t c = 0;
  for (int i = 0; i < n; i++) {
    uint64_t t = c + (i < a->n ? a->w[i] : 0) + (i < b->n ? b->w[i] : 0);
    r->w[i] = (uint32_t)t; c = t >> 32;
  }
  r->n = n;
  if (c) r->w[r->n++] = (uint32_t)c;
}
/* a -= b (a >= b) */
static void rgb_sub(RgBig* a, const RgBig* b) {
  int64_t c = 0;
  for (int i = 0; i < a->n; i++) {
    int64_t t = (int64_t)a->w[i] - (int64_t)(i < b->n ? b->w[i] : 0) + c;
    a->w[i] = (uint32_t)t; c = t < 0 ? -1 : 0;
  }
  while (a->n > 0 && a->w[a->n - 1] == 0) a->n--;
}

/* Shortest round-trip digits of a finite nonzero |a| (as a float when
 * is_f32): Burger & Dybvig's free-format algorithm with exact arithmetic,
 * which like the real library's Grisu/Dragon picks the shortest digit
 * string that reads back as `a`, and the one closest to `a` among those
 * (ties up). Returns the number of digits, sets *exp10 (scientific
 * exponent). */
static int rg_shortest(double a, int is_f32, char* digits, int* exp10) {
  if (a == 0) { digits[0] = '0'; digits[1] = 0; *exp10 = 0; return 1; }
  if (a < 0) a = -a;
  uint64_t f, hidden; int e, min_e;
  if (is_f32) {
    float x = (float)a; uint32_t bits; memcpy(&bits, &x, 4);
    int be = (int)((bits >> 23) & 0xff); uint32_t m = bits & 0x7fffff;
    if (be == 0) { f = m; e = -149; } else { f = m | 0x800000u; e = be - 150; }
    hidden = 1u << 23; min_e = -149;
    a = (double)x;
  } else {
    uint64_t bits; memcpy(&bits, &a, 8);
    int be = (int)((bits >> 52) & 0x7ff); uint64_t m = bits & ((1ULL << 52) - 1);
    if (be == 0) { f = m; e = -1074; } else { f = m | (1ULL << 52); e = be - 1075; }
    hidden = 1ULL << 52; min_e = -1074;
  }
  if (a < (is_f32 ? 16777216.0 : 9007199254740992.0) && a == (double)(uint64_t)a) {
    /* an integer below 2^53 (2^24 for f32: its neighbours are at most 1 away): its
       digits, without trailing zeros */
    char buf[24]; int n = 0; uint64_t u = (uint64_t)a;
    while (u) { buf[n++] = (char)('0' + u % 10); u /= 10; }
    *exp10 = n - 1;
    int z = 0; while (z < n - 1 && buf[z] == '0') z++;
    int nd = 0; for (int i = n - 1; i >= z; i--) digits[nd++] = buf[i];
    digits[nd] = 0;
    return nd;
  }
  /* bounds of the rounding interval are inclusive for an even mantissa */
  int ok = (f & 1) == 0;
  /* v = r/s; the interval is [v - mm/s, v + mp/s] */
  RgBig r, s, mp, mm, t;
  if (e >= 0) {
    rgb_set(&r, f); rgb_set(&mp, 1);
    if (f != hidden) { rgb_shl(&r, e + 1); rgb_set(&s, 2); rgb_shl(&mp, e); mm = mp; }
    else { rgb_shl(&r, e + 2); rgb_set(&s, 4); rgb_shl(&mp, e + 1); rgb_set(&mm, 1); rgb_shl(&mm, e); }
  } else {
    rgb_set(&r, f); rgb_set(&s, 1);
    if (e == min_e || f != hidden) { rgb_shl(&r, 1); rgb_shl(&s, 1 - e); rgb_set(&mp, 1); rgb_set(&mm, 1); }
    else { rgb_shl(&r, 2); rgb_shl(&s, 2 - e); rgb_set(&mp, 2); rgb_set(&mm, 1); }
  }
  /* scale so that the digits are those of v / 10^k, 10^(k-1) <= high < 10^k */
  int k = (int)ceil(log10(a) - 1e-10);
  if (k >= 0) rgb_mul_pow10(&s, k);
  else { rgb_mul_pow10(&r, -k); rgb_mul_pow10(&mp, -k); rgb_mul_pow10(&mm, -k); }
  rgb_add(&t, &r, &mp);
  { int c = rgb_cmp(&t, &s); if (ok ? c >= 0 : c > 0) { rgb_mul_small(&s, 10); k++; } }
  int n = 0;
  for (;;) {
    rgb_mul_small(&r, 10); rgb_mul_small(&mp, 10); rgb_mul_small(&mm, 10);
    int d = 0;
    while (rgb_cmp(&r, &s) >= 0) { rgb_sub(&r, &s); d++; }
    int c1 = rgb_cmp(&r, &mm);
    int tc1 = ok ? c1 <= 0 : c1 < 0;
    rgb_add(&t, &r, &mp);
    int c2 = rgb_cmp(&t, &s);
    int tc2 = ok ? c2 >= 0 : c2 > 0;
    if (!tc1 && !tc2) { digits[n++] = (char)('0' + d); if (n >= 30) break; continue; }
    if (tc1 && tc2) {
      RgBig r2 = r; rgb_shl(&r2, 1);
      if (rgb_cmp(&r2, &s) >= 0) d++;
    } else if (tc2) d++;
    digits[n++] = (char)('0' + d);
    break;
  }
  /* (a rounded-up 9 carries into the previous digits) */
  for (int i = n - 1; i > 0 && digits[i] > '9'; i--) { digits[i] = '0'; digits[i - 1]++; }
  if (digits[0] > '9') { digits[0] = '1'; k++; }
  while (n > 1 && digits[n - 1] == '0') n--;
  digits[n] = 0;
  *exp10 = k - 1;
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
