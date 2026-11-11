# CTaskComplexGangJoinRespond (AI) - RE-SPEC-AI-2 group J2

## 0x661710 CTaskComplexGangJoinRespond::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x4C3` (0x661710); `ret` -> returns 1219 (TASK_COMPLEX_GANG_JOIN_RESPOND). Vtable: slot 4 (+0x10) of vtable 0x86FB5C (CTaskComplexGangJoinRespond).

## 0x661790 CTaskComplexGangJoinRespond::MakeAbortable  (catalogue: specified/REA, port level 0)
- `__thiscall bool`: returns true (1) in AL only (`mov al, 1` 0x661790; upper EAX bits undefined), `ret 0xc` (12 bytes of stack args, ignored). Vtable: slot 6 (+0x18) of vtable 0x86FB5C (CTaskComplexGangJoinRespond).

## 0x664A80 CTaskComplexGangJoinRespond::~CTaskComplexGangJoinRespond  (catalogue: specified/REA, port level 3)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor CTaskComplexGangJoinRespond::Destructor 0x661720 (0x664A83); if `flags & 1` (0x664A88) calls CTask::operator delete 0x61A5B0 (this) (0x664A90, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x86FB5C (CTaskComplexGangJoinRespond).

## 0x662290 CTaskComplexGangJoinRespond::Clone  (catalogue: specified/REA, port level 2)
- Convention: virtual slot 1, `__thiscall CTask* (this = ECX)`, plain `ret`; SEH frame. p = CTask::operator new(0x14) 0x61A5A0 (cdecl); if p == NULL -> return NULL (0 in EAX). Else p->CTaskComplex::CTaskComplex() 0x61A3B0; p+0xC = this+0xC (u8); vtbl = 0x86FB5C (CTaskComplexGangJoinRespond); p+0xD = 0 (u8); return p.
- gta-reversed: not hooked at this address (catalogue source REA); described from the bytes.

<!-- RE-SPEC-FINAL helper B additions -->

## 0x6617A0 CTaskComplexGangJoinRespond::CreateNextSubTask  (catalogue: specified/REA, port level 2)
- `__thiscall CTask* (this, CPed* ped unused)`, `ret 4`; SEH frame. Fields: +0xC (u8) accept, +0xD (u8) "gangs" anim block referenced (set by ControlSubTask 0x661950), +0x10 wait counter.
- Behaviour: if `m_pSubTask->GetTaskType() == 0x190` (TASK_SIMPLE_ANIM) -> 0. If `+0xD != 0` (anims available): `p = new(0x20)`; NULL -> 0; return `CTaskSimpleRunAnim::CTaskSimpleRunAnim(p, 0x34 (group), (+0xC == 1) ? 0x132 : 0x133 (anim), 4.0f, false)` 0x61A8B0. Else `+0x10 += 1`; if `(int32)+0x10 > 10` -> 0; else `p = new(0x20)`; return `CTaskSimpleStandStill(p, 250, false, false, 8.0f)` 0x62F310 (0 on allocation failure).

## 0x6618D0 CTaskComplexGangJoinRespond::CreateFirstSubTask  (catalogue: specified/REA, port level 3)
- `__thiscall CTask* (this, CPed* ped unused)`, `ret 4`; SEH frame.
- Behaviour: `this->+0x10 = 0` (0x6618E9, before the allocation); `p = CTask::operator new(0x28)`; if NULL return 0. Else `CTaskComplexTurnToFaceEntityOrCoord::CTaskComplexTurnToFaceEntityOrCoord(p, CWorld::FindPlayerPed(0) 0x56E210, [0x86FC7C]=0.5f, [0x86FC80]=0.2f)` 0x66B890 (the entity overload: first arg is the player CPed*; floats pushed as raw dwords) (0x661924); return p.

## 0x661950 CTaskComplexGangJoinRespond::ControlSubTask  (catalogue: specified/REA, port level 2)
- `__thiscall CTask* (this, CPed* ped [esp+4])`, `ret 4`; always returns `m_pSubTask` (+8). Field +0xD (u8) = "gangs" anim block referenced.
- Behaviour:
  1. Face the player: `P = ped position`, `Q = CWorld::FindPlayerPed(-1)` position (matrix pos or placement); local `v = (Q.x - P.x, Q.y - P.y, 0.0)` (fsub/fstp f32, z stored as dword 0); `CVector::Normalise(&v)` 0x59C910; `ped->+0x55C = CGeneral::LimitRadianAngle(GetRadianAngleBetweenPoints(v.x, v.y, 0, 0))` (fstp f32).
  2. If `+0xD == 0`: if `CTaskComplexGangLeader::ShouldLoadGangAnims()` 0x65E7F0: `idx = CAnimManager::GetAnimationBlockIndex("gangs" [0x86F92C])` 0x4D3990; if `ms_aAnimBlocks[idx].m_bLoaded` (u8 at 0xB5D4B0 + idx*0x20) != 1 -> `CStreaming::RequestModel(idx + 0x63E7, 8)` 0x4087E0; else `CAnimManager::AddAnimBlockRef(idx)` 0x4D3FB0 and `+0xD = 1`.
     Else (`+0xD != 0`): if `!ShouldLoadGangAnims()` -> `CAnimManager::RemoveAnimBlockRef(GetAnimationBlockIndex("gangs"))` 0x4D3FD0 and `+0xD = 0`.
