// First-access trace of the legacy address arena of a native (i686) sa-game run.
//
// LD_PRELOAD library: once sa-game has filled the arena from gta_sa.exe (its `mprotect` of
// `.rdata`), every page of the watched ranges is made inaccessible. A fault on a watched page
// records the faulting address as read-first or write-first (page-fault error code), opens the
// page, single-steps the faulting instruction (TF) and closes the page again. A page whose last
// $SA_TRACE_STABLE faults (default 100000) found no new address is left open (counted as saturated;
// arena_usage.py then keeps every object on it). At exit the per-byte
// state is written to $SA_TRACE_OUT.<pid> (binary, see dump()).
//
// Build: gcc -m32 -O2 -shared -fPIC -o arena_trace.so arena_trace.c -ldl
// Run:   WRAP="env LD_PRELOAD=$PWD/arena_trace.so SA_TRACE_OUT=/path/trace" tools/e2e/scenarios/frontier.sh ...
// Analyse with tools/pure/arena_usage.py.
//
// Approximations: the access width is unknown (4 bytes are marked); a second access of one
// instruction to the same page is not seen; accesses by the kernel (read(2) into the arena) are
// handled by opening the pages for the call (`read` is interposed and marks the bytes written).

#define _GNU_SOURCE
#include <dlfcn.h>
#include <fcntl.h>
#include <signal.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>
#include <sys/syscall.h>
#include <ucontext.h>
#include <unistd.h>

#define PAGE 4096u
#define LO 0x00401000u
#define HI 0x01577000u
#define NPAGES ((HI - LO) / PAGE)

// watched ranges and their protection once open
static const struct { uint32_t lo, hi; int prot; } RANGES[] = {
    {0x00401000u, 0x00858000u, PROT_READ},              // game .text + _rwcseg (read-only data)
    {0x00858000u, 0x008A4000u, PROT_READ},              // .rdata
    {0x008A4000u, 0x008E4000u, PROT_READ | PROT_WRITE}, // .data (initialised part)
    {0x00C9E000u, 0x00CB1000u, PROT_READ | PROT_WRITE}, // _TEXT_HA, _rwdseg, .rsrc
    {0x01556000u, 0x01577000u, PROT_READ},              // .HOODLUM (read-only data)
};

static uint8_t state[HI - LO];         // 0 untouched, 1 read first, 2 write first
static uint32_t faults[NPAGES];
static uint32_t since_new[NPAGES];
static uint8_t open_for_good[NPAGES];
static int8_t page_prot[NPAGES];      // -1 not watched
static volatile int armed;
static uint32_t stable = 100000; // SA_TRACE_STABLE
static __thread uint32_t pending;     // page index + 1 being single-stepped by this thread
static struct sigaction app_segv, app_bus, app_trap;
static int (*real_sigaction)(int, const struct sigaction *, struct sigaction *);
static int (*real_mprotect)(void *, size_t, int);
static ssize_t (*real_read)(int, void *, size_t);

static int sys_mprotect(uint32_t a, uint32_t n, int prot) {
    return syscall(SYS_mprotect, (void *)(uintptr_t)a, (size_t)n, prot);
}

static void mark(uint32_t a, uint32_t n, uint8_t how) {
    for (uint32_t i = 0; i < n; i++) {
        uint32_t x = a + i;
        if (x >= LO && x < HI && state[x - LO] == 0) {
            state[x - LO] = how;
            since_new[(x - LO) / PAGE] = 0;
        }
    }
}

static void chain(struct sigaction *app, int sig, siginfo_t *si, void *uc) {
    if (app->sa_flags & SA_SIGINFO) {
        if (app->sa_sigaction) {
            app->sa_sigaction(sig, si, uc);
            return;
        }
    } else if (app->sa_handler != SIG_DFL && app->sa_handler != SIG_IGN) {
        app->sa_handler(sig);
        return;
    }
    signal(sig, SIG_DFL);
    raise(sig);
}

static void on_segv(int sig, siginfo_t *si, void *ucv) {
    ucontext_t *uc = ucv;
    uint32_t a = (uint32_t)(uintptr_t)si->si_addr;
    if (armed && a >= LO && a < HI) {
        uint32_t p = (a - LO) / PAGE;
        if (page_prot[p] >= 0) {
            int write = (uc->uc_mcontext.gregs[REG_ERR] & 2) != 0;
            if (write && !(page_prot[p] & PROT_WRITE)) {
                chain(sig == SIGBUS ? &app_bus : &app_segv, sig, si, ucv);
                return;
            }
            mark(a, 4, write ? 2 : 1);
            faults[p]++;
            since_new[p]++;
            sys_mprotect(LO + p * PAGE, PAGE, page_prot[p]);
            if (faults[p] > stable && since_new[p] > stable) {
                open_for_good[p] = 1;
            } else {
                pending = p + 1;
                uc->uc_mcontext.gregs[REG_EFL] |= 0x100;
            }
            return;
        }
    }
    chain(sig == SIGBUS ? &app_bus : &app_segv, sig, si, ucv);
}

static void on_trap(int sig, siginfo_t *si, void *ucv) {
    ucontext_t *uc = ucv;
    if (pending) {
        uint32_t p = pending - 1;
        pending = 0;
        uc->uc_mcontext.gregs[REG_EFL] &= ~0x100;
        if (!open_for_good[p])
            sys_mprotect(LO + p * PAGE, PAGE, PROT_NONE);
        return;
    }
    chain(&app_trap, sig, si, ucv);
}

