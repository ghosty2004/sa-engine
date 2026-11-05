# Native-reference call thunk (i686, mingw). Calls an original gta_sa.exe function on the real CPU with
# the given register/stack inputs and x87/SSE control state, and captures the outputs exactly the way the
# Unicorn runner does (see src/native.rs for the shared `Ctx` layout; offsets below must match it).
#
#   void sa_call(struct ctx *c);   cdecl
#
# ctx: +0 fn, +4 ecx, +8 edx, +12 nstack, +16 stack[16], +80 cw (u16), +84 mxcsr,
#      +88 out eax, +92 out edx, +96 esp_delta, +100 fpsw (u16), +102 fptag (u16), +104 st0 (10 bytes),
#      +116 st0 stored as f32, +120 mxcsr after, +124 has_st0
.intel_syntax noprefix
.text
.globl _sa_call
_sa_call:
    push ebp
    push ebx
    push esi
    push edi
    mov esi, [esp + 20]
    mov [saved_ctx], esi
    fnstcw [host_cw]
    stmxcsr [host_mxcsr]
    fninit
    fldcw [esi + 80]
    ldmxcsr [esi + 84]
    mov [saved_esp], esp
    # fill STACK_FILL_LEN (0x800) bytes below the return address slot with STACK_FILL (as the Unicorn
    # backend does), so uninitialised stack reads are deterministic
    mov ecx, [esi + 12]
    lea edi, [ecx * 4 + 4]
    mov edx, esp
    sub edx, edi
    lea edi, [edx - 0x800]
    mov ecx, 0x200
    mov eax, 0x5A5A5A5A
    cld
    rep stosd
    mov ecx, [esi + 12]
1:  test ecx, ecx
    jz 2f
    push dword ptr [esi + 12 + ecx * 4]
    dec ecx
    jmp 1b
2:  mov eax, [esi + 0]
    mov ecx, [esi + 4]
    mov edx, [esi + 8]
    call eax
    mov esi, [saved_ctx]
    mov [esi + 88], eax
    mov [esi + 92], edx
    mov eax, esp
    sub eax, [saved_esp]
    mov [esi + 96], eax
    mov esp, [saved_esp]
    fnstsw [esi + 100]
    stmxcsr [esi + 120]
    fnstenv [env]
    mov ax, [env + 8]
    mov [esi + 102], ax
    fldenv [env]
    mov dword ptr [esi + 124], 0
    fxam
    fnstsw ax
    and ah, 0x45
    cmp ah, 0x41
    je 3f
    fst dword ptr [esi + 116]
    fstp tbyte ptr [esi + 104]
    mov dword ptr [esi + 124], 1
3:  fninit
    fldcw [host_cw]
    ldmxcsr [host_mxcsr]
    pop edi
    pop esi
    pop ebx
    pop ebp
    ret

.data
saved_ctx:  .long 0
saved_esp:  .long 0
host_cw:    .short 0
            .short 0
host_mxcsr: .long 0
env:        .space 28
