# CTaskSimpleJetPack (AI) - RE-SPEC-AI-2 group J7

## 0x6801D0 CTaskSimpleJetPack::~CTaskSimpleJetPack  (catalogue: specified/REA, port level 12)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor CTaskSimpleJetPack::Destructor 0x67E600 (0x6801D3); if `flags & 1` (0x6801D8) calls CTask::operator delete 0x61A5B0 (this) (0x6801E0, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x8705C4 (CTaskSimpleJetPack).

<!-- RE-SPEC-FINAL helper B additions -->

## 0x67B660 CTaskSimpleJetPack::DropJetPack  (catalogue: specified/REA, port level 18)
- `__thiscall void (this, CPed* ped [esp+4])`, `ret 4`. Callers: CPlayerInfo::Process 0x56F8D0, CTaskSimpleJetPack::ProcessControlInput 0x67E7B0. Fields: +8 (u8, finished / already dropped), +0xD, +0xE (u8), +0x10 (dword).
- Behaviour: `if this->+8 == 0`: `P = ped position` (matrix pos or placement, the selection is evaluated separately for x, y and z); `CPickups::CreatePickupCoorsCloseToCoors(P.x, P.y, P.z, &out.x, &out.y, &out.z)` 0x458A80 (cdecl, floats as raw dwords); `CPickups::GenerateNewOne(out (CVector by value), 0x172 (model 370, jetpack), 5 (pickup type), 0 (ammo), 0 (moneyPerDay), 1 (isEmpty), NULL)` 0x456F20 (cdecl, result ignored). Then always: `this->MakeAbortable(ped, 1, NULL)` (vslot +0x18 = 0x67E6E0); `+0xD = 0; +0xE = 0; +0x10 = 0`.
