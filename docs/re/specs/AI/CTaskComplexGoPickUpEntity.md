# CTaskComplexGoPickUpEntity (AI) - RE-SPEC-FINAL (helper B)

## 0x6919C0 sub_6919c0 (proposed: CTaskComplexGoPickUpEntity::CTaskComplexGoPickUpEntity)  (catalogue: unknown/REA, port level 2)
- Evidence: vtable 0x870B98 (TASK_COMPLEX_GO_PICKUP_ENTITY); callers CRunningScript::ProcessCommands1900To1999 0x46B460, CPlayerInfo::Process 0x56F8D0, Clone 0x692C80.
- `__thiscall T* (this, CEntity* entity [esp+4], int32 animGroup [esp+8])`, `ret 8`; SEH frame.
- Behaviour: CTaskComplex ctor 0x61A3B0; `+0xC = entity; vtbl = 0x870B98; +0x10 = -999.0f (0xC479C000); +0x14 = 0; +0x18 = 0; +0x1C = -999.0f; +0x20 = 0; +0x24 = 0; +0x2C = animGroup`; `if entity` -> RegisterReference(entity, &this->+0xC) 0x571B70; `+0x28 = 0; +0x30 = 0 (u8, anim block referenced)`; return this.

## 0x691A50 sub_691a50 (proposed: CTaskComplexGoPickUpEntity::~CTaskComplexGoPickUpEntity (non-deleting))  (catalogue: unknown/REA, port level 2)
- Evidence: vtable 0x870B98; caller 0x6935F0 (scalar deleting dtor).
- `__thiscall void (this)`, plain `ret`; SEH frame.
- Behaviour: `vtbl = 0x870B98`; `if +0xC (entity)` -> CleanUpOldReference(entity, &this->+0xC) 0x571A00. `if +0x30 (u8)`: `blk = (ms_aAnimAssocGroups ([0xB4EA34]) [this->+0x2C] .m_pAnimBlock (dword at +0 of a 0x14-byte group) - 0xB5D4A0 (ms_aAnimBlocks)) >> 5` (block index, stride 0x20, signed `sar`); `CAnimManager::RemoveAnimBlockRef(blk)` 0x4D3FD0 (cdecl); `+0x30 = 0`. Then CTaskComplex::~CTaskComplex 0x61A3D0.
