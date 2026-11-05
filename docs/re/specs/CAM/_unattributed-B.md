# CAM unattributed / helper functions (group B)

Unnamed functions whose only callers are CCam methods (CCam is owned by group A); headings give the proposed class and
name. None of them is in gta-reversed.

## 0x512EF0 CCam::sub_512EF0 (proposed: arrest camera "cop head view" setup)  (catalogue: unknown/REA, port level 5)
- Attribution: only caller CCam::ProcessArrestCamOne 0x518500 (2 sites); `this` = ECX is a CCam (writes m_fFOV +0xB4,
  m_vecFront +0x190, m_vecSource +0x19C, m_vecUp +0x1B4). Not in gr.
- Convention: `__thiscall bool(CPed* ped [esp+4], bool checkAndRandom [esp+8])`, AL, `ret 8`. The `ped` slot is reused
  for the time fraction t.
- Callees: `rand` 0x821B1E, RwHelper::GetAnimHierarchyFromSkinClump 0x734A40, RpHAnimIDGetIndex 0x7C51A0,
  RpHAnimHierarchyGetMatrixArray 0x7C5120, CVector::Normalise 0x59C910, CVector::CrossProduct(out, a, b) 0x59C730,
  `CWorld::GetIsLineOfSightClear(const CVector& from, const CVector& to, bool buildings, bool vehicles, bool peds,
  bool objects, bool dummies, bool seeThrough, bool cameraIgnore)` 0x56A490 (cdecl, AL).
- Globals: `TheCamera.m_pTargetEntity` 0xB6F980 (read; its flags dword +0x1C modified), `CTimer::m_snTimeInMilliseconds`;
  f32 0xB6EC60 (.data, unnamed, catalogue "s_ExitCam+4": a start time in ms), f32 0xB70024 (static, unnamed);
  statics: byte 0xB70020 (toggle flag), init-guard dword 0xB7001C (bit0 -> vec 0xB70010 = (0,0,0); bit1 -> vec 0xB70004 =
  (0, 0, -0.5f)); .data tunables 0x8CCCA8 = 0.5, 0x8CCCAC = -0.06 (0xBD75C28F), 0x8CCCB0 = 1000.0.
  Constants 1/32767 = 3.0518509e-05f [0x858C7C], 0.65f [0x858F50], 1.0f [0x858624], 0.0f [0x858B50], 2^32 [0x858C54].
- Behaviour:
  1. If `checkAndRandom`: `if ((float)rand() * 3.0518509e-05f > 0.65f) return false;` (fild of the int; NaN impossible).
  2. `tgt = TheCamera.m_pTargetEntity`; `if ((tgt->byte[0x36] & 7) != 3 /*ped*/ || ped == nullptr) return false;`
  3. `t = (((float)(uint32)time - g_B6EC60) - g_B70024) / 1000.0f-global` (stored f32). Clamp + toggle:
     if `t > 1.0f`: `t = 1.0f` and `flag = !flag` (byte 0xB70020: written 0, then 1 only if it was 0);
     else if `t < 0.0f`: `t = 0`, `flag = 1`; else if `t < 1.0f` (or NaN): `flag = 1`; else (t == 1.0f exactly): toggle.
     The flag is not read by this function.
  4. Statics init (guard bits as above).
  5. `oldSrc = m_vecSource`; `m_vecSource = pos(tgt)` (m_matrix ? m_matrix->pos : placement.pos); `m_fFOV = 100.0f`.
  6. `h = GetAnimHierarchyFromSkinClump(ped->m_pRwClump (+0x18)); i = RpHAnimIDGetIndex(h, 5 /*head*/);
     head = RpHAnimHierarchyGetMatrixArray(h)[i].pos` (RwMatrix stride 0x40, pos +0x30);
     `L = (head.x, head.y, (k_m006 + head.z) - k_05 * t)` (x87: `fld -0.06; fadd head.z; fld 0.5; fmul t; fsubp`).
  7. `m_vecSource += off` (off = static (0,0,-0.5); `off.c + src.c` per component).
  8. `m_vecFront = L - m_vecSource`; `m_vecFront.Normalise()`; `m_vecUp = (0,0,1)`;
     `R = CrossProduct(m_vecFront, m_vecUp); R.Normalise(); m_vecUp = CrossProduct(R, m_vecFront)`.
  9. If `checkAndRandom`: `if (!GetIsLineOfSightClear(m_vecSource, L, true, true, false, true, false, false, true)) return false;`
     then `if (!GetIsLineOfSightClear(m_vecSource, L, true, false, false, true, false, false, false)) m_vecSource = oldSrc;`
  10. `tgt->m_nFlags (+0x1C) &= ~0x80` (clears bIsVisible on the target ped); return true.
