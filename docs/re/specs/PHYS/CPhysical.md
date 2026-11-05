# CPhysical (PHYS), full specs for REA-source functions

Layout (gta-reversed Entity/Physical.h, offsets confirmed by the bytes below): `CEntity` base (`m_placement`
pos +0x4, `m_matrix` (CMatrix*) +0x14, `m_pRwObject` +0x18, entity flags dword +0x1c (bit 8 = `m_bIsBIGBuilding`),
type byte +0x36 (low 3 bits: 2 vehicle, 3 ped, 4 object)), `physicalFlags` dword +0x40 (bit 0
`bMakeMassTwiceAsBig`, 0x2 `bApplyGravity`, 0x4 `bDisableCollisionForce`, 0x8 `bCollidable`, 0x10
`bDisableTurnForce`, 0x20 `bDisableMoveForce`, 0x40 `bInfiniteMass`, 0x80 `bDisableZ`), `m_vecMoveSpeed` +0x44,
`m_vecTurnSpeed` +0x50, `m_vecFrictionMoveSpeed` +0x5c, `m_vecFrictionTurnSpeed` +0x68, `m_vecForce` +0x74,
`m_vecTorque` +0x80, `m_fMass` +0x8c, `m_fTurnMass` +0x90, `m_fVelocityFrequency` +0x94, `m_fAirResistance` +0x98,
`m_fElasticity` +0x9c, `m_fBuoyancyConstant` +0xa0, `m_vecCentreOfMass` +0xa4, `m_pCollisionList`
(`CEntryInfoList`, head `CEntryInfoNode*`) +0xb0.

Helper layouts used by Add / RemoveAndAdd: `CEntryInfoNode` (pool, 0x14): `list` (CPtrListDoubleLink*) +0,
`link` (CPtrNodeDoubleLink*) +4, `repeatSector` +8, `prev` +0xc, `next` +0x10. `CPtrNodeDoubleLink` (pool, 0xc):
`item` +0, `next` +4, `prev` +8. A `CPtrListDoubleLink` is a single head pointer. `CRepeatSector` (0xc):
Vehicles +0, Peds +4, Objects +8; `CWorld::ms_aRepeatSectors` 0xb992b8, 16x16, index `(y & 15)*16 + (x & 15)`.
`CRect` from `GetBoundRect`: left +0, top +4, right +8, bottom +0xc (top = max y, bottom = min y).

## 0x441130 CPhysical::SetVelocity  (catalogue: specified/REA, port level 0)
- Convention / signature: `__thiscall void (CPhysical* this = ECX, CVector v)` with `v` passed by value at
  [esp+4..0xc]; `ret 0xc`. Stub `jmp 0x156c740`.
- Callers: 7 (script opcode handlers 0x470a90, 0x481300, 0x483bd0, 0x48cdd0; `CObject::ProcessControl`,
  `ProcSurfaceInfo_c::AddObject`, `CPlane::DoPlaneGenerationAndRemoval` x2). Callees: none. Globals: none.
- Behaviour: `this->m_vecMoveSpeed = v` (+0x44, +0x48, +0x4c; plain dword copies through a local, no FPU).
- gta-reversed: inline `SetVelocity` in Physical.h:231, identical. Open questions: none.

## 0x4abba0 CPhysical::ApplyMoveForce(float x, float y, float z)  (catalogue: specified/REA, port level 1)
- Convention / signature: `__thiscall void (CPhysical* this = ECX, float x [esp+4], float y [esp+8], float z
  [esp+0xc])`, `ret 0xc`. ECX is passed through unchanged to the callee.
- Callers: 4 (`CEventKnockOffBike::CalcForcesAndAnims` x2, `CTaskSimpleInAir::ProcessPed`,
  `CTaskSimpleSwim::ProcessSwimmingResistance`, `CPed::MakeTyresMuddySectorList<...>` x2). Callees:
  `CPhysical::ApplyMoveForce(CVector)` 0x5429f0 (`__thiscall`, vector by value, `ret 0xc`).
