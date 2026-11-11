# _unattributed-FA (POP) - RE-SPEC-FINAL group A

## 0x422E10 CCarCtrl plane line-of-sight helper (sub_422e10)  (catalogue: unknown/REA, port level 8)
Catalogue name: <none>, proposed: `static bool CCarCtrl::FindPlaneObstacleHeight(CVehicle* plane, float pitch, float heading, float* outZ)`
(evidence: only caller 0x422F80 (itself called by CCarCtrl::FlyAIPlaneInCertainDirection 0x423000) passes the pitch table and keeps the
maximum returned z).
- Convention: `__cdecl bool (CVehicle* plane [esp+4], float pitch [esp+8], float heading [esp+0xC], float* outZ [esp+0x10])`, plain `ret`, AL.
  Locals (E = esp at entry): dir CVector [E-0x74], start [E-0x68], tmp0 [E-0x50], s [E-0x48], dyc [E-0x58], dzc [E-0x54], X [E-0x38],
  end [E-0x44], CColPoint [E-0x2C] (0x2C bytes), out entity [E-0x78].
- Callees: CVector::Normalise 0x59C910; CWorld::ProcessLineOfSight 0x56BA00 (cdecl, 12 args).
- Globals: 50.0f [0x858B40]=50.0f; 0.0f [0x858B50]=0.0f; 200.0f [0x858A48]=200.0f.
- Behaviour:
  1. `start = pos + moveSpeed*50`: x87 `vx = speed.x (+0x44) * 50` -> f32, `vy = speed.y*50` -> f32, `vz = speed.z*50` (st), pos = matrix ?
     matrix+0x30 : plane+4; `start.z = vz + pos.z; start.y = vy + pos.y; start.x = vx + pos.x` (each f32).
  2. `dir = (cos(heading), sin(heading), 0)` (fcos/fsin on the f32 arg, stored f32), `dir.Normalise()`.
  3. `s = sin(pitch)`; `z0 = 0.0f * s` (stored f32 AND kept in st); `c = cos(pitch)`; `X = dir.x*c + z0_f32`, `Y = dir.y*c + z0_reg`,
     `Z = dir.z*c + s_f32` (dir.y*c, dir.z*c stored f32 first; X stored f32); `end = start + (X*200, Y*200, Z*200)` (each product stored f32,
     then added to start, f32).
  4. `hit = ProcessLineOfSight(start, end, colPoint, outEntity, buildings 1, vehicles 0, peds 0, objects 0, dummies 0, seeThrough 0,
     cameraIgnore 0, shootThrough 1)`. If hit: `*outZ = colPoint.m_vecPoint.z` (colPoint+8) and return 1; else return 0.
- gta-reversed: absent. Open questions: none.

## 0x422F80 CCarCtrl plane direction helper (sub_422f80)  (catalogue: unknown/REA, port level 9)
Catalogue name: <none>, proposed: `static float CCarCtrl::FindMaxObstacleHeightForPlane(CVehicle* plane, float heading)` (caller
CCarCtrl::FlyAIPlaneInCertainDirection 0x423000).
- Convention: `__cdecl float (CVehicle* plane [esp+4], float heading [esp+8])`, result ST0, plain `ret`.
- Globals: pitch table f32[6] at 0x8A5B2C (.data) = {0.10471976, 0.05235988, 0.0, -0.34906587, -0.69813174, -1.0471976}; 100000.0f [0x858EF8]=100000.0f.
- Behaviour: `maxZ = 0.0f` (int 0 stored); for each table entry p in order (0x8A5B2C .. <0x8A5B44): if `0x422E10(plane, p, heading, &z)` hits:
  if p is the FIRST entry (pointer == 0x8A5B2C) return 100000.0f immediately; else if `!(maxZ > z)` (`fld maxZ; fcomp z; test ah,0x41; je`;
  equal/NaN replace) `maxZ = z`. Return maxZ.
- gta-reversed: absent (FlyAIPlaneInCertainDirection is `Reversed = false`). Open questions: none.

