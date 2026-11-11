# CPointRoute (AI) - RE-SPEC-FINAL (helper B)

## 0x63A2E0 CPointRoute::sub_63A2E0 (proposed: CTaskUtilityLineUpPedWithCar scalar deleting destructor)  (catalogue: unknown/REA, port level 1)
- Name correction: not a CPointRoute member - calls `CTaskUtilityLineUpPedWithCar::Destructor` 0x64FC00 and `_free` 0x8214BD (heap delete), i.e. CTaskUtilityLineUpPedWithCar's scalar deleting destructor.
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor CTaskUtilityLineUpPedWithCar::Destructor 0x64FC00 (0x63A2E3); if `flags & 1` (0x63A2E8) calls _free 0x8214BD (this) (0x63A2F0, cdecl, caller pops); returns this in EAX.
