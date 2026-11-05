# InteriorManager_c - full specs (constructor / destructor only)

Source of truth: the 1.0 US disassembly. gr declares both as `= default` with address comments only
(no hooks). The other InteriorManager_c methods are reversed in gr and are not part of this assignment.

## Layout (size 0x4758; gr's offsets in InteriorManager_c.h match every offset used below)

| off | type | gr name |
|---|---|---|
| 0x0000 | Interior_c[8] (stride 0x794) | m_Interiors |
| 0x3CA0 | TList_c<Interior_c> | m_InteriorPool |
| 0x3CAC | InteriorGroup_c[8] (stride 0xBC) | m_InteriorGroups |
| 0x428C | TList_c<InteriorGroup_c> | m_InteriorGroupList |
| 0x4298 | TList_c<InteriorGroup_c> | m_InteriorGroupPool |
| 0x42A4 | u32 | m_InteriorCount |
| 0x42A8 | int32[64] | m_InteriorIds |
| 0x43A8 | int32 | m_ObjectCount |
| 0x43AC | InteriorObject[32] (stride 0x1C) | m_Objects |
| 0x472C | bool[16] | m_InteriorPedsAliveState |
| 0x473C | CEntryExit* | m_EnEx |
| 0x4740 | CRect | m_EnExRect |
| 0x4750..0x4752 | bool x3 | m_IsFrozen, m_IsActive, m_ArePedsEnabled |
| 0x4754 | u32 | m_TimeLastPickupsGenerated |

Global instance: g_interiorMan at 0xBAF670.

## 0x5984c0 InteriorManager_c::InteriorManager_c  (catalogue: specified/REA, port level 1)
- Convention: `__thiscall` (this = ECX: InteriorManager_c*, in practice `g_interiorMan` 0xBAF670), no args, returns `this` in EAX, `ret`. SEH frame (handler 0x83d12d) for unwinding partially constructed members.
- Callers: 1 (`__init_g_interiorMan` 0x84f740). Callees: `eh_vector_constructor_iterator` 0x821d30 (x2), 0x4a8dd0 TList_c ctor (x3: `head = tail = 0, cnt = 0` at +0/+4/+8).
- Behaviour (in order):
  1. `eh_vector_constructor_iterator(this+0, 0x794, 8, Interior_c::Constructor 0x5921d0, Interior_c::Destructor 0x591360)` — 8 Interior_c.
  2. TList_c ctor on `this+0x3CA0` (m_InteriorPool).
  3. `eh_vector_constructor_iterator(this+0x3CAC, 0xBC, 8, InteriorGroup_c::Constructor 0x597fe0, InteriorGroup_c::Destructor 0x597ff0)`.
  4. TList_c ctor on `this+0x428C` (m_InteriorGroupList), then on `this+0x4298` (m_InteriorGroupPool).
  5. Inline CRect default ctor at `this+0x4740` (m_EnExRect): `+0x4740 left = 1000000.0f`, `+0x474C bottom = 1000000.0f`, `+0x4748 right = -1000000.0f`, `+0x4744 top = -1000000.0f` (written in that order; gr names from Rect.h: left, top, right, bottom).
- Nothing else is written (counts, ids, objects, flags stay as in .bss = 0 until Init 0x5c0500).
- gta-reversed: `= default` with member initialisers `{}` (would zero the other members; the original does not, harmless for a static object in .bss).

## 0x598580 InteriorManager_c::~InteriorManager_c  (catalogue: specified/REA, port level 1)
- Convention: `__thiscall` (this = ECX), no args, `ret`. SEH frame (handler 0x83d18d).
- Callers: 1 (`__dtor_g_interiorMan` 0x856650, tail call). Callees: 0x4a8de0 TList_c dtor (bare `ret`, x3), `eh_vector_destructor_iterator` 0x821e02 (x2).
- Behaviour (reverse construction order): TList_c dtor on `+0x4298`, on `+0x428C`; `eh_vector_destructor_iterator(this+0x3CAC, 0xBC, 8, InteriorGroup_c::Destructor 0x597ff0)`; TList_c dtor on `+0x3CA0`; `eh_vector_destructor_iterator(this, 0x794, 8, Interior_c::Destructor 0x591360)`. All member destructors are no-ops, so the function has no observable effect.
- gta-reversed: `= default`.
