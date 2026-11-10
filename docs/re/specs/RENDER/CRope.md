# CRope (RENDER) - full specs

CRope layout: see CRope.deviations.md (Rope.h names: +0x000 m_aSegments[32], +0x180 m_aSpeed[32], +0x304 m_fGroundZ,
+0x308 "m_fMass", +0x30C "m_fTotalLength", +0x310 m_pRopeHolder, +0x314 m_pAttachedEntity (the hook object),
+0x318 m_pRopeAttachObject (the carried entity), +0x31C "m_fSegmentLength", +0x320 m_nTime, +0x324 m_nSegments (u8),
+0x325 m_nType, +0x326 m_nFlags1 (u8 cooldown counter), +0x327 m_nFlags2 (bit0 0x01, bit2 0x04)). The bytes use +0x30C as the
rest length between two consecutive segments and +0x31C as the rope-extension value clamped to [0.01, 0.9] that the
UpdateWeightInRope / pickup code consumes; gr's names for +0x308/+0x30C/+0x31C do not describe this use (name note only).
Rope types: 1 CRANE_MAGNET1, 2 CRANE_HARNESS, 3 MAGNET, 4 CRANE_MAGNO, 5 WRECKING_BALL, 6 QUARRY_CRANE_ARM, 7 CRANE_TROLLEY.
`CRopes::PlayerControlsCrane` 0xB76898. Script switches: bEnableCraneRelease 0xA44494, bEnableCraneLower 0xA44495,
bEnableCraneRaise 0xA44496. Entity position "pos(e)" = `e->m_matrix ? &m_matrix->pos (+0x30) : &placement.pos (+4)`.
`top(e)` = `CMatrix::operator*(e->m_matrix, CVector(0, 0, c))` (0x59C890, full transform) with
`c = ms_modelInfoPtrs[(i16)e->+0x22]->m_pColModel (+0x14)->+0x14` (bounding box max.z).

## 0x5561B0 CRope::UpdateWeightInRope  (catalogue: specified/REA, port level 1)
- Convention: `__thiscall bool (CRope* this = ECX, CVector P (by value, [esp+4..0xC]), float unused [esp+0x10],
  CVector* out [esp+0x14])`, `ret 0x14`, AL. The float argument (CRope::Update passes 0.1f) is never read.
  gr's signature `(float, float, float, int32, float*)` matches the stack shape.
- Callers: CRope::Update 0x557530. Callees: CVector::Normalise 0x59C910. Globals: CTimer::ms_fTimeStep 0xB7CB5C.
- Notation: `n = m_nSegments (+0x324, u8)`, `L = +0x30C` (segment rest length), `A = seg[n]` (copied at entry),
  `len(v) = sqrt((v.x*v.x + v.z*v.z) + v.y*v.y)` (x87 order used at every distance in this function).
- Behaviour:
  1. `seg[31] = P` (0x5561F8-0x55621B, raw copy). `maxLen = (float)(31 - n) * L`; `d = P - A` (stored f32).
  2. If `len(d) < maxLen` (`fcom; test ah,5; jp` -> taut branch when not less, incl. NaN) - slack branch, 6 iterations
     (ebx = 6, 0x55626E):
     a. Backward pass (0x556280-0x5563FB): `j = 31`; while `j > n + 1`: `e = seg[j] - seg[j-1]`; if `!(len(e) > L)`
        (`test ah,0x41; jne`) stop this pass; else `old = seg[j-1]`, `k = L / len(e)`,
        `seg[j-1] = seg[j] + (seg[j-1] - seg[j]) * k` (x, y, z), `speed[j-1] = (seg[j-1] - old) * (1.0f / ts)` (products
        `inv*dx`, `dy*inv`, `dz*inv`), `j--`.
     b. Forward pass (0x556401-0x5564FD): for `j = n+1 .. 30`: `e = seg[j] - seg[j-1]`; if `len(e) > L`:
        `seg[j] = seg[j-1] + e * (L / len(e))` (the length recomputed from the stored dy/dz copies; no speed update).
     After the 6 iterations, final backward pass (0x55650A-0x55660A): `j = 31`; while `j > n + 1`: if `!(len(seg[j] -
     seg[j-1]) > L)` return 0; else `seg[j-1] = seg[j] + (seg[j-1] - seg[j]) * (L / len)`, `j--`. Return 0 (AL = 0; `*out`
     is not written).
  3. Taut branch (0x55662E): `k = maxLen / len(d)`; `*out = A + d*k` (x, y, z: `d.c*k + A.c`, stored f32). Then for
     `j = n+1 .. 31`: `Normalise(&d)`; `d *= L` (each component stored f32); `seg[j] = A + d * (float)(j - n)` (fild of the
     integer). `d` is re-normalised and re-scaled every iteration (so it stays length L up to rounding). Return 1.