static void dump(void) {
    const char *out = getenv("SA_TRACE_OUT");
    if (!out || !armed)
        return;
    armed = 0;
    char path[512];
    snprintf(path, sizeof path, "%s.%d", out, (int)getpid());
    int fd = open(path, O_WRONLY | O_CREAT | O_TRUNC, 0644);
    if (fd < 0)
        return;
    // header: magic, LO, HI, NPAGES; then state[HI-LO], faults[NPAGES] (u32), open_for_good[NPAGES]
    uint32_t h[4] = {0x31525450u, LO, HI, NPAGES};
    write(fd, h, sizeof h);
    write(fd, state, sizeof state);
    write(fd, faults, sizeof faults);
    write(fd, open_for_good, sizeof open_for_good);
    close(fd);
}

static void arm(void) {
    struct sigaction sa;
    memset(&sa, 0, sizeof sa);
    sa.sa_sigaction = on_segv;
    sa.sa_flags = SA_SIGINFO | SA_NODEFER;
    sigemptyset(&sa.sa_mask);
    real_sigaction(SIGSEGV, &sa, &app_segv);
    real_sigaction(SIGBUS, &sa, &app_bus);
    sa.sa_sigaction = on_trap;
    sa.sa_flags = SA_SIGINFO;
    real_sigaction(SIGTRAP, &sa, &app_trap);
    for (uint32_t p = 0; p < NPAGES; p++)
        if (page_prot[p] >= 0)
            sys_mprotect(LO + p * PAGE, PAGE, PROT_NONE);
    armed = 1;
    atexit(dump);
}

__attribute__((constructor)) static void init(void) {
    real_sigaction = dlsym(RTLD_NEXT, "sigaction");
    real_mprotect = dlsym(RTLD_NEXT, "mprotect");
    real_read = dlsym(RTLD_NEXT, "read");
    if (getenv("SA_TRACE_STABLE"))
        stable = (uint32_t)strtoul(getenv("SA_TRACE_STABLE"), 0, 0);
    memset(page_prot, -1, sizeof page_prot);
    for (size_t r = 0; r < sizeof RANGES / sizeof RANGES[0]; r++)
        for (uint32_t a = RANGES[r].lo; a < RANGES[r].hi; a += PAGE)
            page_prot[(a - LO) / PAGE] = (int8_t)RANGES[r].prot;
}

// SA_IMAGE_KEEP=FILE (instead of tracing): every byte of the watched ranges outside the ranges listed in FILE
// (lines "START END", hex, END exclusive) is zeroed once the arena is filled, as an image built only from those
// objects would leave it. A run whose report matches the full image's shows that the kept set suffices for it.
static void keep_only(const char *path) {
    static uint8_t keep[HI - LO];
    FILE *f = fopen(path, "r");
    if (!f)
        return;
    unsigned s, e;
    while (fscanf(f, "%x %x", &s, &e) == 2)
        for (uint32_t a = s; a < e && a < HI; a++)
            if (a >= LO)
                keep[a - LO] = 1;
    fclose(f);
    for (size_t r = 0; r < sizeof RANGES / sizeof RANGES[0]; r++) {
        sys_mprotect(RANGES[r].lo, RANGES[r].hi - RANGES[r].lo, PROT_READ | PROT_WRITE);
        for (uint32_t a = RANGES[r].lo; a < RANGES[r].hi; a++)
            if (!keep[a - LO])
                *(volatile uint8_t *)(uintptr_t)a = 0;
        sys_mprotect(RANGES[r].lo, RANGES[r].hi - RANGES[r].lo, RANGES[r].prot);
    }
}

int mprotect(void *addr, size_t len, int prot) {
    uint32_t a = (uint32_t)(uintptr_t)addr;
    if (!armed && a == 0x00858000u && len == 0x4C000u && prot == PROT_READ && getenv("SA_IMAGE_KEEP")) {
        int r = real_mprotect(addr, len, prot);
        keep_only(getenv("SA_IMAGE_KEEP"));
        return r;
    }
    if (armed && a >= LO && a < HI) {
        // a later change of a watched page (samp's image patches): remember it, keep the page closed
        for (uint32_t x = a & ~(PAGE - 1); x < a + len && x < HI; x += PAGE)
            if (page_prot[(x - LO) / PAGE] >= 0)
                page_prot[(x - LO) / PAGE] = (int8_t)prot;
        return 0;
    }
    int r = real_mprotect(addr, len, prot);
    if (!armed && a == 0x00858000u && len == 0x4C000u && prot == PROT_READ && getenv("SA_TRACE_OUT"))
        arm();
    return r;
}

int sigaction(int sig, const struct sigaction *act, struct sigaction *old) {
    if (armed && act && (sig == SIGSEGV || sig == SIGBUS || sig == SIGTRAP)) {
        struct sigaction *keep = sig == SIGSEGV ? &app_segv : sig == SIGBUS ? &app_bus : &app_trap;
        if (old)
            *old = *keep;
        *keep = *act;
        return 0;
    }
    return real_sigaction(sig, act, old);
}

ssize_t read(int fd, void *buf, size_t n) {
    uint32_t a = (uint32_t)(uintptr_t)buf;
    if (armed && n && a < HI && a + n > LO) {
        for (uint32_t x = (a < LO ? LO : a) & ~(PAGE - 1); x < a + n && x < HI; x += PAGE) {
            uint32_t p = (x - LO) / PAGE;
            if (page_prot[p] >= 0) {
                open_for_good[p] = 1;
                sys_mprotect(x, PAGE, page_prot[p]);
            }
        }
        mark(a, (uint32_t)n, 2);
    }
    return real_read(fd, buf, n);
}
