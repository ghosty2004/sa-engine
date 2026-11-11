# CTaskSimpleFight (AI) - RE-SPEC-AI-2 group J3

Vtable 0x86D684. Layout (gr TaskSimpleFight.h): +0x8 m_bIsFinished u8, +0x9 m_bIsInControl u8, +0xA m_bAnimsReferenced u8, +0xC required anim
group (AssocGroupId i32, 0x21 = none), +0x10 m_nIdlePeriod u16, +0x12 m_nIdleCounter u16, +0x14 m_nContinueStrike i8, +0x15 m_nChainCounter i8,
+0x18 m_pTargetEntity (registered ref), +0x1C m_pAnim, +0x20 m_pIdleAnim, +0x24 m_nComboSet i8, +0x25 m_nCurrentMove i8, +0x26 m_nNextCommand u8,
+0x27 m_nLastCommand u8.

## 0x5BEDC0 CTaskSimpleFight::LoadMeleeData  (catalogue: specified/REA, port level 2) - RE-SPEC-AI-2 (lead)
- Convention: `__cdecl void ()` (static). Frame `sub esp,0x80` + push ebx/ebp/esi/edi; the epilogue is split: 0x5BF2E8 `jmp 0x40204B`
  (`add esp,0x80`; `jmp 0x5BF2ED` = `ret`). Below "L+n" = local at (esp after the four pushes) + n.
- Callers: CGame::Init1 0x5BF840. Callees: CFileMgr::OpenFile 0x538900 ("DATA\\melee.dat" [0x86AAB4], "rb" [0x859C6C]),
  CFileLoader::LoadLine(FILE*) 0x536F80, `_strncmp` 0x8214D0, `_sscanf` 0x8220AD, CAnimManager::GetAnimGroupName 0x4D3A20,
  CFileMgr::CloseFile 0x5389D0, 0x5BD360 (hit-level letter parser, below), 0x5BD3B0 (hit-anim id mapper, below).
- Globals: CTaskSimpleFight::m_aComboData 0xC170D0 (W; 13 entries of CMeleeInfo, stride 0x88; loop end test `entry+4 < 0xC177BC`),
  a 7 x 12-byte float triple table at 0xC177D0..0xC17823 (W; read by CTaskSimpleFight::ProcessPed 0x629920 and 0x62A380; no gr name;
  proposed `CTaskSimpleFight::ms_aHitLevelOffsets` - name UNCLEAR), CAnimManager::ms_numAnimAssocDefinitions 0xB4EA28 (R).
- CMeleeInfo layout as the bytes use it (differs from gr TaskSimpleFight.h:23 at +0x50..+0x5B, see gta-reversed below):
  +0 i32 anim group, +4 f32 range, +8 f32 hit[5], +0x1C f32 chain[5], +0x30 f32 radius[5], +0x44 f32 ground loop, +0x48 f32 block hit,
  +0x4C f32 block chain, +0x50 u8 hitLevel[5], +0x55 u8 damage[5], +0x5C i32 hitAnim[5], +0x70 i32 altHitAnim[5], +0x84 u16 flags.