- Oddities: on the first LOS failure the function returns false after having already modified source/front/up/FOV and
  without hiding the target. Step 9's second test can only restore the old source (the front/up keep pointing from the
  new one).
- gta-reversed: absent.
- Open questions: writers/meaning of 0xB6EC60 and 0xB70024 (0x512F57, 0x512F5D).

## 0x513220 CCam::sub_513220 (proposed: two-player "both players visible from point" test)  (catalogue: unknown/REA, port level 8)
- Attribution: only caller CCam::Process_Cam_TwoPlayer 0x525E50 (3 sites). Not in gr.
- Convention: callee-cleaned, one `CVector` **by value** (12 bytes, `ret 0xC`); ECX is overwritten on entry (so `this`
  is unused; equivalent to `__stdcall bool(CVector point)`). Result AL.
- Entry obfuscation: `sub esp,0x30` then `jmp 0x40110D` (HOODLUM-style stub) which computes
  `ECX = *(uint32*)([0x15605F4] + [0x701691])` = `*(CPed**)0xB7CD98` = `CWorld::Players[0].m_pPed`, and jumps back to
  0x513229. Treat as `ped0 = Players[0].m_pPed`.
- Callees: `CWorld::ProcessLineOfSight(const CVector& start, const CVector& end, CColPoint& cp, CEntity*& ent, bool buildings,
  bool vehicles, bool peds, bool objects, bool dummies, bool seeThrough, bool cameraIgnore, bool shootThrough)` 0x56BA00
  (cdecl, AL).
- Globals: `gCurCamColVars` 0x8CCB80 (u8) written; `CWorld::Players[1].m_pPed` 0xB7CF28.
- Behaviour:
  1. `ent = nullptr` (local); `gCurCamColVars = 5` (only the index byte; `gpCamColVars` is NOT updated).
  2. `if (ProcessLineOfSight(point, pos(ped0), cp, ent, true, false, false, false, false, true, true, false)) return false;`
     (pos = `m_matrix ? m_matrix->pos : placement.pos`).
  3. `return !ProcessLineOfSight(point, pos(Players[1].m_pPed), cp, ent, same flags);` (`neg al; sbb al,al; inc al`).
- Oddities: the side effect on gCurCamColVars without refreshing gpCamColVars (keep). No null checks on the peds.
- gta-reversed: absent.
- Open questions: none.

## 0x5132D0 CCam::sub_5132D0 (proposed: two-player camera source/front/target from both players)  (catalogue: unknown/REA, port level 1)
- Attribution: only caller CCam::Process_Cam_TwoPlayer 0x525E50 (4 sites); `this` = ECX is a CCam (reads
  `m_fVerticalAngle` +0xAC). Not in gr.
- Convention: `__thiscall void(float heading [esp+4], CVector* outSource [esp+8], CVector* outFront [esp+0xC],
  CVector* outTarget [esp+0x10])`, `ret 0x10`. The heading and outFront arg slots are reused as temps.
- Callees: CVector::Normalise 0x59C910 (thiscall, twice).
- Globals: `CWorld::Players[0].m_pPed` 0xB7CD98, `CWorld::Players[1].m_pPed` 0xB7CF28 (CPlayerInfo stride 0x190).
  Entity position = `m_matrix (+0x14) ? &m_matrix->pos (+0x30) : &placement.pos (+4)` (re-evaluated at every use).
- Constants: 0.67f [0x8631C8], 7.0f [0x858F48], 0.25f [0x858C84], 0.5f [0x858B8C], 1.0f [0x858624], 0.1f [0x858B1C].
- Behaviour (A = this->m_fVerticalAngle, H = heading; p0 = Players[0] ped pos, p1 = Players[1] ped pos):
  1. `f.x = -(cos(H) * cos(A))`, `f.y = -(sin(H) * cos(A))`, `f.z = sin(A)` (x87 `fcos`/`fsin` on the loaded f32; cos(A)
     kept on the stack; each result stored f32); `*outFront = f` (copied as dwords).
  2. `v = (f.x, f.y, 0)`; `v.Normalise()`.
  3. `d = sqrt((dz*dz + dy*dy) + dx*dx)` with `dx = p0.x - p1.x`, `dy = p0.y - p1.y`, `dz = p0.z - p1.z` (z term first);
     `R = d * 0.67f + 7.0f` (f32).
  4. `w = (p0.x - p1.x, p0.y - p1.y, 0)` (y difference computed first, both stored f32); `w.Normalise()`.
  5. `t = 0.5f - ((w.y*v.y + w.x*v.x) + v.z*w.z) * 0.25f` (f32).
  6. `s = 1.0f - t`; `outTarget = (t*p0.x + s*p1.x, t*p0.y + s*p1.y, t*p0.z + s*p1.z)` - the s-products are formed first
     (x, y stored f32, z kept), then the t-products; x = `t*p0.x + (s*p1.x)`, z = `(t*p0.z) + (s*p1.z)`; each stored f32.
  7. `outSource = (target.x - R*f.x, target.y - R*f.y, target.z - R*f.z)`, then `outSource.z = R*0.1f + outSource.z`.
     (`f` re-read from *outFront.)
