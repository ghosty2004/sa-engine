# Source of the Unicorn exit stub (`EXIT_STUB` in src/emu.rs). The emulated function returns into this
# code, which captures the outputs with the same instructions as native/thunk.S, so both backends observe
# the result through identical x87 operations. Result block at 0x11000, x87 environment at 0x11100 (see
# the layout constants in src/emu.rs). Assemble: `as --32 exit_stub.S -o x.o && objcopy -O binary -j .text`.
.intel_syntax noprefix
.text
    mov ds:[0x11000 + 88], eax
    mov ds:[0x11000 + 92], edx
    fnstsw ds:[0x11000 + 100]
    stmxcsr ds:[0x11000 + 120]
    fnstenv ds:[0x11100]
    mov ax, ds:[0x11100 + 8]
    mov ds:[0x11000 + 102], ax
    fldenv ds:[0x11100]
    mov dword ptr ds:[0x11000 + 124], 0
    fxam
    fnstsw ax
    and ah, 0x45
    cmp ah, 0x41
    je 3f
    fst dword ptr ds:[0x11000 + 116]
    fstp tbyte ptr ds:[0x11000 + 104]
    mov dword ptr ds:[0x11000 + 124], 1
3:  nop