- Edge cases: `n >= 30` skips the passes; ts == 0 gives inf speeds in pass a. The speed array is touched only in pass a.
- gta-reversed: Rope.cpp:91 `plugin::CallMethodAndReturn` (unreversed). Open questions: none.

## 0x557530 CRope::Update  (catalogue: specified/REA, port level 13)
- Convention: `__thiscall void (CRope* this = ECX)`, plain `ret`; SEH frame (two CMatrix locals).
- Callers: CRopes::Update 0x558D70. Callees: CRT `pow` 0x822130 (x87 (0.8, timeStep)), CWorld::FindGroundZFor3DCoord 0x5696C0,
  `rand`, CPad::GetPad 0x53FB70, CPad::GetCarGunUpDown 0x53FC10, CPad::CarGunJustDown 0x53FFE0,
  CAudioEngine::ReportMissionAudioEvent 0x5073B0 (thiscall on AudioEngine 0xB6BC90, (0x68, entity, 0, 1.0f)),
  CRope::UpdateWeightInRope 0x5561B0, CPhysical::GetSpeed 0x542CE0, CVector::Normalise 0x59C910, CMatrix copy ctor 0x59BCF0,
  CMatrix::ForceUpVector 0x59B7E0 (thiscall, CVector by value), CMatrix::operator= 0x59BBC0, ~CMatrix 0x59ACD0,
  CEntity::RegisterReference 0x571B70, CPlaceable::SetPosn 0x4241C0, CPed::IsPlayer 0x5DF8F0, CPhysical::AddToMovingList
  0x542800, CRope::ReleasePickedUpObject 0x556030, CRope::Remove 0x556780, `fmaxf`-like helper 0x420800 (cdecl (a, b) ->
  ST0 = a > b ? a : b), entity vtable +0x10 (SetIsStatic).
- Constants: 0.8 f64 [0x863E40], 200.0f [0x858A48], 0.0015f [0x863E38], 0.5f [0x858B8C], 1.0f [0x858624], 0.001f [0x858CDC],
  0.15f [0x858FCC], 0.3f [0x858C24], 1e-05f [0x858C14], 0.0f [0x858B50], 0.9f [0x858C20], 0.01f [0x858C58], 0.84f [0x863E34],
  750.0f [0x859A98], 0.00083333335f [0x863E30], 0.1f [0x858B1C] / imm 0x3DCCCCCD, 2.5f [0x858FA0], -50.0f [0x859B60];
  .data floats 0x8CD898 (= 0.06) and 0x8CD89C (= 0.13) (read at run time).
