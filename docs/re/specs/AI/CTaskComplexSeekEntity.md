# CTaskComplexSeekEntity (AI) - RE-SPEC-FINAL (helper B)

## 0x66CA40 CTaskComplexSeekEntity<CEntitySeekPosCalculatorFixedPos>::CTaskComplexSeekEntity  (catalogue: unknown/REA, port level 2)
- `__thiscall T* (this, CEntity* entity [esp+4], int32 seekInterval [esp+8], int32 scanInterval [esp+0xC], float maxEntityDist2D [esp+0x10], float moveStateRadius [esp+0x14], float followNodeThresholdHeightChange [esp+0x18], bool bPlayTiredAnim [esp+0x1C], bool bFaceEntityWhenDone [esp+0x20])`, `ret 0x20`; SEH frame. Callers: vt_87004c::Clone 0x671330, CTaskComplexInvestigateDisturbance::CreateNextSubTask 0x6749F0.
- Behaviour (gr Tasks/TaskTypes/SeekEntity/TaskComplexSeekEntity.h names): CTaskComplex ctor 0x61A3B0; `+0xC m_entityToSeek = entity; +0x10 m_seekInterval; +0x14 m_scanInterval; +0x18 m_maxEntityDist2D; +0x1C m_moveStateRadius` (raw dwords); `vtbl = 0x87004C`; `+0x20 = 0; +0x24 m_followNodeThresholdHeightChange`; two CTaskTimers zeroed (`+0x28, +0x2C = 0; +0x30, +0x31 = 0 (u8)`; `+0x34, +0x38 = 0; +0x3C, +0x3D = 0`); embedded calculator `+0x40 = 0x86FD18` (CEntitySeekPosCalculatorFixedPos vtable) with `+0x44..+0x4C = 0` (its CVector); flag byte `+0x54 = (+0x54 & 0xF0) | ((bFaceEntityWhenDone & 1) << 1) | (bPlayTiredAnim & 1)` (bits 2-3 cleared, 4-7 kept uninitialised); `+0x50 m_moveState = 6` (PEDMOVE_RUN); `if entity` -> RegisterReference(entity, &+0xC) 0x571B70; return this.

## 0x66CBB0 CTaskComplexSeekEntity<...>::~CTaskComplexSeekEntity  (catalogue: unknown/REA, port level 2)
- Complete dtor, `__thiscall void (this)`, plain `ret`; SEH frame (state 1 at the cleanup: the embedded calculator member is still alive). Caller: 0x66CB90 (scalar deleting dtor, slot 0 of 0x87004C).
- Behaviour: `vtbl = 0x87004C`; `if +0xC (m_entityToSeek)` -> CleanUpOldReference(+0xC entity, &this->+0xC) 0x571A00; `this->+0x40 = 0x859DC4` (embedded CEntitySeekPosCalculator member reset to the base vtable = its inlined dtor, 0x66CBED); CTaskComplex::~CTaskComplex 0x61A3D0.

## 0x673070 CTaskComplexSeekEntity<...>::CreateSubTask (proposed: CTaskComplexSeekEntity<CEntitySeekPosCalculatorFixedPos>::CreateSubTask)  (catalogue: unknown/REA, port level 4)
- `__thiscall CTask* (this, int32 taskType [esp+4], CPed* ped [esp+8])`, `ret 8`; SEH frame (entry `nop; jmp` .HOODLUM stub). Fields: +0xC m_entityToSeek, +0x18 m_maxEntityDist2D, +0x1C m_moveStateRadius, +0x24 m_followNodeThresholdHeightChange, +0x40 calculator, +0x50 m_moveState, +0x54 flags. Allocation failures -> 0 unless noted.
- By taskType:
  - `0xDB` (TIRED): `new(0x1C) CTaskSimpleTired(1000)` 0x630F20.
  - `0xCB` (STAND_STILL): `new(0x20) CTaskSimpleStandStill(2000, false, false, 8.0f)`.
  - `0xF4` (SEQUENCE): `seq = new(0x40) CTaskComplexSequence()` (NULL on failure, AddTask then on NULL); if `+0x54 & 2` (m_bFaceEntityWhenDone) -> `seq->AddTask(new(0x28) CTaskComplexTurnToFaceEntityOrCoord(m_entityToSeek, [0x86FC7C]=0.5f, [0x86FC80]=0.2f))`; `seq->AddTask(new(0x20) CTaskSimpleStandStill(100, false, false, 8.0f))`; return seq.
  - `0x2C0` (LEAVE_CAR): `new(0x34) CTaskComplexLeaveCar(ped->m_pVehicle, 0, 0, true, false)`.
  - `0x2CF` (CAR_DRIVE_TIMED): `new(0x70) CTaskSimpleCarDriveTimed(ped->m_pVehicle, 2000)` 0x5FF940.
  - `0x38A` (FOLLOW_NODE_ROUTE): `p = new(0x60)`; NULL -> 0; `calc->ComputeEntitySeekPos(ped, entity, &L)` (vslot 0 of +0x40, evaluated after the allocation); return `CTaskComplexFollowNodeRoute(p, m_moveState, &L, m_maxEntityDist2D, m_moveStateRadius, m_followNodeThresholdHeightChange, false, -1, true)` 0x66EA30.
  - `0x387` (GO_TO_POINT_AND_STAND_STILL): `p = new(0x28)`; if p: compute L as above and `EAX = CTaskComplexGoToPointAndStandStill(p, m_moveState, &L, m_maxEntityDist2D, m_moveStateRadius, false, false)`; else EAX = 0. Then OUTSIDE the NULL check: `EAX->+0x24 = (EAX->+0x24 & ~8) | (this->+0x54 & 8)` (original bug: NULL read/write at 0x24 on allocation failure). Return EAX.
  - `0x398` (TURN_TO_FACE_ENTITY): `new(0x28) CTaskComplexTurnToFaceEntityOrCoord(m_entityToSeek, 0.5f, 0.2f)`.
  - `0x516` or anything else -> 0.

## 0x673BA0 CTaskComplexSeekEntity<...>::ControlSubTask-helper (proposed: CTaskComplexSeekEntity<...>::ComputeSeekPos)  (catalogue: unknown/REA, port level 0)
- `__thiscall void (this, CPed* ped [esp+4], CVector* out [esp+8])`, `ret 8`. Caller: vt_87004c::ControlSubTask 0x673540 (2 sites).
- Behaviour: `calc = this + 0x40` (embedded CEntitySeekPosCalculator); `calc->vfunc[0](ped, this->+0xC (m_entityToSeek), &tmp)` (thiscall on calc, `call [edx]` 0x673BBA = ComputeEntitySeekPos); `*out = tmp` (3 dwords, 0x673BC6-0x673BD3).
