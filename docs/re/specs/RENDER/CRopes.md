# CRopes (RENDER) - full specs

`CRopes::aRopes` 0xB768B8: 8 x CRope (stride 0x328, end 0xB781F8). CRope layout (gr `Rope.h`, VALIDATE_SIZE 0x328; the gr member
names for +0x304..+0x31C are doubtful, see RegisterRope): +0x000 m_aSegments[32] (CVector), +0x180 m_aSpeed[32] (CVector),
+0x300 m_nId (u32), +0x304 (gr m_fGroundZ), +0x308 (gr m_fMass; holds the rope length), +0x30C (gr m_fTotalLength; holds the
segment spacing = length/31), +0x310 m_pRopeHolder, +0x314 m_pAttachedEntity, +0x318 m_pRopeAttachObject, +0x31C (gr
m_fSegmentLength; 0.9 / 0.5 depending on the holder), +0x320 m_nTime (expiry ms), +0x324 m_nSegments (u8), +0x325 m_nType (u8,
0 = NONE), +0x326 (gr m_nFlags1), +0x327 (gr m_nFlags2; bit 0 set on (re)registration, bit 2 = `flags & 1`).

## 0x555E40 CRopes::FindCoorsAlongRope  (catalogue: specified/REA, port level 0)
- Convention: `__cdecl bool (uint32 ropeId [esp+4], float t [esp+8], CVector* outPos [esp+0xC], CVector* outSpeed [esp+0x10])`,
  plain `ret`; result in AL only (0x555E68 `xor al,al` / 0x555F6D `mov al,1`; upper EAX bits are garbage).
- Callers: CTaskComplexUseSwatRope::ControlSubTask 0x65A460. Callees: CRT `_ftol` 0x821B40 (truncation toward zero, pops ST0).
- Constants: [0x858B50] = 0.0f, [0x863E2C] = 0.999f (0x3F7FBE77), [0x863E28] = 31.0f, [0x858624] = 1.0f.
- Behaviour:
  1. Find rope: for i = 0..7 (pointer 0xB76BB8 + i*0x328 = &aRopes[i].m_nId, 0x555E50-0x555E66): match if `m_nType (+0x325) != 0`
     and `m_nId == ropeId`. No match: return false (outputs untouched).
  2. Clamp t (0x555E6F-0x555E9D): if `0.0f > t` (fcomp 0.0 vs t, `test ah,0x41; jne` not taken only when 0.0 > t) then t = 0.0 and
     skip the upper clamp; else if `t > 0.999f` (`test ah,5; jp` not taken only for 0.999 < t) then t = 0.999. NaN passes unchanged.
  3. `s = t * 31.0f` (0x555EA3); `n = _ftol(s)` (0x555EAB, trunc); `f = s - n` (fisub, 0x555EB4). A = m_aSegments[n],
     B = m_aSegments[n+1].
  4. x87, in this order: `bx = f * B.x` (kept in ST, 0x555ED3); `by = f * B.y` -> f32 (0x555EE5); `bz = f * B.z` -> f32 (0x555EF1);
     `g = 1.0f - f` (0x555EFB); `ax = g * A.x` -> f32, `ay = g * A.y` -> f32, `az = g * A.z` -> f32 (0x555EFD-0x555F11);
     `outPos.x = ax + bx` (f32 + ST value, 0x555F19), `outPos.y = ay + by`, `outPos.z = az + bz` (f32 + f32, 0x555F29-0x555F4C).
  5. If `outSpeed != NULL` (0x555F53): `*outSpeed = m_aSpeed[n+1]` (raw copy, 0x555F57-0x555F6A; not interpolated).
  6. Return true.
- Edge cases: t is clamped to <= 0.999 so n+1 <= 31; NaN t gives `_ftol(NaN)` = 0x80000000 -> out-of-bounds read (original behaviour).
- gta-reversed: `plugin::CallAndReturn` only (Ropes.cpp:94). Open questions: none.

## 0x556B40 CRopes::RegisterRope  (catalogue: specified/REA, port level 16)
- Convention: `__cdecl` (gr signature confirmed by the stack reads): `bool RegisterRope(uint32 ropeId [esp+4], uint32 ropeType [+8]
  (only the low byte is used, 0x556D1C), CVector startPos by value [+0xC/+0x10/+0x14], bool bExpires [+0x18], uint8 segmentCount
  [+0x1C], uint8 flags [+0x20], CPhysical* holder [+0x24], uint32 timeExpire [+0x28])`, plain `ret`; returns EAX = 0 / 1 (full dword).
  (In the body the args are at [esp+0x4C..+0x70] after `sub esp,0x38` and 4 pushes.) Body split over 0x556B40-0x55747D and 0x557480-0x557512.
- Callers (5): CRopes::CreateRopeForSwatPed 0x558D10, CObject::ProcessControlLogic 0x5A29A0, CHeli::SendDownSwat 0x6C69C0,
  CHeli::ProcessControl 0x6C7050, CVehicle::UpdateWinch 0x6D3B80.
- Callees: CRope::CreateHookObjectForRope 0x556070 (thiscall ECX = &aRopes[i]; 3 sites, one per exit path), CEntity::RegisterReference
  0x571B70 (thiscall ECX = holder, arg CEntity** = &rope.m_pRopeHolder).
