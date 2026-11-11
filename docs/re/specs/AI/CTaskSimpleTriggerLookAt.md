# CTaskSimpleTriggerLookAt (AI) - RE-SPEC-FINAL (helper B)

## 0x634440 CTaskSimpleTriggerLookAt::CTaskSimpleTriggerLookAt  (catalogue: unknown/REA, port level 2)
- `__thiscall T* (this, CEntity* entity [esp+4], int32 time [esp+8], int32 offsetBoneTag [esp+0xC], CVector offsetPos (by value, 3 dwords [esp+0x10..0x18]), uint8 useTorso [esp+0x1C], float speed [esp+0x20], int32 blendTime [esp+0x24], int8 priority [esp+0x28])`, `ret 0x28`; SEH frame via .HOODLUM stub. Callers: script switches 1400-1499 0x48EAA0 (2), 1600-1699 0x493FE0, 1700-1799 0x496E00; Clone 0x634560.
- Behaviour: CTaskSimple ctor 0x61A390; `m_time (+0xC) = time; m_nOffsetBoneTag (+0x10) = offsetBoneTag; m_vOffsetPos (+0x14..+0x1C) = offsetPos; m_bUseTorso (+0x20) = useTorso; m_Speed (+0x24) = speed; vtbl = 0x86E3CC; m_pEntity (+8) = entity; m_nBlendTime (+0x28) = blendTime; m_nPriority (+0x2D) = priority`; `if entity` -> RegisterReference(entity, &+8) 0x571B70 and `m_bEntityExist (+0x2C) = 1`; else `+0x2C = 0`; return this.

## 0x634500 sub_634500 (proposed: CTaskSimpleTriggerLookAt::~CTaskSimpleTriggerLookAt (non-deleting))  (catalogue: unknown/REA, port level 1)
- Name proposal: complete dtor of CTaskSimpleTriggerLookAt. Evidence: stores vtable 0x86E3CC (TASK_SIMPLE_TRIGGER_LOOK_AT); caller 0x6394D0 = slot 0 (scalar deleting dtor) of 0x86E3CC.
- `__thiscall void (this)`, plain `ret`; SEH frame (handler 0x840878).
- Behaviour: `vtbl = 0x86E3CC`; `if m_pEntity (+8)` -> CleanUpOldReference(m_pEntity, &this->+8) 0x571A00 (0x634536); CTaskSimple::~CTaskSimple 0x61A3A0.
