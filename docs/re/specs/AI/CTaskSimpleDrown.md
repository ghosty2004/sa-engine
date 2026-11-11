# CTaskSimpleDrown (AI) - RE-SPEC-AI-2 group J3

## 0x62FF00 CTaskSimpleDrown::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0xD5` (0x62FF00); `ret` -> returns 213 (TASK_SIMPLE_DROWN). Vtable: slot 4 (+0x10) of vtable 0x86DE30 (CTaskSimpleDrown).

## 0x6378D0 CTaskSimpleDrown::~CTaskSimpleDrown  (catalogue: specified/REA, port level 3)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor FUN_0062ff10 0x62FF10 (0x6378D3); if `flags & 1` (0x6378D8) calls CTask::operator delete 0x61A5B0 (this) (0x6378E0, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x86DE30 (CTaskSimpleDrown).

## 0x62FEC0 CTaskSimpleDrown::CTaskSimpleDrown  (catalogue: specified/REA, port level 1)
- `__thiscall this ()`, plain `ret`. CTaskSimple::CTaskSimple() 0x61A390; +8 = 0; +0x10 = 0; +0x14 = 0; +0x1C = 0; +0x24 = 0;
  +0xC = 0x8C (anim id 140); +0x18 = 4.0f; byte +0x20 &= 0xFC; vtbl = 0x86DE30; byte +0x28 = 1. Returns this.

## 0x635E60 CTaskSimpleDrown::Clone  (catalogue: specified/REA, port level 2)
- Convention: virtual slot 1, `__thiscall CTask* (this = ECX)`, plain `ret`; SEH frame. p = CTask::operator new(0x2C) 0x61A5A0 (cdecl); if p == NULL -> return NULL (0 in EAX). Else p->CTaskSimple::CTaskSimple() 0x61A390; inline CTaskSimpleDie-style init: p+8 = 0; p+0x10 = 0; p+0x14 = 0; p+0x1C = 0; p+0x24 = 0; p+0x20 (u8) &= 0xFC (bits 0,1 cleared, other bits keep the uninitialised heap byte); p+0xC = 0x8C (anim id); p+0x18 = 4.0f (imm 0x40800000); vtbl = 0x86DE30 (CTaskSimpleDrown); p+0x28 = 1 (u8). No field is copied from this; return p.
- gta-reversed: not hooked at this address (catalogue source REA); described from the bytes.