- Globals: aRopes (see header), CTimer::m_snTimeInMilliseconds 0xB7CB84. Jump table 0x557514 (index = type - 3, 5 entries).
- Behaviour:
  1. Existing rope (0x556B4D-0x556B6A): for i = 0..7, if `aRopes[i].m_nType != 0 && aRopes[i].m_nId == ropeId` go to 2. Otherwise
     go to 3.
  2. Re-register (0x556B92-0x556C5C), r = &aRopes[i]:
     a. `seg[0] = startPos` (raw dwords), `speed[0] = (0,0,0)`.
     b. `byte [r+0x327] |= 1` (0x556BE8); `r->m_nSegments = segmentCount` (0x556BFB).
     c. `test bl,bl; jb` (0x556BF5/0x556C01) is never taken (CF = 0 after test), so: for k = 0; k <= r->m_nSegments (u8 re-read each
        iteration, 0x556C38-0x556C45); k++: `seg[k] = startPos; speed[k] = (0,0,0)`. That writes segmentCount+1 entries
        (segmentCount >= 32 overruns into m_aSpeed / the next fields - original behaviour; existing callers pass small counts).
     d. `r->CreateHookObjectForRope()` (0x556C4B); return 1. Type, holder, lengths, timer, flags other than bit 0 are NOT changed.
  3. Free slot (0x556B6C-0x556B86): first i in 0..7 with `aRopes[i].m_nType == 0` (byte scan from 0xB76BDD). None: return 0 (0x556B8B).
  4. New rope (0x556C5D-0x556DC0), r = &aRopes[i], in this store order:
     `m_nId = ropeId` (0x556C67); `seg[0] = startPos`; `speed[0] = (0,0,0)`;
     `byte [r+0x327] = ((flags & 1) << 2) | (old & 0xF9) | 1` (0x556CBD-0x556CDF; bit 1 cleared, bits 3-7 kept);
     `m_nSegments = segmentCount` (0x556CD9); `dword [r+0x304] = 0` (0x556CE5); `m_pAttachedEntity = NULL` (0x556CEB);
     `m_pRopeAttachObject = NULL` (0x556CF1);
     `[r+0x31C] = (holder && (holder->+0x36 & 7) == 2 /*vehicle*/) ? 0.9f (0x3F666666, 0x556D04) : 0.5f (0x3F000000, 0x556D10)`;
     `byte [r+0x326] = 0` (0x556D26); `m_pRopeHolder = holder` (0x556D2C); `m_nType = (u8)ropeType` (0x556D2E);
     if holder: `holder->RegisterReference(&r->m_pRopeHolder)` (0x556D37);
     `m_nTime = bExpires ? CTimer::m_snTimeInMilliseconds + timeExpire : 0` (0x556D3C-0x556D56);
     switch on m_nType (`type - 3` unsigned `> 4` -> default, 0x556D65-0x556D6D), writing `[r+0x308]` (length) and `[r+0x30C]` (spacing):
     | type | 0x308 | 0x30C | at |
     |---|---|---|---|
     | 3 (MAGNET) | 10.0 (0x41200000) | 0.32258064 (0x3EA5294A) | 0x556DA0 |
     | 4 (CRANE_MAGNO) | 50.0 (0x42480000) | 1.6129032 (0x3FCE739D) | 0x556D74 |
     | 5, 6, 7 (WRECKING_BALL, QUARRY_CRANE_ARM, CRANE_TROLLEY) | 68.0 (0x42880000) | 2.1935484 (0x400C6319) | 0x556D8A |
     | other (0, 1, 2, 8+) | 20.0 (0x41A00000) | 0.6451613 (0x3F25294A) | 0x556DB6 |
  5. Segment layout, L = f32 [r+0x30C], always for j = 1..31 (independent of segmentCount):
     - If `4 <= m_nType <= 7` (unsigned byte compares 0x556DCA/0x556DD2), hanging rope (0x556DDA-0x556FFD; unrolled x8 for j = 1..24
       (`cmp edx,0x19`), tail loop for j = 25..31): `seg[j].x = seg[j-1].x` (integer copy), `seg[j].y = seg[j-1].y` (fld/fstp),
       `seg[j].z = seg[j-1].z - L` (x87, stored f32). m_aSpeed[1..31] are NOT written (keep stale values).
     - Else, zig-zag rope (0x557015-0x5574FA; unrolled x8 for j = 1..24, tail loop 0x557480 for j = 25..31):
       `seg[j].x = (j odd) ? (L + seg[j-1].x) : (seg[j-1].x - L)` (0x55705A fadd / 0x557081 fsub; parity from `j & 1`), `seg[j].y`,
       `seg[j].z` copied via fld/fstp, `speed[j] = (0,0,0)`.
  6. `r->CreateHookObjectForRope()` (0x557000 / 0x557500); return 1.
- Edge cases: the re-register path keeps the old type/holder/timer and collapses segments 0..segmentCount onto startPos; segments above
  segmentCount keep their old positions. The new path ignores segmentCount for the layout. y/z copies through fld/fstp would quiet an
  SNaN (not reachable in practice).
- gta-reversed: `plugin::CallAndReturn` only (Ropes.cpp:73). The CRope member names at +0x304..+0x31C in gr Rope.h do not match how
  this function uses them (length at +0x308, spacing at +0x30C, holder-dependent 0.9/0.5 at +0x31C); ports should name fields by use.
  Open questions: meaning of +0x31C (0.9 for vehicle holders, else 0.5) - set at 0x556D04/0x556D10, consumer not examined here.
