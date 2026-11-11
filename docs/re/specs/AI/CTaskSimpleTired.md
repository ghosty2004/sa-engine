# CTaskSimpleTired (AI) - RE-SPEC-AI-2 group J4

## 0x630F50 CTaskSimpleTired::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0xDB` (0x630F50); `ret` -> returns 219 (TASK_SIMPLE_TIRED). Vtable: slot 4 (+0x10) of vtable 0x86DED0 (CTaskSimpleTired).

## 0x637950 CTaskSimpleTired::~CTaskSimpleTired  (catalogue: specified/REA, port level 2)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor CTaskSimpleTired::Destructor 0x630F60 (0x637953); if `flags & 1` (0x637958) calls CTask::operator delete 0x61A5B0 (this) (0x637960, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x86DED0 (CTaskSimpleTired).

<!-- RE-SPEC-FINAL helper B additions -->

## 0x630FF0 CTaskSimpleTired::MakeAbortable  (catalogue: specified/REA, port level 1)
- Name note: called directly from CTaskSimpleTired::ProcessPed 0x637970 (not via the vtable in our corpus); signature is MakeAbortable's.
- `__thiscall bool (this, CPed* ped [esp+4], int32 priority, const CEvent* event)`, `ret 0xC`; always returns AL = 1.
- Behaviour: `a = this->+0x18 (m_pAnim)`; if a == NULL -> return 1. `b = RpAnimBlendClumpGetAssociation(ped->m_pRwClump (+0x18), 0xA)` 0x4D68B0 (cdecl, anim id 10); `if a == b` -> `a->m_BlendDelta (+0x1C) = -4.0f` (0x631012). Return 1 either way.

## 0x636180 CTaskSimpleTired::Clone  (catalogue: specified/REA, port level 2)
- `__thiscall CTask* (this)`, plain `ret`; SEH frame.
- Behaviour: `p = CTask::operator new(0x1C)`; if NULL return 0. Else `t = this->+8 (m_tiredDurationMs)`; CTaskSimple ctor 0x61A390; `p+8 = t; p->vtbl = 0x86DED0; p+0xC = 0; p+0x10 = 0; p+0x14 = 0 (u8); p+0x15 = 0 (u8); p+0x18 = 0 (m_pAnim)`; return p.

## 0x637970 CTaskSimpleTired::ProcessPed  (catalogue: specified/REA, port level 10)
- `__thiscall bool (this, CPed* ped [esp+4])`, `ret 4`. Fields: +8 duration, +0xC/+0x10/+0x14/+0x15 CTaskTimer {start, interval, started, reset}, +0x18 m_pAnim.
- Behaviour: `if m_pAnim == NULL` -> `CTaskSimpleTired::StartAnim(this, ped)` 0x630F70. `if !timer.started` -> return 0. `if timer.reset` -> `start = now ([0xB7CB84]); reset = 0`. `if (u32)now >= (u32)(start + interval)` (`cmp; sbb; inc`, 0x6379A5-0x6379B0): `this->MakeAbortable(ped, 0, NULL)` (vslot +0x18, 0x6379BF; = 0x630FF0) and return 1. Else return 0.