- Behaviour (blocks):
  1. Entry 0x557530-0x5575A2: `damp = (float)pow(0.8, ms_fTimeStep)` (stored f32). `C = TheCamera pos`
     (`[0xB6F03C] ? +0x30 : 0xB6F02C`); `d = sqrt(dx*dx + dy*dy)` with `dx = C.x - seg[0].x`, `dy = C.y - seg[0].y`;
     if `!(d < 200.0f)` return immediately (0x558CCE, the flag clear of step 9 is skipped).
  2. Free fall of segment 0 (0x5575A8-0x557609): if `!(flags2 & 1)` and `CTimer::m_snTimeInMilliseconds > m_nTime`
     (unsigned): `speed[0].z -= ts*0.0015f`; then `seg[0].x += ts*speed[0].x`, `.y += ts*speed[0].y`, `.z += ts*speed[0].z`
     (each `ts*v + p`, stored f32; ts = ms_fTimeStep 0xB7CB5C re-read).
  3. Ground (0x55760C-0x5576A7): if `flags2 & 4` and `(m_FrameCounter & 7) == 2`: `m_fGroundZ = FindGroundZFor3DCoord(seg0.x,
     seg0.y, seg0.z, 0, 0)`. If `flags2 & 4` and `m_pRopeAttachObject` is a vehicle or has model MI_OBJECTFORMAGNOCRANE1/2/3/5
     (0x8CD6C8/CC/D0/D8): `m_fGroundZ = pos(m_pAttachedEntity).z - 0.5f`.
  4. Segment chain (0x5576AD-0x557927), for `i = m_nSegments + 1 .. 31` (skipped when `m_nSegments + 1 >= 32`), with
     `om = 1.0f - damp`: save `old = seg[i]`;
     `speed[i].x += (float)((rand() & 0xF) - 8) * 0.001f`; then `speed[i].y` the same with a second rand();
     `speed[i] = damp*speed[i] + om*speed[i-1]` per component (z, y, x computed; stored x, y, z);
     `seg[i].z -= ts*0.15f`; if `flags2 & 4`: `seg[i].z = (seg[i].z <= groundZ + 0.3f) ? groundZ + 0.3f : seg[i].z`
     (`test ah,0x41`; NaN takes groundZ+0.3);
     `dv = seg[i] - seg[i-1]`, `k = (+0x30C) / sqrt((dx*dx + dy*dy) + dz*dz)`; `seg[i] = seg[i-1] + dv*k` (x, y, z);
     `speed[i] = (seg[i] - old) * (1.0f / ts)` (the reciprocal computed once per segment).
  5. Extension control (0x55792D-0x557BC7): types other than 1..7 go to step 9. Types 4/5/7/6 are "controlled" when
     PlayerControlsCrane == 1/2/3/4 respectively; uncontrolled types 4..7 skip to the clamp below. Types 1, 2, 3 always use
     the pad branch.
     - Pad branch (0x557B04): `s = GetPad(0)->GetCarGunUpDown()` (i16). If `(s < 0 && bEnableCraneRaise) || (s > 0 &&
       bEnableCraneLower)`: `L(+0x31C) = L - ((float)s * ts) * 1e-05f`. Then `L = min(L, 0.84f)`.
     - Controlled 4..7: if bEnableCraneRaise: `v = ((float)(i16)GetPad(0)->+0x1C * ts) * 1e-05f`; if `v > 0` and
       `v + L < 0.9f`: ReportMissionAudioEvent(0x68, m_pRopeHolder, 0, 1.0f); `L = v + L`. If bEnableCraneLower:
       `v = ((float)(i16)GetPad(0)->+0x20 * ts) * 1e-05f`; if `v > 0` and `L - v > 0.01f`: same audio event; `L = L - v`.
     - Clamp (all types 1..7): `L = max(L, 0.01f)` (NaN kept), then `L = min(L, 0.9f)`.
  6. Carried object coupling (0x557BC1-0x5581C7): `obj = m_pRopeAttachObject`. If obj: `mass = (obj model == 0x1AC) ? 750.0f :
     obj->m_fMass (+0x8C)`; `k = (f[0x8CD89C] * 0.00083333335f) * mass + f[0x8CD898]`; `if (0.5f < k) k = 0.5f`;
     `obj->+0xB8 = 0`. Else `obj = m_pAttachedEntity`, `obj->+0x1C |= 1` (m_bUsesCollision), `k = f[0x8CD898]`.
     If obj != 0:
     a. `if (UpdateWeightInRope(pos(obj) (by value), 0.1f, &np))`: `pos(obj) = np` (matrix or placement), then
        `V = obj->m_vecMoveSpeed (+0x44)`; `hs = m_pRopeHolder->GetSpeed(seg[0] - pos(holder))` (types 4..7: hs = 0);
        `V -= hs`; `dir = normalise(pos(obj) - seg[0])`; `dot = (dir.z*V.z + dir.x*V.x) + dir.y*V.y`; if `dot > 0`:
        `V -= dir*dot`; `D = (V + hs) - obj->m_vecMoveSpeed`; `obj->m_vecMoveSpeed += D*(1 - k)`;
        `holder->m_vecMoveSpeed -= D*k`.
        If `m_pRopeAttachObject != 0`: `m0 = *obj->m_matrix`; `obj->m_matrix->ForceUpVector(-dir)`; `m1 = *obj->m_matrix`;
        rows right/forward/up (not pos) of `obj->m_matrix = m0*0.9f + m1*0.1f` element-wise (x87 `(m0*0.9) + (m1*0.1)`).
        Else: `m_pAttachedEntity->m_matrix->ForceUpVector(-dir)`.
     b. If `m_pRopeAttachObject != 0` (0x558121): `*hook->m_matrix = *carried->m_matrix` (operator=), `hook->m_vecMoveSpeed =
        carried->m_vecMoveSpeed`, `pos(hook) = top(carried)`.
  7. With a carried object (0x5581EC-0x5582CF): if (controlled 4..7 or type 1/2/3) and bEnableCraneRelease and
     `GetPad(0)->CarGunJustDown()`: ReleasePickedUpObject(). If still carried and `carried->+0x40 & 0x20000000`: clear
     carried flags 0x2000000 and 0x80000000 (`+0x40`), `m_pRopeAttachObject = 0`, `hook->+0x1C |= 1`, `m_nFlags1 = 60`. If still
     carried, it is a vehicle with `+0x594 == 9` (bike) and `+0x460 != 0` (driver): ReleasePickedUpObject(). Go to step 9.
  8. Without a carried object (0x5582D4): if `m_nFlags1 != 0` { `m_nFlags1--`; go to step 9 }. Switch on type (table 0x558CE8):
     1 -> vehicle scan; 2 -> ped scan; 3 -> object flags {WONG_DISH, KMB_ROCK, KMB_PLANK, KMB_BOMB} + "RC only" then vehicle
     scan; 4 -> object flag MAGNOCRANE then vehicle scan; 5 -> nothing; 6 -> object flag QUARRY then vehicle scan;
     7 -> object flag TROLLEY, object scan only. `S = seg[31]` (this+0x174).
     - Vehicle scan (0x55835C): vehicle pool from the highest slot down (stride 0xA18, flag byte < 0 = free). Accept when
       (`+0x594 == 0`, or `== 9` with `+0x460 == 0`, or `== 1`, or model 0x1D9 or 0x21B) and `!(+0x40 & 0x20000000)` and
       `+0x42D & 0x10` and (not "RC only" or model 0x234) and `!(+0x4A8 & 0x60)`, and `|top(v) - S| < 2.5f` (sum order
       `(dx*dx + dz*dz) + dy*dy`). On accept: `m_pRopeAttachObject = v`, RegisterReference, `v->+0x40 |= 0x80000000`; if
       `(v->+0x36 & 0xF8) == 0x10` set status bits to 3 (`(& 7) | 0x18`); `pos(hook) = top(v)` (direct write);
       `hook->+0x1C &= ~1`; `dz = S.z - top.z` (from the first top); if `dz > 0`:
       `L = max0.01((L - dz*(1/(+0x308))) - (1/(+0x308))*(+0x30C))` (fst into L then `if !(L > 0.01) L = 0.01`). Stop scan.
       After the vehicle scan, if any object flag is set (types 3, 4, 6), run the object scan.
     - Ped scan (type 2, 0x5585A4): ped pool from the top (stride 0x7C4): skip free, `+0x530 == 0x37` (dead), players,
       `+0x46C & 0x100` (in vehicle); accept when `|pos(p) - S| < 2.5f` (`(dx*dx + dy*dy) + dz*dz`). Then attach as above
       (flag 0x80000000), `hook->SetPosn(top(p))`, `hook->+0x1C &= ~1`, and if `S.z - top.z > 0`:
       `L = 0x420800((L - dz*inv) - inv*(+0x30C), 0.01f)` (inv = 1/(+0x308); argument stored f32). Go to step 9.
     - Object scan (0x55876F, only if nothing carried): object pool from the top (stride 0x19C, flag & 0x80 = free); require
       `+0x140 & 0x40000` (bCanBeAttachedToMagnet) and model in the flagged set (MAGNOCRANE1-5 0x8CD6C8..D8; TROLLEY:
       MI_OBJECTFORBUILDINGSITECRANE1 0x8CD6DC; QUARRY: QUARY_ROCK1-3 0x8CD704/708/70C or DEAD_TIED_COP 0x8CD714, and only
       with `+0xFC == 0`; MAGNET: WONG_DISH 0x8CD724, KMB_ROCK 0x8CD734, KMB_PLANK 0x8CD738, KMB_BOMB 0x8CD73C).
       `P = top(o)`, `rot = 0`; for MAGNOCRANE1/2/3/5 also try `M*(0,0,-c)` (rot 2), `M*(c,0,0)` (rot 3), `M*(-c,0,0)` (rot 1),
       each replacing P when its z is greater. Accept when `|P - S| < 2.5f`: attach (flag 0x80000000), `hook->SetPosn(top(o))`,
       `hook->+0x1C &= ~1`; if `o->+0x1C & 4` (static) or `& 0x40000`: `o->SetIsStatic(false)` (vtable +0x10),
       `o->AddToMovingList()`; `o->+0x40 |= 0x2000000`; L update with the 0.01 floor as in the vehicle scan; then `rot` times:
       `old = right; right = up; up = -old` on `o->m_matrix` (0x558C32-0x558C9D). Stop scan.
  9. Exit (0x558CA3 -> stub 0x405FA5 -> 0x558CAE): if `!(flags2 & 1)` and `seg[0].z < -50.0f`: `Remove()`. Always
     `flags2 &= ~1`. Return.
- Edge cases: the controlled/uncontrolled split means types 4..7 never move L unless the player controls that crane, but
  the 0.01..0.9 clamp is always applied. The vehicle-scan L floor uses `!(L > 0.01)`, the ped one uses 0x420800 (same
  result except NaN).
- gta-reversed: Rope.cpp:196 `plugin::CallMethod` only. Confidence: medium - the block structure, constants and branch
  conditions are from the bytes; the exact x87 order inside steps 6a and 8 (object alternatives) was taken from the
  decompile cross-checked against the instruction stream only for the operations quoted with addresses.
- Open questions: 0x5579CB / 0x557A76 - which CPad fields are +0x1C / +0x20 (raise / lower analogue values, i16).
  0x5589A7, 0x558A35, 0x558ABC - operand order of the "higher z" test for the alternative pick points (assumed: replace when
  alternative z > current z, as in the decompile).
