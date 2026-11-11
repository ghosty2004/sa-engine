# CTaskSimpleTogglePedThreatScanner (AI) - RE-SPEC-AI-2 group J3

## 0x492DD0 CTaskSimpleTogglePedThreatScanner::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x515` (0x492DD0); `ret` -> returns 1301 (TASK_SIMPLE_TOGGLE_PED_THREAT_SCANNER). Vtable: slot 4 (+0x10) of vtable 0x85A334.

## 0x492DE0 CTaskSimpleTogglePedThreatScanner::MakeAbortable  (catalogue: specified/REA, port level 0)
- `__thiscall bool`: returns false (0) in AL only (`xor al, al` 0x492DE0; upper EAX bits undefined), `ret 0xc` (12 bytes of stack args, ignored). Vtable: slot 6 (+0x18) of vtable 0x85A334.

## 0x492DF0 CTaskSimpleTogglePedThreatScanner::~CTaskSimpleTogglePedThreatScanner  (catalogue: specified/REA, port level 2)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor FUN_00492d40 0x492D40 (0x492DF3); if `flags & 1` (0x492DF8) calls CTask::operator delete 0x61A5B0 (this) (0x492E00, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x85A334.

## 0x492D50 CTaskSimpleTogglePedThreatScanner::Clone  (catalogue: specified/REA, port level 2)
- Convention: virtual slot 1, `__thiscall CTask* (this = ECX)`, plain `ret`; SEH frame. p = CTask::operator new(0xC) 0x61A5A0 (cdecl); if p == NULL -> return NULL (0 in EAX). Else reads b8 = this+8, b9 = this+9, bA = this+0xA (u8) first; p->CTaskSimple::CTaskSimple() 0x61A390; p+9 = b9; p+0xA = bA; vtbl = 0x85A334 (CTaskSimpleTogglePedThreatScanner); p+8 = b8; return p.
- gta-reversed: not hooked at this address (catalogue source REA); described from the bytes.
