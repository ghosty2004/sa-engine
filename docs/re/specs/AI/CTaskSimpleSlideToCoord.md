# CTaskSimpleSlideToCoord (AI) - RE-SPEC-AI-2 group J3

## 0x66C440 CTaskSimpleSlideToCoord::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x3A6` (0x66C440); `ret` -> returns 934 (TASK_SIMPLE_SLIDE_TO_COORD). Vtable: slot 4 (+0x10) of vtable 0x86FFEC (CTaskSimpleSlideToCoord).

## 0x6712C0 CTaskSimpleSlideToCoord::~CTaskSimpleSlideToCoord  (catalogue: specified/REA, port level 3)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor FUN_0066c4c0 0x66C4C0 (0x6712C3); if `flags & 1` (0x6712C8) calls CTask::operator delete 0x61A5B0 (this) (0x6712D0, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x86FFEC (CTaskSimpleSlideToCoord).

## 0x66D300 CTaskSimpleSlideToCoord::Clone  (catalogue: specified/REA, port level 4)
- Convention: virtual slot 1, `__thiscall CTask* (this = ECX)`, plain `ret`; SEH frame. Branch on this+0x78 & 2 (0x66D319, `je 0x66D37F` when clear), each path calls CTask::operator new(0x80) 0x61A5A0 and returns NULL if it fails: set -> p = new(0x80); p->CTaskSimpleSlideToCoord(&this->+0x64 (CVector*), this+0x70, this+0x74, &this->+0x10 (anim name), &this->+0x28 (anim block), this+0x5C, this+0x38, (this+0xC & 0x10) >> 4, this+0x40) 0x66C450 (anim variant); clear -> p = new(0x80); p->CTaskSimpleSlideToCoord(&this->+0x64, this+0x70, this+0x74) 0x66C3E0; return p.
- gta-reversed: not hooked at this address (catalogue source REA); described from the bytes.