- Oddities: no null checks on either player ped (Players[1].m_pPed must exist; only used in 2-player mode).
- gta-reversed: absent (Process_Cam_TwoPlayer is `.Reversed = false`).
- Open questions: none.

## 0x515D80 CCam::sub_515D80 (proposed: arrest-camera source position from an entity and a target)  (catalogue: unknown/REA, port level 1)
- Attribution: only caller CCam::ProcessArrestCamOne 0x518500 (4 sites). ECX is not read (overwritten at 0x515D8F), so
  `this` is unused; callee-cleans 0x10 (`ret 0x10`): treat as `__thiscall` on the CCam with an unused `this` (or
  `__stdcall`; equivalent for a port). Not in gr.
- Signature: `bool (const void* guard [esp+4], CEntity* ent [esp+8], const CVector* target [esp+0xC], CVector* outSource [esp+0x10])`,
  result AL (1 on success, 0 if `guard == nullptr || ent == nullptr`, 0x516003). `guard` is only null-tested.
- Callees: CVector::CrossProduct(out, a, b) 0x59C730 (cdecl, out = a x b), CVector::Normalise 0x59C910 (thiscall, 3 sites).
- Globals (.data f32, unnamed in gr, tunables): 0x8CC8C8 = 5.0 (back distance), 0x8CC8CC = 3.0 (side offset),
  0x8CC8D4 = 8.0 (minimum length). Constants: -0.7071f [0x8631E8] (also stored as the immediate 0xBF350481),
  1.4142271f [0x8631E4], 0.0f [0x858B50], 1.0f [0x858624].
- Behaviour (P = ent position: `m_matrix (+0x14) ? m_matrix->pos : placement.pos`, copied as dwords; T = *target):
  1. `D = T - P` (x, y, z, each f32). `C = CrossProduct(D, (0,0,1))`; `C.Normalise()`.
  2. `Q = (C.x*k3 + P.x, C.y*k3 + P.y, C.z*k3 + P.z)` with k3 = [0x8CC8CC] (products first, then `+ P`).
  3. `D.Normalise()`.
  4. If `D.z < -0.7071f` (NaN skips): `D.z = -0.7071f` (bit pattern 0xBF350481); `h = sqrt(D.y*D.y + D.x*D.x) * 1.4142271f`;
     if `h > 0.0f`: `r = 1.0f / h; D.x = D.x*r; D.y = r*D.y`; then `D.Normalise()` (also when h <= 0).
     Else if `D.z > 0.0f`: `D.z = 0`, `D.Normalise()`. Else (0 >= D.z >= -0.7071, or NaN): no normalise.
  5. `E = D * k5` (k5 = [0x8CC8C8]); `Q' = Q - E` (x, y, z); `F = T - Q'` (x: `T.x - Q'.x` with Q'.x stored f32 first;
     y and z use the x87 values of Q'.y / Q'.z, i.e. not re-rounded through memory - same result at 24-bit precision).
  6. `len = sqrt((F.z*F.z + F.y*F.y) + F.x*F.x)` (z first, stored f32 in the `guard` slot). If `len < k8` (k8 = [0x8CC8D4];
     `test ah,5; jp`: NaN and >= skip) and `len > 0.0f`: `s = k8 / len`; `F.x = F.x*s; F.y = F.y*s; F.z = s*F.z`.
  7. `*outSource = (T.x - F.x, T.y - F.y, T.z - F.z)`; return 1.
  Net: source = target - F where F = (target - P) - 3*side + 5*dir, stretched to at least length 8.