- Behaviour:
  1. Defaults (0x5BEDC0-0x5BEE36), for each of the 13 entries: +0 = 0x21; +4 = 1.5f (imm 0x3FC00000); for i = 0..4: hit[i] = 100.0f
     (imm 0x42C80000), chain[i] = 100.0f, radius[i] = 1.0f (imm 0x3F800000), hitLevel[i] = 7, damage[i] = 0, hitAnim[i] = 0,
     altHitAnim[i] = 0; then +0x44 = 0 (i32 0 = 0.0f), +0x48 = 100.0f, +0x4C = 100.0f, +0x84 = 0 (u16).
  2. Level table defaults (0x5BEE38-0x5BEF41): each of the 7 triples at 0xC177D0 + 12*k = (0.0f, 0.75f (imm 0x3F400000), 0.0f).
     Stack locals L+0x44/L+0x48/L+0x4C (the sscanf targets of step 6) end up 0, 0.75f, 0 by the interleaved stores at 0x5BEE38/0x5BEEDF/
     0x5BEF2E-0x5BEF32 (a level line with missing fields keeps these, later the previous line's values).
  3. f = OpenFile("DATA\\melee.dat", "rb") (no null check before LoadLine). State: inCombo (byte L+0x12) = 0, inLevels (L+0x13) = 0,
     comboDw (L+0x18, dword index of the current entry, step 0x22) = 0, ebp (byte offset of the current entry, step 0x88) = 0,
     ebx (line counter / level index) = 0, column (L+0x14) = 0.
  4. Per line s = LoadLine(f) until NULL (0x5BEF63/0x5BF2C8): skip if s[0] == '#' (0x23) or s[0] == 0 (0x5BEF82-0x5BEF8C).
     If strncmp(s, "END_MELEE_DATA" [0x86AAA4], 14) == 0 -> stop (0x5BEFA4).
  5. Outside a block (both flags 0): strncmp(s, "START_COMBO" [0x86AA98], 11) == 0 -> inCombo = 1; else strncmp(s, "START_LEVELS"
     [0x86AA88], 12) == 0 -> inLevels = 1; any other line is ignored (0x5BEFAA-0x5BEFF5).
  6. Inside a block: strncmp(s, "END_COMBO" [0x86AA7C], 9) == 0 -> if inCombo { comboDw += 0x22; ebp += 0x88 }; ebx = 0; inCombo =
     inLevels = 0 (0x5BEFFA-0x5BF02B; END_COMBO also closes a levels block). Else if inLevels: sscanf(s, "%s %f %f %f" [0x86AA70],
     L+0x50, L+0x44, L+0x48, L+0x4C); triple[ebx] = (L+0x44, L+0x48, L+0x4C) raw (0x5BF057-0x5BF075); ebx++ (no bound check).
     Else (in combo): k = ebx; ebx++; jump table 0x5BF2F0 on k (k > 8 ignored):
     - k = 0 (0x5BF092): sscanf(s, "%s %s" [0x869860], L+0x50, L+0x70); for g = 0 .. ms_numAnimAssocDefinitions-1: if
       strcmp(L+0x70, GetAnimGroupName(g)) == 0 (inline case-sensitive compare 0x5BF0D0) { entry.+0 = g; break } (no match: unchanged).
     - k = 1 (0x5BF117): sscanf(s, "%s %f" [0x86AA68], L+0x50, L+0x28); entry.+4 range = L+0x28 raw (not scaled).
     - k = 2..6 (0x5BF13B): L+0x1C = 0 first (0x5BF171); sscanf(s, "%s %f %f %f %s %d %d %d %f" [0x86AA4C], L+0x50, L+0x20, L+0x24,
       L+0x2C, L+0x70, L+0x30, L+0x34, L+0x38, L+0x1C). column i = k-2 via jump table 0x5BF314 on (ebx-3) (0x5BF17E-0x5BF1B8).
       hit[i] = L+0x20 * (1/30) 0.033333335f [0x858F10] (fld, fmul, fstp f32, 0x5BF1C4-0x5BF1DC); radius[i] = L+0x2C raw dword
       (0x5BF1F1); chain[i] = L+0x24 * 0.033333335f [0x858F10] (0x5BF1E6-0x5BF1F7); hitLevel[i] = 0x5BD360(L+0x70) (0x5BF206);
       damage[i] = low byte of L+0x30 (0x5BF212); hitAnim[i] = (i32)(i16)0x5BD3B0(L+0x34) (0x5BF226); altHitAnim[i] =
       (i32)(i16)0x5BD3B0(L+0x38) (0x5BF241); if L+0x1C > 0.0f [0x858B50] (`fcomp; test ah,0x41; jne` = skip when <= 0 or NaN):
       entry.+0x44 ground loop = L+0x1C * 0.033333335f [0x858F10] (0x5BF24E-0x5BF258).
     - k = 7 (0x5BF260): sscanf(s, "%s %f %f" [0x86AA40], L+0x50, L+0x20, L+0x24); +0x48 = L+0x20 * 0.033333335f [0x858F10];
       +0x4C = L+0x24 * 0.033333335f [0x858F10] (0x5BF27A-0x5BF297).
     - k = 8 (0x5BF29F): sscanf(s, "%s %x" [0x86AA38], L+0x50, L+0x3C); +0x84 = (u16)L+0x3C.
  7. CloseFile(f) (0x5BF2DB).
- 0x5BD360 (cdecl `u8 (const char* s)`, proposed `CTaskSimpleFight::GetHitLevelFromString`): first char 'H' -> 0, 'L' -> 1, 'G' -> 2,
  'B' -> 3, anything else -> 7. The code then compares the ZERO-EXTENDED FIRST BYTE with 0x484C / 0x4C4C / 0x474C (intended
  two-letter codes for values 4/5/6, 0x5BD382-0x5BD3A1); those can never match, so the final `setne al; add al,6` always gives 7.
  Original bug: keep (4, 5, 6 are unreachable).
- 0x5BD3B0 (cdecl `i32 (i32 n)`, proposed `CTaskSimpleFight::GetHitAnimId`): jump table 0x5BD3F4 on n-1 (unsigned, n-1 > 7 -> 0x3E):
  1 -> 0x3D, 2 -> 0x3E, 3 -> 0x3F, 4 -> 0x40, 5 -> 0x41, 6 -> 0x42, 7 -> 0x43, 8 -> 0x44, else 0x3E. Caller sign-extends AX.
- Edge cases: combo/level indices are not bounded (more than 13 combos or 7 level lines overflow into following .data). The anim-group
  search reads ms_numAnimAssocDefinitions each iteration. Hit lines store radius unscaled but hit/chain scaled by 1/30 (frames -> s).
- gta-reversed: plugin::Call (TaskSimpleFight.cpp:36), not reversed. gr's CMeleeInfo (TaskSimpleFight.h:23) declares `uint8 m_nHitLevel;
  int32 m_nDamage; int32 field_58;` at +0x50/+0x54/+0x58; the bytes use u8[5] at +0x50 and u8[5] at +0x55 (0x5BEDF8/0x5BEDFC, 0x5BF206/
  0x5BF212). Total size 0x88 matches. Layout error in gr.
- Open questions: the meaning/name of the triple table 0xC177D0 (users 0x629920, 0x62A380).

## 0x61C470 CTaskSimpleFight::CTaskSimpleFight  (catalogue: specified/REA, port level 2)
- `__thiscall CTaskSimpleFight* (this, CEntity* target [esp+4], int32 command [esp+8], u32 idlePeriod [esp+0xC])`, `ret 0xC`, SEH frame.
- CTaskSimple::CTaskSimple 0x61A390; vtbl = 0x86D684; +0x24 = +0x25 = 0xFF (0x61C49E-0x61C4A1); +0x8 = 0; +0x9 = 1; +0xA = 0; +0xC = 0x21;
  +0x12 (u16) = 0; +0x14 = +0x15 = 0; +0x18 = target; +0x1C = +0x20 = 0; +0x26 = (u8)command; +0x27 = 0 (0x61C4AF-0x61C4DB).
- If target: `target->RegisterReference(&this->+0x18)` 0x571B70 (0x61C4E3). +0x10 (u16) = `min(idlePeriod, 60000)` (unsigned `cmp 0xEA60; jbe`,
  0x61C4EC-0x61C4FC). Returns this. gr: plugin::Call (cpp:7).

## 0x61C520 CTaskSimpleFight::GetTaskType  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`: `mov eax, 0x3F8` (0x61C520); `ret` -> returns 1016 (TASK_SIMPLE_FIGHT). Vtable: slot 4 (+0x10) of vtable 0x86D684 (CTaskSimpleFight).

## 0x61C530 CTaskSimpleFight::~CTaskSimpleFight  (catalogue: specified/REA, port level 1)
- Non-deleting dtor, `__thiscall void (this)`, SEH frame, order: vtbl = 0x86D684; if +0x18 target: `CleanUpOldReference(&+0x18)` 0x571A00;
  if +0x1C anim: `SetDeleteCallback(DefaultAnimCB 0x4D1390, nullptr)` 0x4CEBC0; same for +0x20 (0x61C57E-0x61C58C);
  if `+0xA != 0 && +0xC != 0x21` (0x61C591-0x61C59E): `CAnimManager::RemoveAnimBlockRef((ms_aAnimAssocGroups 0xB4EA34)[+0xC].m_pAnimBlock -
  ms_aAnimBlocks 0xB5D4A0) >> 5)` 0x4D3FD0 (group stride 0x14, block size 0x20) and `+0xA = 0`; then CTaskSimple::~CTaskSimple 0x61A3A0.
- gr: plugin::Call (cpp:12).

## 0x61C5E0 CTaskSimpleFight::ControlFight  (catalogue: specified/REA, port level 2)
- `__thiscall bool (this, CEntity* target [esp+4], u8 command [esp+8])`, `ret 8`. Always returns AL = 1 (0x61C61D).
- `+0x9 = 1` (0x61C5F1). If target != +0x18 (0x61C5EC): if old non-null `CleanUpOldReference(&+0x18)` 0x571A00; `+0x18 = target`; if non-null
  `target->RegisterReference(&+0x18)` 0x571B70. If `(i8)command > (i8)+0x26` (signed `jle`, 0x61C613) then `+0x26 = command`.
- gr: plugin::Call (cpp:17).

## 0x6239D0 CTaskSimpleFight::~CTaskSimpleFight  (catalogue: specified/REA, port level 2)
- Scalar deleting dtor, `__thiscall void* (this, u32 flags [esp+4])`, `ret 4`: calls dtor CTaskSimpleFight::~CTaskSimpleFight 0x61C530 (0x6239D3); if `flags & 1` (0x6239D8) calls CTask::operator delete 0x61A5B0 (this) (0x6239E0, cdecl, caller pops); returns this in EAX. Vtable: slot 0 (+0x0) of vtable 0x86D684 (CTaskSimpleFight).

## 0x629920 CTaskSimpleFight::ProcessPed  (catalogue: specified/REA, port level 20)
- `__thiscall bool (this, CPed* ped [esp+4])`, `ret 4`; ESI = this, EBX = ped, EBP = 0. Returns AL = 1 only from the finished path, else 0 (0x62A08A).
- Combo table `M = &CTaskSimpleFight::m_aComboData[(i8)+0x24 - 4]` (0xC170D0, stride 0x88; CMeleeInfo: +0 anim group, +0x8 f32 hit time[5],
  +0x1C f32 chain time[5], +0x48 f32 block-hit loop start, +0x4C f32 block-chain loop end, +0x50 u8 hit level[5], +0x5C i32 hit sound[5],
  +0x85 flag byte (bit 2 tested)). gr types +0x48/+0x4C as int32 but the bytes load them as f32 (0x629D26, 0x629D43).
- Helpers (unnamed, proposed names, see _unattributed-J4.md): 0x61C7F0 GetAvailableComboSet(ped, i8 cmd) -> i8, 0x61C9B0 (ped) (player
  commands 3..6), 0x624710 ChooseAttackPlayer(ped) -> i16, 0x624A40 ChooseAttackAI(ped) -> i16, 0x623B10 StartAnim(ped, i32 move),
  0x6240B0 FightStrike(ped, CVector&), 0x61DAE0 FinishMeleeAnimCB (delete callback).
- Block 1, finished (+0x8 != 0, 0x62992E): if +0x20 (idle anim) != 0: `SetDeleteCallback(DefaultAnimCB 0x4D1390, 0)`; if `idle->m_fBlendAmount (+0x18)
  > 0.0 [0x858B50]` and NOT `idle->m_fBlendDelta (+0x1C) < 0.0` (`test ah,1`): `BlendAnimation(ped->m_pRwClump, ped->+0x4D4, 3, 8.0f)`; `+0x20 = 0`.
  Return 1 (0x629987).
- Block 2, idle counter (0x629990): if `+0x24 != 0 && +0x9 != 0`: `+0x12 = 0`; else `+0x12 += (u16)_ftol(ms_fTimeStep * 0.02 [0x858B38] * 1000.0
  [0x858C4C])` (0x6299A4-0x6299BB, 0x821B40 truncation).
- Block 3, not in control (+0x9 == 0, 0x6299C4): if `(i8)+0x26 < 1` -> exit (0x62A089, returns 0 WITHOUT clearing +0x9); if `+0x1C != 0 &&
  anim->m_nAnimId (+0x2C, i16) == 0x2F` -> exit likewise.
- Block 4, no idle anim yet (+0x20 == 0, 0x6299ED): if `+0x27` in {1, 0xF, 0x10, 0x11}: `+0x8 = 1` (finish next frame). Else if +0x1C == 0 and
  (`ped->m_nMoveState (+0x534) <= 4` or `!ped->IsPlayer()`): combo set `c = GetWeaponInfo(activeWeaponType, 1)->+0x6E` (0x629A50-0x629A55); if c == 4 then
  `c = ped->+0x72D` (fighting style). If `M[c].+0x85 & 4` and `M[c].group != 0x21`: (re)reference the anim block of M[c].group: if it differs from +0xC,
  release the old one when +0xA (RemoveAnimBlockRef) and set +0xC; if the block (ms_aAnimAssocGroups[+0xC] block pointer, or GetAnimationBlock(
  GetAnimBlockName(+0xC)) when null) is loaded (`block->+0x10`): AddAnimBlockRef, `+0xA = 1`, keep c; else `CStreaming::RequestModel(blockIndex +
  0x63E7, 8)` and, if `11 <= (i8)+0x26 <= 14`, `+0x26 = 11`; c = 4 (0x629B22-0x629B42). If the flag bit is clear or the anim is not ready, c = 4.
  Then `+0x24 = c`; `+0x20 = BlendAnimation(clump, M[c].group, 0xDF, 4.0f)`; `+0x20->SetDeleteCallback(0x61DAE0, this)`; `ped->SetMoveState(1)`;
  `ped->+0x538 = 1`; `+0x24 = 0`; `+0x27 = 0` (0x629B44-0x629B97).
- Block 5 (0x629B9B): if `+0xC != 0x21 && +0xA == 0`: `GetAvailableComboSet(ped, -1)` (result discarded).
- Block 6, no attack anim (+0x1C == 0, 0x629BB2): for the player, if `+0x12 > +0x10` (u16) and +0x26 == 0 and +0x24 == 0: `+0x26 = 1`. If +0x26 == 0
  and +0x24 == 0, or +0x27 == 0x10 -> block 8. Else `+0x24 = GetAvailableComboSet(ped, +0x26)`; player: if `3 <= +0x26 <= 6` call 0x61C9B0(ped),
  else `StartAnim(ped, (i16)ChooseAttackPlayer(ped))`; AI: `StartAnim(ped, (i16)ChooseAttackAI(ped))` (0x629BFE-0x629C49 -> 0x629FEF).
- Block 7, attack anim running (0x629C4E), M = combo of +0x24, anim A = +0x1C: if the active weapon type == 9 (chainsaw):
  `ped->m_weaponAudio (+0x394).AddAudioEvent(0x9A)` 0x4E69F0.
  a. `(i8)+0x24 < 4` (0x629C87): if +0x27 is 8 or 10 and `A->m_fCurrentTime (+0x20) / A->m_pHierarchy(+0x14)->m_fTotalLength (+0x10) > 0.4 [0x8D2E94]`
     and `A->+0x1C > -4.0 [0x859C3C]`: `A->+0x1C = -4.0f` (immediate). Else if +0x27 == 3: if +0x26 == 3 then `+0x26 = 0`; else if `A->+0x1C > -4.0`
     then `A->+0x1C = -4.0f`.
  b. +0x27 == 2 (block, 0x629D09): if +0x26 == 2 and `A->+0x2E & 1` (looped): if NOT(time >= M.+0x48 ... ) - exact: `if (!(A.t < M.48) && (A.t + A.dt
     (+0x28) < M.48))` or `(!(A.t < M.4C) && (A.t + A.dt < M.4C))` is false -> skip; when either window test passes (0x629D23-0x629D5B) clear the loop flag
     `A->+0x2E &= ~1` and `A->SetCurrentTime(M.+0x48)` 0x4CEA80. If +0x26 != 2: if A not looped and `A->+0x18 > 0.0` and NOT `A->+0x1C < 0.0` then
     `A->+0x1C = -4.0f`; if `(i8)+0x26 >= 11`: `A->SetDeleteCallback(DefaultAnimCB, 0)`, `+0x1C = 0`. Finally if +0x26 == 2 then `+0x26 = 0`.
  c. Otherwise (attack, 0x629DCD): only if `A->+0x18 > 0.9 [0x858C20]` and NOT `A->+0x1C < 0.0`; m = (i8)+0x25:
     - Strike window: `A.t > M.hit[m]` and NOT `(A.t - A.dt) > M.hit[m]` (`test ah,5; jp`): `ped->m_pedAudio (+0x138).AddAudioEvent(0x3C, 0, 1.0f, 0, 0,
       M.hitSound[m], 0)`; if +0x24 == 5 and 0 <= m <= 2 a second event with `[0x8D2E3C + 4*m]` as last arg; if `11 <= +0x26 <= 14` `+0x26 = 0`;
       if `M.hitLevel[m] != 7`: `p = ped->m_matrix * table[0xC177D0 + 12*hitLevel]` (CMatrix*CVector 0x59C890); if m == 4: `p += ped->m_vecMoveSpeed *
       ms_fTimeStep` (0x40FEC0 then 0x411A00); `FightStrike(ped, p)` 0x6240B0.
     - Else if NOT `A.t < M.chain[m]` and `11 <= +0x26 <= 14`: m 0/1 -> `+0x24 = GetAvailableComboSet(ped, +0x26)` then StartAnim(ped, player ?
       ChooseAttackPlayer(ped) : m + 1); m == 3 -> player only: if ChooseAttackPlayer(ped) == 3 then StartAnim(ped, 3); m == 4 -> if +0x24 == 12 and player:
       `A->SetCurrentTime(M.hit[m] - 0.01 [0x858C58])`; else same as m 0/1 with `(u8)+0x26`.
- Block 8, heading (0x629FF7): if target +0x18: `ped->+0x55C (m_fAimingRotation) = atan2(-(t.x - p.x), t.y - p.y)` (`fchs` on dx, `fpatan`);
  else if player && CCamera::m_bUseMouse3rdPerson 0xB6EC2E && `GetPadFromPlayer()->GetTarget()`: `= atan2(-TheCamera+0x304, TheCamera+0x308)` (0xB6F32C,
  0xB6F330; 0xB6F334 copied but unused). Then `+0x9 = 0` (0x62A084), return 0.
- gta-reversed: plugin::Call (cpp:49). Open questions: exact semantics of the window tests in 7b (0x629D23-0x629D5B) are given above as compares
  (`test ah,5; jp` = "not less" / `test ah,1; je` = "not below"); the port should transcribe the compare chain literally.

## 0x4ABDA0 CTaskSimpleFight::GetComboAnimGroupID  (catalogue: specified/REA, port level 0)
- `__thiscall int32 ()`, plain `ret`. i = (int32)(int8)this+0x24 (m_nComboSet) - 4; if i < 0 then i = 0 (branchless `setl; dec; and`,
  0x4ABDA4-0x4ABDAF); returns m_aComboData[i].m_nAnimGroup = dword [0xC170D0 + i*0x88] (no upper bound check).

## 0x4ABDC0 CTaskSimpleFight::IsComboSet  (catalogue: specified/REA, port level 0)
- `__thiscall bool ()`, plain `ret`, result AL (setne; upper EAX = 0 from movzx). i = max((int8)this+0x24 - 4, 0) as in 0x4ABDA0;
  f = (u16)[0xC17154 + i*0x88] (CMeleeInfo +0x84 flags); m = 0x10 << (this+0x25 & 31) (`shl edx, cl` with CL = m_nCurrentMove byte;
  x86 masks the count to 5 bits); returns (f & m) != 0.

## 0x4ABDF0 CTaskSimpleFight::IsHitComboSet  (catalogue: specified/REA, port level 0)
- `__thiscall bool ()`, plain `ret`, returns EAX = 0/1. c = (int8)this+0x25 (m_nCurrentMove); if c > 2 (signed) return 0
  (0x4ABDF5). i = max((int8)this+0x24 - 4, 0); f = (u16)[0xC17154 + i*0x88]; return (f & (0x1000 << c)) != 0 ? 1 : 0.
  Negative c: `shl` uses CL & 31, so c = -1 shifts by 31 (gives 0 for the 16-bit flags) - keep.

## 0x622E40 CTaskSimpleFight::Clone  (catalogue: specified/REA, port level 3)
- Convention: virtual slot 1, `__thiscall CTask* (this = ECX)`, plain `ret`; SEH frame. p = CTask::operator new(0x28) 0x61A5A0 (cdecl); if p == NULL -> return NULL (0 in EAX). Else p->CTaskSimpleFight::CTaskSimpleFight(target = this+0x18, command = (int32)(int8)this+0x27 (m_nLastCommand), idlePeriod = (u32)(u16)this+0x10) 0x61C470; return p.
- gta-reversed: not hooked at this address (catalogue source REA); described from the bytes.

## 0x6239F0 CTaskSimpleFight::MakeAbortable  (catalogue: specified/REA, port level 9)
- Convention: `__thiscall bool (this, CPed* ped [esp+4], int32 priority [esp+8], const CEvent* event [esp+0xC])`, `ret 0xC`, AL.
- Behaviour:
  1. If priority is neither 1 nor 2: +0x26 (m_nNextCommand, u8) = 1; return 0.
  2. If event != NULL: p = event->vtable[+8]() (GetEventPriority); if p <= 0x1F return 0; if event->vtable[+8]() (called again) == 0x3C
     return 0.
  3. If +0x1C (m_pAnim) != NULL: if priority == 2: m_pAnim+0x1C (blend delta) = -1000.0f (imm 0xC47A0000); m_pAnim->SetDeleteCallback
     (0x4D1390, NULL) 0x4CEBC0; +0x1C = 0.
  4. If +0x20 (m_pIdleAnim) != NULL: SetDeleteCallback(0x4D1390, NULL) on it; a = +0x20; if a+0x18 (blend amount) > 0.0f [0x858B50]
     (`test ah,0x41; jne` skip when <= 0 or NaN) and a+0x1C (blend delta) >= 0.0f (`test ah,1; jne` skip when < 0 or NaN):
     CAnimManager::BlendAnimation(ped+0x18 (clump), ped+0x4D4 (anim group), 3, priority == 2 ? 1000.0f (0x447A0000) : 16.0f
     (0x41800000)) 0x4D4610 (cdecl). Then +0x20 = 0.
  5. If ped != NULL and ped->IsPlayer(): pd = ped+0x480 (player data); pd+0xC = 0; pd+0x10 = 0; this->0x61C9B0(ped) (unnamed thiscall
     helper that fetches the fight strafe anims 0x29..0x2C of the ped clump and adjusts them from pd+0xC/+0x10; proposed
     `CTaskSimpleFight::SetPlayerMoveAnim`, UNCLEAR).
  6. +8 (m_bIsFinished) = 1; return 1.
- gta-reversed: not hooked at this address.