## 0x44FB60 sub_44fb60 (path data, this+0x10c4)  (catalogue: unknown/REA, port level 0)
Catalogue name: <none>, proposed: `int32 CPathFind::FindNodesInRange(const CVector* pos, float radius, uint8 nodeType, int32 maxCount,
CNodeAddress* out, bool skipFlag20, int32 unused7, bool skipFlag200, bool switchedOff, int32 unused10)` (evidence: walks all 72 areas'
node arrays of CPathFind: +0x804 m_pPathNodes[a] (0x1C-byte CPathNode), +0xFA4 / +0x10C4 per-area node counts; no code xrefs - dead in 1.0 US).
- Convention: `__thiscall int32 (CPathFind* this, 10 dword args)`, `ret 0x28`, EAX = number of addresses written. Arg slots: pos [esp+4],
  radius [esp+8], type [esp+0xC] (u8), maxCount [esp+0x10], out [esp+0x14], a6 [esp+0x18] (u8), a7 [esp+0x1C] (unused), a8 [esp+0x20] (u8),
  a9 [esp+0x24] (u8, read once into CL, then the slot is REUSED as the loop end), a10 [esp+0x28] (unused).
- Globals: 0.125f [0x858C48]=0.125f.
- Behaviour: `n = 0`; `for (area = 0; area < 72; area++)`: skip the area if `this->+0x804[area] == 0`. Range by type: 0 -> [0, this->+0x10C4[area]);
  1 -> [this->+0x10C4[area], this->+0xFA4[area]); any other value -> start = the previous area's start local and end = the a9 slot's current
  content (stale; uninitialised start on the first area). `r2 = radius*radius` (f32, recomputed per area). If start >= end next area.
  For `i = start; i < end; i++`, node = nodes + i*0x1C:
  - skip if `a6 && (node[+0x18] & 0x20)`; skip if `a8 && (node[+0x19] & 2)`; skip if `(node[+0x18] >> 7) != CL (a9 low byte)`.
  - `nx = (i16)node[+8] * 0.125f`, `ny = (i16)node[+0xA] * 0.125f`, `nz = (i16)node[+0xC] * 0.125f` (nz stored f32);
    `dx = pos.x - nx` -> f32, `dy = pos.y - ny` (st), `dz = pos.z - nz_f32`; `d2 = dz*dz + dy*dy; d2 += dx_f32*dx_f32`; if `d2 < r2`
    (`test ah,5; jp`): `out[n] = {u16 area, u16 i}`; n++.
  - After the distance test (inside or outside, not after the flag skips): `if (n == maxCount) return n`.
  Return n.
- gta-reversed: absent. Open questions: semantic names of node flag bits +0x18&0x20, +0x18 bit7, +0x19&2; behaviour for type > 1 relies on
  stale locals (0x44FBBD) - only reachable with an invalid type.

## 0x4AE1A0 vt_85ae88 ctor (from CloneEditable)  (catalogue: unknown/REA, port level 2)
Catalogue name: <none>, proposed: `CEventPotentialGetRunOver::CEventPotentialGetRunOver(CVehicle* vehicle)` (vtable 0x85AE88, GetEventType 0x4AE210
returns 12 = EVENT_POTENTIAL_GET_RUN_OVER; callers CCarCtrl::SlowCarDownForPedsSectorList 0x425440 (2), vt_85ae88::CloneEditable 0x4B6F40).
- Convention: `__thiscall CEventPotentialGetRunOver* (this, CVehicle* v [esp+4])`, `ret 4`, returns this. SEH frame (handler 0x839B88).
- Behaviour (store order): +4 = 0, +8 byte = 0, +0xC byte = 1, +0xE i16 = 0xC8 (200), +0x10 i16 = -1 (CEventEditableResponse base);
  vtable = 0x85AE88; +0x14 = v; if v `v->RegisterReference(&this->+0x14)` 0x571B70.
- gta-reversed: CEventPotentialGetRunOver (Events) - same fields. Open questions: none.

## 0x5DE540 CGangInfo (proposed: CGangInfo::~CGangInfo)  (catalogue: specified/REA, port level 0)
- Empty body (`ret`); element dtor of CGangs::Gang (0x850CE0 / 0x856760). Open questions: none.
