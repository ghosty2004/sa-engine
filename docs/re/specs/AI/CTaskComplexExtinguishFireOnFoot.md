# CTaskComplexExtinguishFireOnFoot (AI) - RE-SPEC-FINAL (helper B)

## 0x659870 CTaskComplexExtinguishFireOnFoot::CTaskComplexExtinguishFireOnFoot  (catalogue: unknown/REA, port level 1)
- `__thiscall T* (this, const CVector* pos [esp+4])`, `ret 4`. No callers.
- Behaviour: CTaskComplex ctor 0x61A3B0; `vtbl = 0x86F5FC` (TASK_COMPLEX_EXTINGUISH_FIRE_ON_FOOT); `+0xC/+0x10/+0x14 = pos->x/y/z` (raw dword copies); return this.

## 0x6598B0 CTaskComplexExtinguishFireOnFoot::~CTaskComplexExtinguishFireOnFoot  (catalogue: unknown/REA, port level 2)
- Non-deleting dtor, `__thiscall void (this)`: `this->vtbl = 0x86F5FC` (0x6598B0); tail-jump to base dtor CTaskComplex::~CTaskComplex 0x61A3D0 (0x6598B6). No other member cleanup.

## 0x659990 CTaskComplexExtinguishFireOnFoot::CreateFirstSubTask-helper (proposed: CTaskComplexExtinguishFireOnFoot::FindNearbyFire)  (catalogue: unknown/REA, port level 1)
- `__stdcall CFire* (CPed* ped [esp+4])`, `ret 4` (ECX not used as this). Caller: vt_86f5fc::CreateFirstSubTask 0x65A720.
- Behaviour: `P = ped->m_matrix ? &m_matrix->pos : &ped->m_placement.pos`; `f = CFireManager::FindNearestFire(&gFireManager 0xB71F80, P, false, true)` 0x538F40 (thiscall; pushes 1, 0, P); if NULL return 0. `d = P - f->m_vecPosition (+4)` (dx, dy, dz in that order); `q = (dz*dz + dx*dx) + dy*dy` (x87 order); `fcomp [0x858628]=100.0f; test ah, 5; jp`: return f only if `q < 100.0` (C0 only); greater, equal or NaN -> return 0.
