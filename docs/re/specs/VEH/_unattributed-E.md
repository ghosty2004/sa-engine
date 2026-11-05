# VEH unnamed / foreign-class helpers (writer group E)

Functions assigned to group E whose class file belongs to another writer, or whose class is not certain.

## 0x536E90 CompressedVector helper (proposed: CompressVector(const CVector&) -> CompressedVector)  (catalogue: unknown/REA, port level 0)
- Evidence: converts each component with `* 128.0f` and stores int16 - gr `CompressedVector =
  FixedVector<int16, 128.0f>` (CompressedVector.h) and the deprecated `CompressVector`. Only caller:
  CAutomobile::UpdateMovingCollision 0x6A1460 (3 sites).
- Convention: `__cdecl void (int16 out[3] [esp+4], const CVector* in [esp+8])`. EAX at return is the last `_ftol`
  result, NOT the out pointer, so callers do not use a returned pointer.
- Behaviour: for x, y, z in that order: `out[k] = (int16)_ftol(in[k] * 128.0f)` (0x858BF4 = 128.0f; f32 fmul,
  `_ftol2` 0x821B40 truncation toward zero, low 16 bits stored -> wraps for |v| >= 256).

## 0x6C4130 CPlaceable helper (proposed: SetMatrixHeadingAroundZ(float angle))  (catalogue: unknown/REA, port level 0)
- Evidence: thiscall on an entity (reads `this+0x14` = CPlaceable::m_matrix) from CPlane::DoPlaneGenerationAndRemoval
  0x6CD2F0 (2 sites). Not gr's `CPlaceable::SetHeading` (that one null-checks and uses SetRotateZOnly with a
  different axis convention). Class uncertain (CPlaceable or a CPlane-local inline).
- Convention: `__thiscall void (CPlaceable* this, float angle [esp+4])`, `ret 4`.
- Behaviour (no null check on m_matrix): `s = fsin(angle)`, `c = fcos(angle)` (x87 instructions on the extended
  value); `m->right = (s, -c, 0)`, `m->forward = (c, s, 0)`, `m->up = (0, 0, 1.0f)`; stores in order: right.z,
  forward.z, up.x, up.y = 0; up.z = 1.0f; right.x = s; right.y = -c; forward.x = c; forward.y = s. The position
  and any other matrix fields are untouched.

## 0x6C7D10 CMonsterTruck::~CMonsterTruck (proposed name)  (catalogue: unknown/REA, port level 16)
- Evidence: only caller is the deleting destructor CMonsterTruck::vfunc_0 0x6C7F90; gr MonsterTruck.h:50 comments
  `~CMonsterTruck() // 0x6C7D10, 0x6C7F90`. (CMonsterTruck.md belongs to another writer, hence here.)
- Convention: thiscall, no args.
- Behaviour: `this->vtable = 0x8717D8` (CMonsterTruck vtable); tail jump to CAutomobile::~CAutomobile 0x6A61E0.