- gta-reversed: absent (ProcessArrestCamOne not reversed).
- Open questions: (1) the type of the first argument (only null-checked; probably the arresting cop ped) - check the
  4 call sites in 0x518500. (2) calling convention of the caller side (ECX value irrelevant).

## 0x516010 CCam::sub_516010 (proposed: arrest-camera side position near the ground)  (catalogue: unknown/REA, port level 9)
- Attribution: only caller CCam::ProcessArrestCamOne 0x518500. ECX not read; `ret 0x10`. Same argument shape as
  0x515D80: `bool (const void* guard [esp+4], CEntity* ent [esp+8], const CVector* target [esp+0xC], CVector* out [esp+0x10])`,
  result AL (0 if `guard` or `ent` is null, 0x51618A; else 1). The `guard` slot is reused as the `found` flag, and the
  `ent`/`target`/`out` slots as temps (the pointers stay in registers).
- Callees: CVector::Normalise 0x59C910, CVector::CrossProduct(out, a, b) 0x59C730,
  `CWorld::FindGroundZFor3DCoord(CVector pos /*by value*/, bool* found, CEntity** outEnt)` 0x5696C0 (cdecl, ST0).
- Globals (.data f32 tunables, unnamed): 0x8CC7F0 = 5.0, 0x8CC7F4 = 10.0, 0x8CC7F8 = 0.7; constant 5.0f [0x858C80].
- Behaviour (P = ent position via m_matrix/placement, T = *target):
  1. `D = (T.x - P.x, T.y - P.y, 0)`; `D.Normalise()`.
  2. `*out = (k5*D.x + T.x, D.y*k5 + T.y, k5*D.z + T.z)` (k5 = [0x8CC7F0]; products then `+ T`, f32 stores).
  3. `C = CrossProduct(D, (0, 0, 1))`; `out.x = k10*C.x + out.x; out.y = k10*C.y + out.y; out.z = k10*C.z + out.z`
     (k10 = [0x8CC7F4]).
  4. `out.z = T.z + 5.0f` (overwrites step 3's z).
  5. `found = false; g = FindGroundZFor3DCoord({out.x, out.y, out.z}, &found, nullptr)`; if `found`: `out.z = 0.7f-global
     [0x8CC7F8] + g`. Return 1.
- gta-reversed: absent.
- Open questions: type of `guard` (see 0x515D80).

## 0x5B2090 CCam::sub_5B2090 (proposed: cutscene spline position evaluator)  (catalogue: unknown/REA, port level 0)
- Attribution: only caller is CCam::Process_FlyBy 0x5B25F0 (4 sites; group A's class), no `this`; it reads
  `TheCamera.m_vecCutSceneOffset`. Free helper used by the fly-by (cutscene path) camera; name not in gr.
- Convention: `__cdecl void(CVector* out [esp+4], const float* spline [esp+8], float timeMs [esp+0xC], uint32* idx [esp+0x10])`,
  plain `ret`. The `spline` argument slot is reused for a temp (endT).
- Callees: `_ftol2` 0x821B40. Globals: f32 0x8D0F80 (.data, initial 32.0; catalogue labels it
  "CCutsceneObject::ms_sCutsceneVehNames+0x18", which is a mislabel of an unnamed float), `TheCamera.m_vecCutSceneOffset`
  0xB6F8AC/0xB6F8B0/0xB6F8B4. Constants 1000.0f [0x858C4C], 1.0f [0x858624], 0.0f [0x858B50], 3.0f [0x858B3C].
- Data layout (gr `CCamPathSplines` float array): `spline[0]` = node count n (float); node k occupies 10 floats at
  `spline[1 + 10k ..]`: +0 time (seconds), +1..3 position, +4..6 "in" control point, +7..9 "out" control point.
  `*idx` is the float index of the time slot of the current (end) node of the active segment (i = 10k+1).
- Behaviour:
  1. `n = _ftol2(spline[0])` (truncation). `i = *idx`. `segDur = (spline[i] - spline[i-10]) * 1000.0f` (kept on the x87
     stack). `endT = spline[10n-9] * 1000.0f` (stored f32).
  2. If `timeMs < endT` (`test ah,5; jp`; NaN/>= skip):
     `q = (uint32)(i-1) / 10`; `ok = true`; if `q > n` (unsigned): `*idx = 10n - 9; ok = false`.
     If `segDur <= g_8D0F80` (`fcom; test ah,0x41; jp`: NaN fails) and `ok`: `*idx += 10`; then if `(uint32)(*idx-1)/10 > n`:
     `*idx = 10n - 9`.
  3. `i = *idx` (new). `u = (timeMs - spline[i-10]*1000.0f) / segDur` - **segDur is the value from step 1 (old index)**,
     the subtrahend uses the new index (0x5B212A-0x5B2138).
  4. Clamp: `if (u > 1.0f) u = 1.0f; else if (u < 0.0f) u = 0.0f;` (NaN stays NaN). Then `if (timeMs > endT) u = 1.0f`.
  5. `P0 = spline[i-9..i-7]` (previous node pos), `P1 = spline[i+1..i+3]` (next node pos), `C0 = spline[i-3..i-1]` (previous
     node out-control), `C1 = spline[i+4..i+6]` (next node in-control).
  6. If `C0.x == P0.x && C0.y == P0.y && C0.z == P0.z` (exact compares, NaN = not equal): linear,
     `out.c = (P1.c - P0.c) * u + P0.c` for c = x, y, z (0x5B22C4-0x5B22F7).
     Else cubic Bezier (0x5B220C-0x5B22BF): `uu = u*u; b3 = uu*u` (f32 store); `v = 1.0f - u; vv = v*v` (f32 store);
     `b0 = vv*v` (f32 store); `b2 = v*uu` (f32 store); `b1 = u*vv` (kept on the x87 stack); per component:
     `out.c = (((b2*C1.c + b1*C0.c) * 3.0f) + b3*P1.c) + b0*P0.c` (for z the b1*C0.z product is formed first, then
     b2*C1.z is added; same value).
  7. `out.x = offset.x + out.x; out.y = offset.y + out.y; out.z = offset.z + out.z` (TheCamera.m_vecCutSceneOffset).
- Oddities (keep): step 3 mixes the old segment duration with the new segment start; the clamp only triggers when
  `q > n`, so `q == n` (one node past the end) is not clamped; the advance in step 2 depends on a writable global
  threshold (32.0 ms by default).
- gta-reversed: absent (Process_FlyBy is `.Reversed = false`).
- Open questions: (1) 0x5B20F3 meaning/writers of the float 0x8D0F80 (not found in gr; only these two readers in the
  rwsets). (2) whether CCamPathSplines time units are seconds (the *1000 suggests so; behaviour does not depend on it).

## 0x5B2330 CCam::sub_5B2330 (proposed: cutscene spline scalar evaluator)  (catalogue: unknown/REA, port level 0)
- Attribution: as 0x5B2090 (only caller CCam::Process_FlyBy 0x5B25F0, 4 sites). Used for the 1-D tracks (roll / FOV).
- Convention: `__cdecl void(float* out [esp+4], const float* spline [esp+8], float timeMs [esp+0xC], uint32* idx [esp+0x10])`,
  plain `ret`.
- Data layout: `spline[0]` = n; node k = 4 floats at `spline[1 + 4k]`: +0 time, +1 value, +2 in-control, +3 out-control.
- Behaviour:
  1. `n = _ftol2(spline[0])`; `i = *idx`; `segDur = (spline[i] - spline[i-4]) * 1000.0f`; `endT = spline[4n-3] * 1000.0f`.
  2. If `timeMs < endT`: `ok = true`; if `((uint32)(i-1) >> 2) > n`: `*idx = 4n-3; ok = false`. Then (fcomp pops segDur)
     if `segDur <= g_8D0F80 && ok`: `*idx += 4`; if `((uint32)(*idx-1) >> 2) > n`: `*idx = 4n-3`.
     Else segDur is popped (0x5B23B4).
  3. `i = *idx`; `u = (timeMs - spline[i-4]*1000.0f) / ((spline[i] - spline[i-4]) * 1000.0f)` - unlike 0x5B2090 the duration
     is **recomputed with the new index** (0x5B23B8-0x5B23D3).
  4. Same clamp as 0x5B2090 (`>1 -> 1`, else `<0 -> 0`; then `timeMs > endT -> 1`).
  5. `v = 1.0f - u; vv = v*v; vvv = vv*v; A = spline[i-1]` (previous out-control), `B = spline[i+2]` (next in-control),
     `P0 = spline[i-3]`, `P1 = spline[i+1]`.
  6. If `A == P0`: `*out = (P1 - P0) * u + P0`. Else (Horner form, 0x5B243D-0x5B246E):
     `*out = (((P1*u + (B*v)*3.0f) * u + (A*vv)*3.0f) * u) + vvv*P0`.
     (No cutscene offset is added.)
- gta-reversed: absent.
- Open questions: same global 0x8D0F80 as above (0x5B2388).