- Behaviour: copies (x, y, z) as raw dwords into a local, then pushes a by-value copy and calls
  `this->ApplyMoveForce(CVector{x, y, z})`. No float arithmetic.
- gta-reversed: Physical.cpp:712, identical. Open questions: none.

## 0x542560 CPhysical::RemoveAndAdd  (catalogue: specified/REA, port level 5)
- Convention / signature: `__thiscall void (CPhysical* this = ECX)`, plain `ret`. EBP = this, EDI = current list
  head pointer, EBX = repeat sector.
- Callers: 6 (`CVehicleRecording::SaveOrRetrieveDataForThisFrame`, `CPhysical::ProcessShift` x2,
  `CPhysical::ProcessCollision` x3, `CHandObject::ProcessControl`, `CTrain::ProcessControl`,
  `CEscalator::Update` x3). Callees: `CEntity::Remove` 0x534ae0, `CEntity::Add()` 0x533020, virtual
  `GetBoundRect` (vtable +0x24; `__thiscall CRect*(CRect* out)`, `ret 4`; CPhysical's is 0x5449b0), CRT `floor`
  0x8219f0 (double in, ST0 out), `_ftol` 0x821b40, `CPtrNodeDoubleLink::operator new` 0x5523c0 / `operator
  delete` 0x5523d0, `CEntryInfoNode::operator new` 0x536dc0 / `operator delete` 0x536dd0 (all cdecl).
- Globals: `CWorld::ms_aRepeatSectors` 0xb992b8 (read/written through list heads); constants 0.02f 0x858b38,
  60.0f 0x858b34.
- Behaviour:
  1. If `this.flags(+0x1c) & 0x100` (big building): `CEntity::Remove(this)`, `CEntity::Add(this)`, return.
  2. `cursor = this->m_pCollisionList` (+0xb0). `rect = this->GetBoundRect()`.
  3. Sector bounds, computed in this order, each as `_ftol(floor((double)(v * 0.02f + 60.0f)))` (x87: `fld f32;
     fmul f32; fadd f32; fstp qword` then CRT floor then `_ftol`): `x0` from rect.left, `y0` from rect.bottom,
     `x1` from rect.right, `y1` from rect.top.
  4. `list` (EDI) starts as the integer `y1` (garbage pointer; only used if the type switch below never matches).
     For `y = y0 .. y1` (inclusive, signed compare; skipped entirely if `y0 > y1`), for `x = x0 .. x1` (inclusive):
     - `sector = &ms_aRepeatSectors[(y & 15)*16 + (x & 15)]`.
     - `switch (type & 7)`: 2 -> `list = &sector.Vehicles`; 3 -> `&sector.Peds`; 4 -> `&sector.Objects`; any
       other value leaves `list` unchanged.
     - If `cursor != NULL` (reuse an existing node): `old = cursor->list`, `link = cursor->link`;
       unlink: `if (*old == link) *old = link->next`; `if (link->prev) link->prev->next = link->next`;
       `if (link->next) link->next->prev = link->prev`. Push front on `list`: `link->prev = 0`; `link->next =
       *list`; `if (*list) (*list)->prev = link`; `*list = link`. Then `cursor->repeatSector = sector`;
       `cursor->list = list`; `cursor = cursor->next`.
     - Else (new node): `link = CPtrNodeDoubleLink::new(0xc)`; if non-null `link->item = this` (a NULL link is
       still dereferenced: crash); `link->prev = 0`; `link->next = *list`; `if (*list) (*list)->prev = link`;
       `*list = link`. Then `node = CEntryInfoNode::new(0x14)`; if non-null: `node->repeatSector = sector`,
       `node->list = list`, `node->link = link` (a NULL node is still written below: crash). `node->prev = 0`;
       `node->next = this->m_pCollisionList`; `if (head) head->prev = node`; `this->m_pCollisionList = node`
       (new nodes go to the FRONT, behind which the cursor keeps walking the old nodes).
  5. Tail: while `cursor != NULL`: `next = cursor->next`; unlink `cursor->link` from `cursor->list` (same three
     steps as above); `CPtrNodeDoubleLink::delete(link)`; `if (this->m_pCollisionList == cursor)
     this->m_pCollisionList = next`; `if (cursor->prev) cursor->prev->next = cursor->next`; `if (cursor->next)
     cursor->next->prev = cursor->prev`; `CEntryInfoNode::delete(cursor)`; `cursor = next`.
- Edge cases: allocation order in the new-node path is link first, node second (Add uses the opposite order).
  An empty rect range (`x0 > x1` or `y0 > y1`) frees every node. Non-physical type values reuse a stale/garbage
  list pointer (never happens for CPhysical subclasses).
- gta-reversed: present but not hooked as reversed (`.Reversed = false`); its logic is the same as the bytes
  (CAT-14 row Physical.cpp:621 is only a comment). It uses `CWorld::GetSectorX` (`floor` + int cast) and the
  list helpers; port the exact order above.
- Open questions: none.

## 0x543100 CPhysical::ApplyFrictionTurnForce  (catalogue: specified/REA, port level 1)
- Convention / signature: `__thiscall void (CPhysical* this = ECX, CVector force [esp+4..0xc], CVector point
  [esp+0x10..0x18])`, both by value, `ret 0x18`. (gta-reversed declares `(CVector posn, CVector velocity)`; the
  bytes use the first vector as the force and the second as the application point, like `ApplyFrictionForce`.)
- Callers: none in the original call graph (only gta-reversed wrappers). Callees: `Multiply3x3` 0x59c790
  (`__cdecl CVector*(CVector* out, CMatrix* m, CVector* v)`), `CVector::CrossProduct` 0x59c730 (`__cdecl
  CVector*(CVector* out, CVector* a, CVector* b)`, out = a x b).
- Globals: 0.5f 0x858b8c, 0.0f 0x858b50, 1.0f 0x858624.
- Behaviour:
  1. If `flags & 0x10` (bDisableTurnForce): return.
  2. `turnMass = this.m_fTurnMass`; `com = (0,0,0)`.
     If `flags & 0x40` (bInfiniteMass): `turnMass = ((com_z * mass) * com_z) * 0.5f + turnMass` with
     `com_z = this.m_vecCentreOfMass.z` (x87: `fld z; fld st0; fmul mass; fmul st1; fmul 0.5; fadd turnMass`).
     Else `com = Multiply3x3(this->m_matrix, &this->m_vecCentreOfMass)` (matrix pointer +0x14, not null-checked).
  3. If `flags & 0x20` (bDisableMoveForce): `force.z = 0` and `pz = 0.0f`; else `pz = point.z`.
  4. `r = (point.x - com.x, point.y - com.y, pz - com.z)` (each stored f32).
  5. `c = CrossProduct(r, force)`.
  6. `k = 1.0f / turnMass`; `this.m_vecFrictionTurnSpeed.x = k*c.x + .x`; then `.y += k*c.y`; `.z += k*c.z`
     (products `k*c.y`, `k*c.z` are stored to f32 temporaries before the adds; the x product is added directly).
- Edge cases: `turnMass == 0` gives inf/NaN like the original (no guard).
- gta-reversed: only a `plugin::Call` wrapper (Physical.cpp:853). Open questions: none.

## 0x544100 CPhysical::ApplySpringDampeningOld  (catalogue: specified/REA, port level 2)
- Convention / signature: `__thiscall bool (CPhysical* this = ECX, float damping [esp+4], float limit [esp+8],
  CVector* dir [esp+0xc], CVector* point [esp+0x10], CVector* speed [esp+0x14])`, returns AL = 1 always,
  `ret 0x14`. Parameter names follow the newer `ApplySpringDampening` (gr Physical.cpp:1157). `limit` is
  never read.
- Callers: none in the original call graph. Callees: `CPhysical::GetSpeed` 0x542ce0 (`__thiscall CVector*(CVector*
  out, CVector point)`, `ret 0x10`), `CPhysical::ApplyForce` 0x542b50 (`__thiscall (CVector force, CVector point,
  bool updateTurnSpeed)`, `ret 0x1c`).
- Globals: `CTimer::ms_fTimeStep` 0xb7cb5c (read), 3.0f 0x858b3c, 0.5f 0x858b8c, -0.53f 0x863c04, 1.0f
  0x858624, 0.0f 0x858b50.
- Behaviour (x87 order):
  1. `s1 = (speed.y*dir.y + speed.z*dir.z) + dir.x*speed.x` (stored f32, reuses the `speed` argument slot).
  2. `v = GetSpeed(*point)`; `s2 = (v.y*dir.y + v.z*dir.z) + v.x*dir.x` (stored f32 via `fst`).
  3. `avg = (s2 + s1) * 0.5f` (kept in ST).
  4. `t = (ms_fTimeStep < 3.0f) ? ms_fTimeStep : 3.0f` (NaN -> 3.0f).
  5. `imp = ((avg * (t * mass)) * damping) * -0.53f`; if `flags & 1` (bMakeMassTwiceAsBig) `imp = imp + imp`.
  6. `ratio = turnMass / ((((p.x*p.x + p.y*p.y) + p.z*p.z) + 1.0f) * mass * 2)` with `p = *point` (computed as
     `((...)+1.0f) * mass`, then `fadd st0,st0`, then `fdivr turnMass`). If `ratio > 1.0f` -> `ratio = 1.0f`
     (NaN kept).
  7. `q = imp / (s2 * mass)`; if `q < 0` -> `q = -q` (NaN kept). If `q > ratio` (strict; NaN false):
     `imp = imp * (ratio / q)`.
  8. `ApplyForce(CVector{imp*dir.x, imp*dir.y, imp*dir.z}, *point, true)`; return 1.
- Edge cases: `s2 == 0` gives `q = +-inf` (then `imp *= ratio/inf = 0` if `ratio` finite) - reproduce with x87
  semantics. Intermediates in steps 5-7 stay in x87 registers (24-bit precision control).
- gta-reversed: only a `plugin::Call` wrapper (Physical.cpp:1195). Open questions: none.

## 0x544990 CPhysical::vfunc_0 (scalar deleting destructor)  (catalogue: specified/REA, port level 14)
- Convention / signature: `__thiscall CPhysical* (CPhysical* this = ECX, uint8 flags [esp+4])`, returns EAX =
  this, `ret 4`. vtable slot 0 of `CPhysical`.
- Callers: 1 typed virtual site (catalogue). Callees: `CPhysical::~CPhysical` 0x542450, CRT `free`/`operator
  delete` 0x8214bd (cdecl).
- Behaviour: `~CPhysical(this)`; `if (flags & 1) operator delete(this)` (0x8214bd); return this.
- gta-reversed: compiler-generated (virtual destructor). Open questions: none.

## 0x544a30 CPhysical::Add  (catalogue: specified/REA, port level 5)
- Convention / signature: `__thiscall void (CPhysical* this = ECX)`, plain `ret`; virtual (overrides
  `CEntity::Add`). Has an SEH frame (unwind deletes the CEntryInfoNode if the inner `new` throws; not
  reproducible behaviour).
- Callers: 2 typed virtual sites (`CIplStore::LoadIplBoundingBox`, `CIplStore::LoadIpl`) plus all `CWorld::Add`
  dispatches. Callees: `CEntity::Add()` 0x533020, virtual `GetBoundRect` (+0x24), `floor` 0x8219f0, `_ftol`
  0x821b40, `CEntryInfoNode::operator new` 0x536dc0, `CPtrNodeDoubleLink::operator new` 0x5523c0.
- Globals: as RemoveAndAdd.
- Behaviour:
  1. If `flags(+0x1c) & 0x100`: `CEntity::Add(this)`; return.
  2. `rect = GetBoundRect()`; bounds in this order: `x0` (left), `y0` (bottom), `x1` (right), `y1` (top), each
     `_ftol(floor((double)(v*0.02f + 60.0f)))`.
  3. `list` (EBP) starts as the integer `y1` (only used if the type never matches). For `y = y0..y1`, `x = x0..x1`
     (inclusive): sector and list selected exactly as in RemoveAndAdd; then
     `node = CEntryInfoNode::new(0x14)`; if `node`: `link = CPtrNodeDoubleLink::new(0xc)`; if `link`:
     `link->item = this` (NULL link still dereferenced); `link->prev = 0`; `link->next = *list`;
     `if (*list) (*list)->prev = link`; `*list = link`; `node->link = link`; `node->repeatSector = sector`;
     `node->list = list`. Then (also when `node` is NULL: crash) `node->prev = 0`; `node->next =
     this->m_pCollisionList`; `if (head) head->prev = node`; `this->m_pCollisionList = node`.
- Edge cases: does not clear an existing collision list (callers only Add after Remove). Allocation order:
  node first, then link.
- gta-reversed: body is `#if 0` (not reversed); the disabled code matches this behaviour.
- Open questions: none.

## 0x546ed0 CPhysical::ApplyScriptCollision  (catalogue: specified/REA, port level 8)
- Convention / signature: `__thiscall float (CPhysical* this = ECX, CVector normal [esp+4..0xc] (by value),
  float elasticity [esp+0x10], float friction [esp+0x14], CVector* point [esp+0x18])`, result in ST0, `ret 0x18`.
- Callers: none in the original call graph (gta-reversed wrapper only). Callees: `CEntity::GetColModel`
  0x535300 (`__thiscall CColModel*()`), `CPhysical::ApplyCollision(CEntity*, CColPoint&, float&)` 0x5435c0
  (`__thiscall bool`, 3 stack args), `CPhysical::ApplyFriction(float, CColPoint&)` 0x5454c0 (`__thiscall
  bool`, `ret 8`).
- Globals: 0.0f 0x858b50.
- Behaviour:
  1. `savedElasticity = this.m_fElasticity` (+0x9c); `impulse = 0.0f` (local).
  2. Local `CColPoint cp` (0x2c bytes on the stack): `cp.normal = normal` (+0x10..+0x18).
  3. If `point != NULL`: `cp.point = *point`. Else: `pos = this->m_matrix ? m_matrix->pos (+0x30) :
     m_placement.pos (+4)`; `r = GetColModel()->m_boundSphere.m_fRadius` (+0x24); `cp.point = (pos.x - normal.x*r,
     pos.y - normal.y*r, pos.z - normal.z*r)` (products first, each stored f32 except x which stays in ST).
  4. `this.m_fElasticity = elasticity`.
  5. `if (ApplyCollision(this /*entity*/, cp, impulse) && friction > 0.0f /*strict, NaN false*/)
     ApplyFriction(friction, cp)`.
  6. `this.m_fElasticity = savedElasticity`; return `impulse` (loaded from the f32 local).
- Edge cases: `cp` fields other than point and normal (+0xc, +0x1c, surfaces +0x20..+0x25, depth +0x28) are
  uninitialised stack memory when passed to ApplyCollision/ApplyFriction (e.g. ApplyFriction reads the surface
  types for spark effects). A port must reproduce or measure what those callees read (see Open questions).
- gta-reversed: `plugin::Call` wrapper only (Physical.cpp:2156).
- Open questions: the uninitialised `cp` bytes (+0x20..+0x28) at 0x546fae are whatever the caller's stack held;
  if ApplyCollision 0x5435c0 or ApplyFriction 0x5454c0 read them observably, parity needs a measurement.
