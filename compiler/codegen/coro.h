#define RG_ALIGN_UP(x, a) (((uint64_t)(x) + (uint64_t)(a) - 1) / (uint64_t)(a) * (uint64_t)(a))
#define RG_MAXU(a, b) ((uint64_t)(a) > (uint64_t)(b) ? (uint64_t)(a) : (uint64_t)(b))
/* rustgo coroutines (stackful) used to run `async` blocks */
#include <ucontext.h>
#include <sys/mman.h>

typedef struct RgCoro {
  ucontext_t ctx;
  ucontext_t caller;
  void (*entry)(uint8_t*);
  uint8_t* data;
  void* stack;
  size_t size;
  int done;
  struct RgCoro* prev;
  RgCsState cs;     /* the coroutine's own cleanup stack */
  RgCsState saved;  /* the resumer's cleanup stack while running */
} RgCoro;

RG_GLOBAL __thread RgCoro* rg_coro_current;
RG_GLOBAL __thread uint8_t* rg_coro_cx;

#define RG_CORO_STACK (512 * 1024)

static void rg_coro_start(unsigned lo, unsigned hi) {
  RgCoro* c = (RgCoro*)(((uintptr_t)hi << 32) | (uintptr_t)lo);
  c->entry(c->data);
  c->done = 1;
  /* returning resumes c->caller via uc_link */
}

static uint8_t* rg_coro_new(void (*entry)(uint8_t*), uint8_t* data) {
  RgCoro* c = (RgCoro*)calloc(1, sizeof(RgCoro));
  c->entry = entry;
  c->data = data;
  c->size = RG_CORO_STACK;
  c->stack = mmap(NULL, c->size, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS | MAP_NORESERVE, -1, 0);
  if (c->stack == MAP_FAILED) { fprintf(stderr, "rustgo: cannot allocate coroutine stack\n"); abort(); }
  getcontext(&c->ctx);
  c->ctx.uc_stack.ss_sp = c->stack;
  c->ctx.uc_stack.ss_size = c->size;
  c->ctx.uc_link = &c->caller;
  uintptr_t p = (uintptr_t)c;
  makecontext(&c->ctx, (void (*)(void))rg_coro_start, 2, (unsigned)(p & 0xffffffffu), (unsigned)(p >> 32));
  return (uint8_t*)c;
}

/* Runs the coroutine until it yields or finishes; returns 1 if finished. */
static uint8_t rg_coro_resume(uint8_t* p) {
  RgCoro* c = (RgCoro*)p;
  c->prev = rg_coro_current;
  rg_coro_current = c;
  c->saved = rg_cs_swap(c->cs);
  swapcontext(&c->caller, &c->ctx);
  c->cs = rg_cs_swap(c->saved);
  rg_coro_current = c->prev;
  return (uint8_t)c->done;
}

static void rg_coro_yield(void) {
  RgCoro* c = rg_coro_current;
  if (!c) return; /* not inside an async block: nothing to suspend */
  swapcontext(&c->ctx, &c->caller);
}

/* Frees a coroutine. A suspended one is cancelled first: like dropping a
   suspended future in Rust, the values live on its stack (its locals,
   temporaries and the captures of the async block's closure) are dropped,
   innermost first, by running its pending cleanup entries (the same
   destructors a panic would run). Its stack stays mapped meanwhile. */
static void rg_coro_free(uint8_t* p) {
  RgCoro* c = (RgCoro*)p;
  if (!c) return;
  if (!c->done) {
    while (c->cs.n > 0) {
      c->cs.n--;
      RgCleanup e = c->cs.cs[c->cs.n];
      if (*e.flag) { *e.flag = 0; e.f(e.p); }
    }
  }
  munmap(c->stack, c->size);
  free(c->cs.cs);
  free(c);
}
