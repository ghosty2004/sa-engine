# CTaskComplexProstituteSolicit (AI) - RE-SPEC-AI-2 group J3

## 0x661AE0 CTaskComplexProstituteSolicit::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x51D` (0x661AE0); `ret` -> returns 1309 (TASK_COMPLEX_PROSTITUTE_SOLICIT). Vtable: slot 4 (+0x10) of vtable 0x86FB88.

## 0x664AA0 CTaskComplexProstituteSolicit::~CTaskComplexProstituteSolicit  (catalogue: specified/REA, port level 3)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor CTaskComplexProstituteSolicit::~CTaskComplexProstituteSolicit 0x661AF0 (0x664AA3); if `flags & 1` (0x664AA8) calls CTask::operator delete 0x61A5B0 (this) (0x664AB0, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x86FB88.

## 0x661AF0 CTaskComplexProstituteSolicit::~CTaskComplexProstituteSolicit  (catalogue: specified/REA, port level 2)
- `__thiscall void ()`, plain `ret`, SEH frame. vtbl = 0x86FB88. p = CWorld::FindPlayerPed(-1) 0x56E210; if p != NULL:
  pd = p->m_pPlayerData (+0x480); if pd+0xA4 (gr m_pPlayerProstitute? entity ref) != NULL: CleanUpOldReference(&pd->+0xA4) 0x571A00;
  pd+0xA4 = 0; then if (u16)this+0x2C & 0x100: this+0x2C &= 0xFEFF (word store). Finally CTaskComplex::~CTaskComplex 0x61A3D0.
  (The flag clear only happens when the player ped exists.)

## 0x661B80 CTaskComplexProstituteSolicit::MakeAbortable  (catalogue: specified/REA, port level 0)
- `__thiscall bool (this, CPed* ped, int32 priority, const CEvent* event)`, `ret 0xC`. r = m_pSubTask (+8)->MakeAbortable(ped,
  priority, event) (vtable +0x18); if r (AL) != 0: byte this+0x2D &= 0xFE (clears bit 8 of the +0x2C word). Returns r (AL).

## 0x661D30 CTaskComplexProstituteSolicit::GetRidOfPlayerProstitute  (catalogue: specified/REA, port level 2)
- `__cdecl void ()` (static). p = CWorld::FindPlayerPed(-1) 0x56E210 (no NULL check); e = p->m_pPlayerData (+0x480)->+0xA4 (player's
  prostitute ped); if e != NULL: t = e->m_pIntelligence (+0x47C)->FindTaskByType(0x51D = 1309, TASK_COMPLEX_PROSTITUTE_SOLICIT)
  0x600EE0; if t != NULL: byte t+0x2C |= 4; word t+0x2A = 0.

## 0x6622F0 CTaskComplexProstituteSolicit::Clone  (catalogue: specified/REA, port level 3)
- Convention: virtual slot 1, `__thiscall CTask* (this = ECX)`, plain `ret`; SEH frame. p = CTask::operator new(0x30) 0x61A5A0 (cdecl); if p == NULL -> return NULL (0 in EAX). Else p->CTaskComplexProstituteSolicit::CTaskComplexProstituteSolicit(this+0xC) 0x661A60; return p.
- gta-reversed: not hooked at this address (catalogue source REA); described from the bytes.
