/* Native reference for the L1 oracle: runs original gta_sa.exe 1.0 US code on the real CPU (under Wine,
 * i686) with the same inputs as the Unicorn runner. Built and driven by src/native.rs.
 *
 *   ref.exe IMAGE PROBES CASES OUT
 *
 * IMAGE   flat image (SizeOfImage bytes, sections laid out, IAT slots pointing at the trap page); everything
 *         after the header page is copied to 0x401000 (the exe's preferred base is 0x400000).
 * PROBES  code blob loaded at 0x10100000 (x87 fidelity probes), or "-".
 * CASES   N records of { u32 len, ctx (128 bytes, see thunk.S), scratch (len bytes at 0x10000000,
 *         len <= SCRATCH_MAX) }, or "-" for stdin.
 * OUT     the same records with the outputs filled in, or "-" for stdout.
 *
 * Wine puts its heaps at 0x400000 when the main exe lives elsewhere, so ref.exe itself is linked at
 * 0x400000 with `gta_space` as its first section (linker script made by src/native.rs): the PE loader
 * reserves the game's whole address range for us, and our own code and data follow it.
 *
 * No guard rails here: only calls that already passed the Unicorn guard (allow-list, budget) are sent.
 */
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <fcntl.h>
#include <io.h>
#include <windows.h>

#define IMAGE_BASE 0x400000u
#define SCRATCH 0x10000000u
#define SCRATCH_MAX 0x8000u
#define PROBE 0x10100000u
#define PROBE_LEN 0x10000u
#define CTX_LEN 128u
/* SizeOfImage of gta_sa.exe 1.0 US minus the header page */
#define GTA_SPACE 0x1176000u

__attribute__((section(".gta"))) char gta_space[GTA_SPACE];

void sa_call(void *ctx);

static void *map_at(uint32_t addr, uint32_t len) {
    void *p = VirtualAlloc((void *)(uintptr_t)addr, len, MEM_RESERVE | MEM_COMMIT, PAGE_EXECUTE_READWRITE);
    if (p != (void *)(uintptr_t)addr) {
        DWORD err = GetLastError();
        fprintf(stderr, "ref: cannot map %#x+%#x (got %p, error %lu)\n", addr, len, p, err);
        for (uint32_t a = addr; a < addr + len;) {
            MEMORY_BASIC_INFORMATION mi;
            if (!VirtualQuery((void *)(uintptr_t)a, &mi, sizeof mi)) break;
            fprintf(stderr, "  %p+%#lx state %#lx type %#lx\n", mi.BaseAddress, (unsigned long)mi.RegionSize,
                    mi.State, mi.Type);
            a = (uint32_t)(uintptr_t)mi.BaseAddress + (uint32_t)mi.RegionSize;
        }
        exit(2);
    }
    return p;
}

/* Commits the stack pages below main's frame once, so the thunk can fill the stack below the
 * arguments without stepping over the guard page. */
static void __attribute__((noinline)) commit_stack(void) {
    volatile char buf[0x10000];
    for (unsigned i = 0; i < sizeof buf; i += 256) buf[i] = 0;
}

int main(int argc, char **argv) {
    commit_stack();
    if (argc != 5) {
        fprintf(stderr, "usage: ref IMAGE PROBES CASES OUT\n");
        return 2;
    }
    FILE *fi = fopen(argv[1], "rb");
    if (!fi) {
        fprintf(stderr, "ref: cannot open %s\n", argv[1]);
        return 2;
    }
    fseek(fi, 0, SEEK_END);
    long ilen = ftell(fi);
    if ((uintptr_t)gta_space != IMAGE_BASE + 0x1000 || ilen != (long)(GTA_SPACE + 0x1000)) {
        fprintf(stderr, "ref: gta_space at %p, image %ld bytes: link layout wrong\n", (void *)gta_space, ilen);
        return 2;
    }
    DWORD old;
    if (!VirtualProtect(gta_space, GTA_SPACE, PAGE_EXECUTE_READWRITE, &old)) {
        fprintf(stderr, "ref: VirtualProtect failed (%lu)\n", GetLastError());
        return 2;
    }
    fseek(fi, 0x1000, SEEK_SET);
    if (fread(gta_space, 1, GTA_SPACE, fi) != GTA_SPACE) {
        fprintf(stderr, "ref: short image\n");
        return 2;
    }
    fclose(fi);
    map_at(SCRATCH, 0x10000);
    void *probe = map_at(PROBE, PROBE_LEN);
    if (strcmp(argv[2], "-") != 0) {
        FILE *fp = fopen(argv[2], "rb");
        if (!fp) {
            fprintf(stderr, "ref: cannot open %s\n", argv[2]);
            return 2;
        }
        fread(probe, 1, PROBE_LEN, fp);
        fclose(fp);
    }

    _setmode(_fileno(stdin), _O_BINARY);
    _setmode(_fileno(stdout), _O_BINARY);
    FILE *in = strcmp(argv[3], "-") ? fopen(argv[3], "rb") : stdin;
    FILE *out = strcmp(argv[4], "-") ? fopen(argv[4], "wb") : stdout;
    if (!in || !out) {
        fprintf(stderr, "ref: cannot open cases/out\n");
        return 2;
    }
    setvbuf(in, NULL, _IOFBF, 1 << 20);
    setvbuf(out, NULL, _IOFBF, 1 << 20);
    static uint8_t rec[4 + CTX_LEN + SCRATCH_MAX];
    unsigned long n = 0;
    uint32_t len;
    while (fread(&len, 1, 4, in) == 4) {
        if (len > SCRATCH_MAX || fread(rec + 4, 1, CTX_LEN + len, in) != CTX_LEN + len) {
            fprintf(stderr, "ref: bad record %lu\n", n);
            return 2;
        }
        memcpy(rec, &len, 4);
        memcpy((void *)(uintptr_t)SCRATCH, rec + 4 + CTX_LEN, len);
        sa_call(rec + 4);
        memcpy(rec + 4 + CTX_LEN, (void *)(uintptr_t)SCRATCH, len);
        fwrite(rec, 1, 4 + CTX_LEN + len, out);
        n++;
    }
    fflush(out);
    fprintf(stderr, "ref: %lu cases\n", n);
    return 0;
}
